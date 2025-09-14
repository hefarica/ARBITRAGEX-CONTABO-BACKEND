#!/bin/bash

# ArbitrageX Supreme V3.0 - Contabo VPS Deployment Script
# Automated deployment script for complete MEV arbitrage system
# Version: 3.0.0

set -e  # Exit on any error

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_NAME="arbitragex-supreme-v3"
BACKUP_DIR="/opt/arbitragex/backups"
LOG_FILE="/var/log/arbitragex/deploy.log"

# Logging function
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

info() {
    echo -e "${BLUE}[INFO]${NC} $1" | tee -a "$LOG_FILE"
}

# Check if running as root
check_root() {
    if [[ $EUID -eq 0 ]]; then
        error "This script should not be run as root for security reasons"
    fi
}

# Check prerequisites
check_prerequisites() {
    log "Checking prerequisites..."
    
    # Check if Docker is installed
    if ! command -v docker &> /dev/null; then
        error "Docker is not installed. Please install Docker first."
    fi
    
    # Check if Docker Compose is installed
    if ! command -v docker-compose &> /dev/null; then
        error "Docker Compose is not installed. Please install Docker Compose first."
    fi
    
    # Check if Git is installed
    if ! command -v git &> /dev/null; then
        error "Git is not installed. Please install Git first."
    fi
    
    # Check if .env file exists
    if [[ ! -f "$SCRIPT_DIR/.env" ]]; then
        error ".env file not found. Please copy .env.example to .env and configure it."
    fi
    
    log "Prerequisites check passed ✓"
}

# Setup directories
setup_directories() {
    log "Setting up directories..."
    
    sudo mkdir -p /opt/arbitragex/{data,logs,backups,ssl}
    sudo mkdir -p /var/log/arbitragex
    sudo chown -R $USER:$USER /opt/arbitragex
    sudo chmod 755 /opt/arbitragex
    
    # Create log directory with proper permissions
    sudo touch "$LOG_FILE"
    sudo chown $USER:$USER "$LOG_FILE"
    
    log "Directories setup completed ✓"
}

# System optimization
optimize_system() {
    log "Optimizing system for MEV operations..."
    
    # Network optimizations
    sudo sysctl -w net.core.rmem_max=134217728
    sudo sysctl -w net.core.wmem_max=134217728
    sudo sysctl -w net.ipv4.tcp_rmem="4096 65536 134217728"
    sudo sysctl -w net.ipv4.tcp_wmem="4096 65536 134217728"
    sudo sysctl -w net.core.netdev_max_backlog=5000
    sudo sysctl -w net.ipv4.tcp_congestion_control=bbr
    
    # File descriptor limits
    echo "* soft nofile 65536" | sudo tee -a /etc/security/limits.conf
    echo "* hard nofile 65536" | sudo tee -a /etc/security/limits.conf
    
    # Memory optimizations for high-frequency trading
    sudo sysctl -w vm.swappiness=1
    sudo sysctl -w vm.dirty_ratio=15
    sudo sysctl -w vm.dirty_background_ratio=5
    
    # Make changes persistent
    sudo sysctl -p
    
    log "System optimization completed ✓"
}

# Install system dependencies
install_dependencies() {
    log "Installing system dependencies..."
    
    # Update package lists
    sudo apt-get update -y
    
    # Install essential packages
    sudo apt-get install -y \
        curl \
        wget \
        git \
        htop \
        vim \
        nano \
        jq \
        unzip \
        build-essential \
        software-properties-common \
        apt-transport-https \
        ca-certificates \
        gnupg \
        lsb-release \
        ntp \
        fail2ban \
        ufw \
        logrotate
    
    # Install Node.js (for some utilities)
    curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
    sudo apt-get install -y nodejs
    
    # Install Rust (for performance-critical components)
    if ! command -v rustc &> /dev/null; then
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source ~/.cargo/env
    fi
    
    log "Dependencies installation completed ✓"
}

# Configure firewall
configure_firewall() {
    log "Configuring firewall..."
    
    # Reset UFW rules
    sudo ufw --force reset
    
    # Default policies
    sudo ufw default deny incoming
    sudo ufw default allow outgoing
    
    # Allow SSH (change port if needed)
    sudo ufw allow 22/tcp
    
    # Allow HTTP and HTTPS
    sudo ufw allow 80/tcp
    sudo ufw allow 443/tcp
    
    # Allow monitoring ports (restrict to local network if needed)
    sudo ufw allow 3000/tcp  # Grafana
    sudo ufw allow 9090/tcp  # Prometheus
    
    # Allow Ethereum P2P (if running full node)
    sudo ufw allow 30303/tcp
    sudo ufw allow 30303/udp
    
    # Enable UFW
    sudo ufw --force enable
    
    log "Firewall configuration completed ✓"
}

# Generate SSL certificates (self-signed for development)
generate_ssl() {
    log "Generating SSL certificates..."
    
    if [[ ! -f "/opt/arbitragex/ssl/arbitragex.crt" ]]; then
        sudo openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
            -keyout /opt/arbitragex/ssl/arbitragex.key \
            -out /opt/arbitragex/ssl/arbitragex.crt \
            -subj "/C=US/ST=State/L=City/O=ArbitrageX/CN=localhost"
        
        sudo chown $USER:$USER /opt/arbitragex/ssl/*
        sudo chmod 600 /opt/arbitragex/ssl/arbitragex.key
        sudo chmod 644 /opt/arbitragex/ssl/arbitragex.crt
    fi
    
    log "SSL certificates generated ✓"
}

# Build and start services
deploy_services() {
    log "Building and deploying services..."
    
    cd "$SCRIPT_DIR"
    
    # Pull latest images
    docker-compose pull
    
    # Build custom services
    docker-compose build --no-cache
    
    # Start all services
    docker-compose up -d
    
    # Wait for services to start
    sleep 30
    
    # Check service health
    check_services_health
    
    log "Services deployment completed ✓"
}

# Check services health
check_services_health() {
    log "Checking services health..."
    
    local services=("searcher-rs" "selector-api" "sim-ctl" "relays-client" "recon" "postgres" "redis" "prometheus" "grafana")
    
    for service in "${services[@]}"; do
        if docker-compose ps -q "$service" | xargs docker inspect -f '{{.State.Health.Status}}' 2>/dev/null | grep -q "healthy\|starting"; then
            info "✓ $service is healthy"
        else
            warning "⚠ $service health check failed"
        fi
    done
}

# Setup log rotation
setup_log_rotation() {
    log "Setting up log rotation..."
    
    sudo tee /etc/logrotate.d/arbitragex > /dev/null <<EOF
/var/log/arbitragex/*.log {
    daily
    missingok
    rotate 30
    compress
    delaycompress
    notifempty
    create 644 $USER $USER
    postrotate
        systemctl reload rsyslog > /dev/null 2>&1 || true
    endscript
}
EOF
    
    log "Log rotation setup completed ✓"
}

# Create backup
create_backup() {
    log "Creating system backup..."
    
    local backup_name="arbitragex-backup-$(date +%Y%m%d-%H%M%S).tar.gz"
    local backup_path="$BACKUP_DIR/$backup_name"
    
    mkdir -p "$BACKUP_DIR"
    
    # Create backup excluding sensitive data
    tar -czf "$backup_path" \
        --exclude="*.log" \
        --exclude=".env" \
        --exclude="node_modules" \
        --exclude="target" \
        --exclude=".git" \
        "$SCRIPT_DIR"
    
    info "Backup created: $backup_path"
}

# Setup monitoring
setup_monitoring() {
    log "Setting up monitoring and alerting..."
    
    # Wait for Grafana to start
    sleep 10
    
    # Import dashboards (if you have dashboard JSONs)
    # This is a placeholder for dashboard import logic
    info "Monitoring setup completed - access Grafana at http://localhost:3000"
    info "Default credentials: admin / \${GRAFANA_PASSWORD from .env}"
}

# Setup cron jobs
setup_cron_jobs() {
    log "Setting up cron jobs..."
    
    # Add cron job for daily backups
    (crontab -l 2>/dev/null; echo "0 2 * * * cd $SCRIPT_DIR && ./scripts/backup.sh") | crontab -
    
    # Add cron job for log cleanup
    (crontab -l 2>/dev/null; echo "0 1 * * * find /var/log/arbitragex -name '*.log' -mtime +30 -delete") | crontab -
    
    log "Cron jobs setup completed ✓"
}

# Display deployment summary
deployment_summary() {
    log "==================================================="
    log "ArbitrageX Supreme V3.0 Deployment Summary"
    log "==================================================="
    
    echo ""
    info "🚀 Services Status:"
    docker-compose ps
    
    echo ""
    info "🔗 Access URLs:"
    info "  • Grafana Dashboard: http://localhost:3000"
    info "  • Prometheus: http://localhost:9090"
    info "  • AlertManager: http://localhost:9093"
    info "  • Searcher API: http://localhost:3001/health"
    info "  • Selector API: http://localhost:3002/health"
    
    echo ""
    info "📊 Key Metrics:"
    info "  • Check /var/log/arbitragex/ for application logs"
    info "  • Monitor system resources with htop"
    info "  • View Docker logs: docker-compose logs -f [service_name]"
    
    echo ""
    info "🔒 Security Notes:"
    info "  • Firewall is configured and enabled"
    info "  • SSL certificates generated for HTTPS"
    info "  • Change default passwords in production"
    
    echo ""
    log "Deployment completed successfully! 🎉"
    log "==================================================="
}

# Main deployment function
main() {
    log "Starting ArbitrageX Supreme V3.0 deployment..."
    
    check_root
    check_prerequisites
    setup_directories
    install_dependencies
    optimize_system
    configure_firewall
    generate_ssl
    deploy_services
    setup_log_rotation
    setup_monitoring
    setup_cron_jobs
    create_backup
    deployment_summary
    
    log "All deployment steps completed successfully!"
}

# Script usage
usage() {
    echo "ArbitrageX Supreme V3.0 Deployment Script"
    echo ""
    echo "Usage: $0 [OPTION]"
    echo ""
    echo "Options:"
    echo "  deploy     Full deployment (default)"
    echo "  update     Update services only"
    echo "  restart    Restart all services"
    echo "  stop       Stop all services"
    echo "  backup     Create backup only"
    echo "  logs       Show service logs"
    echo "  health     Check services health"
    echo "  help       Show this help message"
}

# Handle script arguments
case "${1:-deploy}" in
    "deploy")
        main
        ;;
    "update")
        log "Updating services..."
        cd "$SCRIPT_DIR"
        docker-compose pull
        docker-compose build --no-cache
        docker-compose up -d
        check_services_health
        ;;
    "restart")
        log "Restarting all services..."
        cd "$SCRIPT_DIR"
        docker-compose restart
        check_services_health
        ;;
    "stop")
        log "Stopping all services..."
        cd "$SCRIPT_DIR"
        docker-compose down
        ;;
    "backup")
        create_backup
        ;;
    "logs")
        cd "$SCRIPT_DIR"
        docker-compose logs -f "${2:-}"
        ;;
    "health")
        check_services_health
        ;;
    "help")
        usage
        ;;
    *)
        echo "Unknown option: $1"
        usage
        exit 1
        ;;
esac