#!/bin/bash

# ArbitrageX Supreme v3.0 - Deployment Script
# Usage: ./deploy.sh [dev|staging|prod]

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Configuration
ENVIRONMENT=${1:-dev}
PROJECT_ROOT="/root/arbitragex/backend"
BACKUP_DIR="/root/arbitragex/backups"
LOG_FILE="/var/log/arbitragex/deploy-$(date +%Y%m%d-%H%M%S).log"

# Functions
log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1" | tee -a "$LOG_FILE"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1" | tee -a "$LOG_FILE"
    exit 1
}

warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1" | tee -a "$LOG_FILE"
}

# Check environment
check_environment() {
    log "Checking deployment environment..."
    
    # Check if running as root or with sudo
    if [[ $EUID -ne 0 ]]; then
        error "This script must be run as root or with sudo"
    fi
    
    # Check required tools
    for cmd in git docker docker-compose curl jq; do
        if ! command -v $cmd &> /dev/null; then
            error "$cmd is required but not installed"
        fi
    done
    
    # Check Docker daemon
    if ! docker info &> /dev/null; then
        error "Docker daemon is not running"
    fi
    
    log "Environment check passed ✓"
}

# Backup current deployment
backup_current() {
    log "Creating backup of current deployment..."
    
    BACKUP_NAME="backup-$(date +%Y%m%d-%H%M%S)"
    mkdir -p "$BACKUP_DIR/$BACKUP_NAME"
    
    # Backup database
    docker exec arbitragex-postgres pg_dump -U arbitragex arbitragex | \
        gzip > "$BACKUP_DIR/$BACKUP_NAME/database.sql.gz"
    
    # Backup Redis
    docker exec arbitragex-redis redis-cli --rdb "$BACKUP_DIR/$BACKUP_NAME/redis.rdb"
    
    # Backup configuration files
    cp -r "$PROJECT_ROOT"/.env* "$BACKUP_DIR/$BACKUP_NAME/" 2>/dev/null || true
    
    log "Backup completed: $BACKUP_DIR/$BACKUP_NAME"
}

# Pull latest code
update_code() {
    log "Pulling latest code from repository..."
    
    cd "$PROJECT_ROOT"
    
    # Stash any local changes
    git stash
    
    # Pull latest changes
    git pull origin main
    
    # Update submodules if any
    git submodule update --init --recursive
    
    log "Code updated to latest version"
}

# Build services
build_services() {
    log "Building Docker images..."
    
    cd "$PROJECT_ROOT"
    
    # Build with appropriate compose file
    case $ENVIRONMENT in
        prod)
            docker-compose -f docker/docker-compose.yml -f docker/docker-compose.prod.yml build --parallel
            ;;
        *)
            docker-compose -f docker/docker-compose.yml build --parallel
            ;;
    esac
    
    log "Docker images built successfully"
}

# Deploy services
deploy_services() {
    log "Deploying services..."
    
    cd "$PROJECT_ROOT"
    
    # Stop current services
    log "Stopping current services..."
    docker-compose -f docker/docker-compose.yml down
    
    # Start services based on environment
    case $ENVIRONMENT in
        prod)
            log "Starting production services..."
            docker-compose -f docker/docker-compose.yml -f docker/docker-compose.prod.yml up -d
            ;;
        *)
            log "Starting development services..."
            docker-compose -f docker/docker-compose.yml up -d
            ;;
    esac
    
    # Wait for services to be healthy
    log "Waiting for services to be healthy..."
    sleep 10
    
    # Check service health
    check_service_health
}

# Check service health
check_service_health() {
    log "Checking service health..."
    
    SERVICES=(
        "arbitragex-postgres:5432"
        "arbitragex-redis:6379"
        "arbitragex-selector:8080/health"
        "arbitragex-simctl:8081/health"
    )
    
    for service in "${SERVICES[@]}"; do
        IFS=':' read -r container endpoint <<< "$service"
        
        if [[ $endpoint == *"/health"* ]]; then
            # HTTP health check
            if curl -f -s "http://localhost:${endpoint}" > /dev/null; then
                log "✓ $container is healthy"
            else
                error "$container health check failed"
            fi
        else
            # Port check
            if docker exec $container echo "OK" &> /dev/null; then
                log "✓ $container is running"
            else
                error "$container is not running"
            fi
        fi
    done
}

# Run migrations
run_migrations() {
    log "Running database migrations..."
    
    # Wait for PostgreSQL to be ready
    until docker exec arbitragex-postgres pg_isready -U arbitragex; do
        log "Waiting for PostgreSQL..."
        sleep 2
    done
    
    # Run migrations
    docker exec arbitragex-postgres psql -U arbitragex -d arbitragex -f /docker-entrypoint-initdb.d/01-schema.sql
    
    log "Migrations completed"
}

# Update monitoring
update_monitoring() {
    log "Updating monitoring configuration..."
    
    # Reload Prometheus configuration
    curl -X POST http://localhost:9090/-/reload || warning "Failed to reload Prometheus"
    
    # Restart Grafana to pick up new dashboards
    docker restart arbitragex-grafana || warning "Failed to restart Grafana"
    
    log "Monitoring updated"
}

# Send deployment notification
send_notification() {
    local status=$1
    local message=$2
    
    # Webhook URL (configure as needed)
    WEBHOOK_URL=${DEPLOY_WEBHOOK_URL:-""}
    
    if [[ -n "$WEBHOOK_URL" ]]; then
        curl -X POST "$WEBHOOK_URL" \
            -H "Content-Type: application/json" \
            -d "{
                \"text\": \"ArbitrageX Deployment [$ENVIRONMENT]\",
                \"status\": \"$status\",
                \"message\": \"$message\",
                \"timestamp\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"
            }" || warning "Failed to send notification"
    fi
}

# Main deployment flow
main() {
    log "Starting ArbitrageX deployment for environment: $ENVIRONMENT"
    
    # Create log directory
    mkdir -p "$(dirname "$LOG_FILE")"
    
    # Pre-deployment checks
    check_environment
    
    # Deployment steps
    backup_current
    update_code
    build_services
    deploy_services
    run_migrations
    update_monitoring
    
    # Post-deployment
    log "Deployment completed successfully! 🎉"
    send_notification "success" "Deployment completed successfully"
    
    # Show service status
    docker-compose -f docker/docker-compose.yml ps
}

# Error handling
trap 'error "Deployment failed! Check logs at: $LOG_FILE"' ERR

# Run main function
main "$@"



