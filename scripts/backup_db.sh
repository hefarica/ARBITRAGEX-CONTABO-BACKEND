#!/bin/bash

# ArbitrageX Database Backup Script
# Supports local and remote backups with encryption

set -euo pipefail

# Configuration
BACKUP_DIR="${BACKUP_DIR:-/root/arbitragex/backups}"
RETENTION_DAYS="${RETENTION_DAYS:-30}"
DB_HOST="${DB_HOST:-postgres-db}"
DB_NAME="${DB_NAME:-arbitragex}"
DB_USER="${DB_USER:-arbitragex}"
REDIS_HOST="${REDIS_HOST:-redis-cache}"
S3_BUCKET="${S3_BUCKET:-}"
ENCRYPTION_KEY="${BACKUP_ENCRYPTION_KEY:-}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Functions
log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
    exit 1
}

warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Create backup directory
create_backup_dir() {
    TIMESTAMP=$(date +%Y%m%d-%H%M%S)
    BACKUP_PATH="$BACKUP_DIR/backup-$TIMESTAMP"
    mkdir -p "$BACKUP_PATH"
    log "Created backup directory: $BACKUP_PATH"
}

# Backup PostgreSQL
backup_postgres() {
    log "Starting PostgreSQL backup..."
    
    # Check if running in Docker
    if docker ps | grep -q arbitragex-postgres; then
        # Docker environment
        docker exec arbitragex-postgres pg_dump \
            -U "$DB_USER" \
            -d "$DB_NAME" \
            --verbose \
            --no-owner \
            --no-acl \
            --format=custom \
            --compress=9 \
            > "$BACKUP_PATH/postgres-$DB_NAME.dump"
    else
        # Direct connection
        PGPASSWORD="$DB_PASSWORD" pg_dump \
            -h "$DB_HOST" \
            -U "$DB_USER" \
            -d "$DB_NAME" \
            --verbose \
            --no-owner \
            --no-acl \
            --format=custom \
            --compress=9 \
            > "$BACKUP_PATH/postgres-$DB_NAME.dump"
    fi
    
    # Also create SQL text backup for easy inspection
    if docker ps | grep -q arbitragex-postgres; then
        docker exec arbitragex-postgres pg_dump \
            -U "$DB_USER" \
            -d "$DB_NAME" \
            --no-owner \
            --no-acl \
            | gzip > "$BACKUP_PATH/postgres-$DB_NAME.sql.gz"
    fi
    
    log "PostgreSQL backup completed"
}

# Backup Redis
backup_redis() {
    log "Starting Redis backup..."
    
    if docker ps | grep -q arbitragex-redis; then
        # Save Redis data
        docker exec arbitragex-redis redis-cli BGSAVE
        
        # Wait for save to complete
        while [ $(docker exec arbitragex-redis redis-cli LASTSAVE) -eq $(docker exec arbitragex-redis redis-cli LASTSAVE) ]; do
            sleep 1
        done
        
        # Copy RDB file
        docker cp arbitragex-redis:/data/dump.rdb "$BACKUP_PATH/redis-dump.rdb"
        
        # Also export as JSON for inspection
        docker exec arbitragex-redis redis-cli --rdb /data/dump.rdb --json > "$BACKUP_PATH/redis-data.json" 2>/dev/null || true
    else
        warning "Redis container not found, skipping Redis backup"
    fi
    
    log "Redis backup completed"
}

# Backup configuration files
backup_configs() {
    log "Backing up configuration files..."
    
    CONFIG_FILES=(
        ".env"
        ".env.production"
        "docker/docker-compose.yml"
        "docker/docker-compose.prod.yml"
        "nginx/nginx.conf"
        "nginx/sites-available/*.conf"
        "monitoring/prometheus.yml"
        "monitoring/alerts.rules"
    )
    
    mkdir -p "$BACKUP_PATH/configs"
    
    for file in "${CONFIG_FILES[@]}"; do
        if [[ -e "$file" ]]; then
            cp -r "$file" "$BACKUP_PATH/configs/" 2>/dev/null || true
        fi
    done
    
    log "Configuration backup completed"
}

# Create backup metadata
create_metadata() {
    log "Creating backup metadata..."
    
    cat > "$BACKUP_PATH/metadata.json" <<EOF
{
    "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
    "version": "$(git rev-parse HEAD 2>/dev/null || echo 'unknown')",
    "environment": "${ENVIRONMENT:-production}",
    "database": {
        "name": "$DB_NAME",
        "size": "$(du -sh "$BACKUP_PATH/postgres-$DB_NAME.dump" | cut -f1)"
    },
    "redis": {
        "size": "$(du -sh "$BACKUP_PATH/redis-dump.rdb" 2>/dev/null | cut -f1 || echo 'N/A')"
    },
    "retention_days": $RETENTION_DAYS
}
EOF
    
    log "Metadata created"
}

# Compress backup
compress_backup() {
    log "Compressing backup..."
    
    cd "$BACKUP_DIR"
    tar -czf "backup-$TIMESTAMP.tar.gz" "backup-$TIMESTAMP"
    
    # Remove uncompressed directory
    rm -rf "backup-$TIMESTAMP"
    
    COMPRESSED_FILE="$BACKUP_DIR/backup-$TIMESTAMP.tar.gz"
    log "Backup compressed: $COMPRESSED_FILE ($(du -sh "$COMPRESSED_FILE" | cut -f1))"
}

# Encrypt backup
encrypt_backup() {
    if [[ -n "$ENCRYPTION_KEY" ]]; then
        log "Encrypting backup..."
        
        openssl enc -aes-256-cbc \
            -salt \
            -in "$COMPRESSED_FILE" \
            -out "$COMPRESSED_FILE.enc" \
            -pass pass:"$ENCRYPTION_KEY"
        
        # Remove unencrypted file
        rm "$COMPRESSED_FILE"
        COMPRESSED_FILE="$COMPRESSED_FILE.enc"
        
        log "Backup encrypted"
    else
        warning "No encryption key provided, backup not encrypted"
    fi
}

# Upload to S3
upload_to_s3() {
    if [[ -n "$S3_BUCKET" ]]; then
        log "Uploading backup to S3..."
        
        aws s3 cp "$COMPRESSED_FILE" "s3://$S3_BUCKET/arbitragex-backups/" \
            --storage-class STANDARD_IA \
            --metadata "timestamp=$TIMESTAMP,environment=${ENVIRONMENT:-production}"
        
        log "Backup uploaded to S3"
    else
        warning "No S3 bucket configured, skipping cloud backup"
    fi
}

# Clean old backups
cleanup_old_backups() {
    log "Cleaning up old backups..."
    
    # Local cleanup
    find "$BACKUP_DIR" -name "backup-*.tar.gz*" -mtime +$RETENTION_DAYS -delete
    
    # S3 cleanup
    if [[ -n "$S3_BUCKET" ]]; then
        aws s3 ls "s3://$S3_BUCKET/arbitragex-backups/" | \
        while read -r line; do
            createDate=$(echo "$line" | awk '{print $1" "$2}')
            createDate=$(date -d "$createDate" +%s)
            olderThan=$(date -d "$RETENTION_DAYS days ago" +%s)
            if [[ $createDate -lt $olderThan ]]; then
                fileName=$(echo "$line" | awk '{print $4}')
                aws s3 rm "s3://$S3_BUCKET/arbitragex-backups/$fileName"
            fi
        done
    fi
    
    log "Cleanup completed"
}

# Verify backup
verify_backup() {
    log "Verifying backup integrity..."
    
    # Check file exists and has size
    if [[ ! -f "$COMPRESSED_FILE" ]] || [[ ! -s "$COMPRESSED_FILE" ]]; then
        error "Backup file is missing or empty"
    fi
    
    # Test archive integrity
    if [[ "$COMPRESSED_FILE" == *.enc ]]; then
        # Decrypt and test
        openssl enc -aes-256-cbc -d \
            -in "$COMPRESSED_FILE" \
            -pass pass:"$ENCRYPTION_KEY" | \
        tar -tzf - > /dev/null 2>&1 || error "Backup verification failed"
    else
        tar -tzf "$COMPRESSED_FILE" > /dev/null 2>&1 || error "Backup verification failed"
    fi
    
    log "Backup verified successfully"
}

# Main execution
main() {
    log "Starting ArbitrageX backup process"
    
    # Create backup directory with timestamp
    create_backup_dir
    
    # Perform backups
    backup_postgres
    backup_redis
    backup_configs
    create_metadata
    
    # Process backup
    compress_backup
    encrypt_backup
    
    # Verify and upload
    verify_backup
    upload_to_s3
    
    # Cleanup
    cleanup_old_backups
    
    log "Backup completed successfully! File: $COMPRESSED_FILE"
}

# Run main function
main "$@"



