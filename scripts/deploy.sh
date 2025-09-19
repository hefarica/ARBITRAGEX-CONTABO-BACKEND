#!/bin/bash
# ArbitrageX Supreme V3.0 - Deployment Automation Scripts
# Paquete Operativo Completo - Automatización de Despliegue

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
BACKEND_REPO="https://github.com/hefarica/ARBITRAGEX-CONTABO-BACKEND.git"
EDGE_REPO="https://github.com/hefarica/ARBITRAGEXSUPREME.git"
FRONTEND_REPO="https://github.com/hefarica/show-my-github-gems.git"

DEPLOY_DIR="/opt/arbitragex"
BACKUP_DIR="/opt/arbitragex-backups"
LOG_FILE="/var/log/arbitragex-deploy.log"

# =====================================================
# UTILITY FUNCTIONS
# =====================================================

log() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1" | tee -a "$LOG_FILE"
}

success() {
    echo -e "${GREEN}✅ $1${NC}" | tee -a "$LOG_FILE"
}

warning() {
    echo -e "${YELLOW}⚠️  $1${NC}" | tee -a "$LOG_FILE"
}

error() {
    echo -e "${RED}❌ $1${NC}" | tee -a "$LOG_FILE"
    exit 1
}

check_prerequisites() {
    log "Checking prerequisites..."
    
    # Check if running as root or with sudo
    if [[ $EUID -ne 0 ]]; then
        error "This script must be run as root or with sudo"
    fi
    
    # Check required commands
    local required_commands=("docker" "docker-compose" "git" "curl" "jq" "wrangler" "npm")
    for cmd in "${required_commands[@]}"; do
        if ! command -v "$cmd" &> /dev/null; then
            error "Required command '$cmd' is not installed"
        fi
    done
    
    # Check Docker daemon
    if ! docker info &> /dev/null; then
        error "Docker daemon is not running"
    fi
    
    success "Prerequisites check passed"
}

create_directories() {
    log "Creating deployment directories..."
    
    mkdir -p "$DEPLOY_DIR"/{backend,edge,frontend}
    mkdir -p "$BACKUP_DIR"
    mkdir -p /var/log/arbitragex
    
    success "Directories created"
}

# =====================================================
# BACKUP FUNCTIONS
# =====================================================

backup_current_deployment() {
    log "Creating backup of current deployment..."
    
    local backup_timestamp=$(date +%Y%m%d_%H%M%S)
    local backup_path="$BACKUP_DIR/backup_$backup_timestamp"
    
    if [ -d "$DEPLOY_DIR" ]; then
        cp -r "$DEPLOY_DIR" "$backup_path"
        success "Backup created at $backup_path"
    else
        warning "No existing deployment to backup"
    fi
}

restore_from_backup() {
    local backup_path="$1"
    
    if [ -z "$backup_path" ]; then
        error "Backup path not specified"
    fi
    
    if [ ! -d "$backup_path" ]; then
        error "Backup path does not exist: $backup_path"
    fi
    
    log "Restoring from backup: $backup_path"
    
    # Stop current services
    stop_all_services
    
    # Remove current deployment
    rm -rf "$DEPLOY_DIR"
    
    # Restore from backup
    cp -r "$backup_path" "$DEPLOY_DIR"
    
    # Start services
    start_all_services
    
    success "Restored from backup successfully"
}

# =====================================================
# BACKEND DEPLOYMENT
# =====================================================

deploy_backend() {
    log "Deploying ArbitrageX Backend..."
    
    cd "$DEPLOY_DIR/backend"
    
    # Clone or update repository
    if [ -d ".git" ]; then
        log "Updating existing backend repository..."
        git fetch origin
        git reset --hard origin/main
    else
        log "Cloning backend repository..."
        git clone "$BACKEND_REPO" .
    fi
    
    # Load environment variables
    if [ -f ".env.production" ]; then
        log "Loading production environment variables..."
        export $(cat .env.production | grep -v '^#' | xargs)
    else
        warning "No .env.production file found, using defaults"
    fi
    
    # Build and deploy services
    log "Building backend services..."
    docker-compose -f docker-compose.prod.yml build --no-cache
    
    log "Starting backend services..."
    docker-compose -f docker-compose.prod.yml up -d
    
    # Wait for services to be ready
    log "Waiting for backend services to be ready..."
    sleep 30
    
    # Run health checks
    local services=("api-server:8080" "searcher-rs:8081" "selector-api:8082" "recon:8083" "relays-client:8084" "sim-ctl:8085")
    for service in "${services[@]}"; do
        local service_name=$(echo "$service" | cut -d':' -f1)
        local port=$(echo "$service" | cut -d':' -f2)
        
        if curl -f -s "http://localhost:$port/health" > /dev/null; then
            success "$service_name is healthy"
        else
            error "$service_name health check failed"
        fi
    done
    
    success "Backend deployment completed"
}

# =====================================================
# EDGE DEPLOYMENT
# =====================================================

deploy_edge() {
    log "Deploying ArbitrageX Edge Workers..."
    
    cd "$DEPLOY_DIR/edge"
    
    # Clone or update repository
    if [ -d ".git" ]; then
        log "Updating existing edge repository..."
        git fetch origin
        git reset --hard origin/main
    else
        log "Cloning edge repository..."
        git clone "$EDGE_REPO" .
    fi
    
    # Install dependencies
    log "Installing edge dependencies..."
    npm ci
    
    # Set up Cloudflare secrets
    log "Setting up Cloudflare secrets..."
    if [ -n "$JWT_SECRET" ]; then
        echo "$JWT_SECRET" | wrangler secret put JWT_SECRET --env production
    fi
    if [ -n "$API_KEY" ]; then
        echo "$API_KEY" | wrangler secret put API_KEY --env production
    fi
    if [ -n "$BACKEND_API_KEY" ]; then
        echo "$BACKEND_API_KEY" | wrangler secret put BACKEND_API_KEY --env production
    fi
    
    # Run D1 migrations
    log "Running D1 database migrations..."
    wrangler d1 migrations apply arbitragex-db-prod --env production
    
    # Deploy workers
    log "Deploying Cloudflare Workers..."
    wrangler deploy --env production
    
    # Set up KV namespaces
    log "Setting up KV namespaces..."
    wrangler kv:namespace create "CONFIG_STORE" --env production
    wrangler kv:namespace create "CACHE_STORE" --env production
    
    # Configure custom domains
    log "Configuring custom domains..."
    wrangler route add "arbitragex.app/*" arbitragex-worker --env production
    wrangler route add "api.arbitragex.app/*" arbitragex-api-proxy --env production
    
    # Health check
    sleep 10
    if curl -f -s "https://arbitragex.workers.dev/health" > /dev/null; then
        success "Edge workers are healthy"
    else
        error "Edge workers health check failed"
    fi
    
    success "Edge deployment completed"
}

# =====================================================
# FRONTEND DEPLOYMENT
# =====================================================

deploy_frontend() {
    log "Deploying ArbitrageX Frontend..."
    
    cd "$DEPLOY_DIR/frontend"
    
    # Clone or update repository
    if [ -d ".git" ]; then
        log "Updating existing frontend repository..."
        git fetch origin
        git reset --hard origin/main
    else
        log "Cloning frontend repository..."
        git clone "$FRONTEND_REPO" .
    fi
    
    # Install dependencies
    log "Installing frontend dependencies..."
    npm ci
    
    # Build for production
    log "Building frontend for production..."
    VITE_API_BASE_URL="https://api.arbitragex.dev" \
    VITE_EDGE_URL="https://arbitragex.workers.dev" \
    VITE_WS_URL="wss://arbitragex.workers.dev/ws" \
    npm run build
    
    # Deploy to Cloudflare Pages
    log "Deploying to Cloudflare Pages..."
    npx wrangler pages deploy dist --project-name=arbitragex-app --env=production
    
    # Configure custom domain
    log "Configuring custom domain..."
    npx wrangler pages domain add arbitragex.app --project-name=arbitragex-app
    npx wrangler pages domain add www.arbitragex.app --project-name=arbitragex-app
    
    # Health check
    sleep 15
    if curl -f -s "https://arbitragex.app" > /dev/null; then
        success "Frontend is accessible"
    else
        error "Frontend health check failed"
    fi
    
    success "Frontend deployment completed"
}

# =====================================================
# DATABASE OPERATIONS
# =====================================================

setup_databases() {
    log "Setting up databases..."
    
    # PostgreSQL setup
    log "Setting up PostgreSQL..."
    docker-compose -f "$DEPLOY_DIR/backend/docker-compose.prod.yml" exec -T postgres psql -U arbitragex_user -d arbitragex_prod -f /docker-entrypoint-initdb.d/schema.sql
    
    # Run migrations
    log "Running PostgreSQL migrations..."
    cd "$DEPLOY_DIR/backend"
    docker-compose -f docker-compose.prod.yml exec -T api-server ./scripts/migrate.sh
    
    success "Databases setup completed"
}

# =====================================================
# MONITORING SETUP
# =====================================================

setup_monitoring() {
    log "Setting up monitoring stack..."
    
    cd "$DEPLOY_DIR"
    
    # Create monitoring directory
    mkdir -p monitoring/{dashboards,datasources,alerts}
    
    # Copy monitoring configuration
    cp backend/monitoring/* monitoring/
    
    # Start monitoring stack
    docker-compose -f monitoring/docker-compose.monitoring.yml up -d
    
    # Wait for Grafana to be ready
    log "Waiting for Grafana to be ready..."
    sleep 30
    
    # Import dashboards
    log "Importing Grafana dashboards..."
    curl -X POST \
        -H "Content-Type: application/json" \
        -d @monitoring/dashboards/arbitragex-backend.json \
        "http://admin:${GRAFANA_ADMIN_PASSWORD}@localhost:3000/api/dashboards/db"
    
    success "Monitoring setup completed"
}

# =====================================================
# SERVICE MANAGEMENT
# =====================================================

start_all_services() {
    log "Starting all ArbitrageX services..."
    
    # Start backend services
    cd "$DEPLOY_DIR/backend"
    docker-compose -f docker-compose.prod.yml up -d
    
    # Start monitoring
    cd "$DEPLOY_DIR"
    docker-compose -f monitoring/docker-compose.monitoring.yml up -d
    
    success "All services started"
}

stop_all_services() {
    log "Stopping all ArbitrageX services..."
    
    # Stop backend services
    if [ -f "$DEPLOY_DIR/backend/docker-compose.prod.yml" ]; then
        cd "$DEPLOY_DIR/backend"
        docker-compose -f docker-compose.prod.yml down
    fi
    
    # Stop monitoring
    if [ -f "$DEPLOY_DIR/monitoring/docker-compose.monitoring.yml" ]; then
        cd "$DEPLOY_DIR"
        docker-compose -f monitoring/docker-compose.monitoring.yml down
    fi
    
    success "All services stopped"
}

restart_all_services() {
    log "Restarting all ArbitrageX services..."
    stop_all_services
    sleep 5
    start_all_services
    success "All services restarted"
}

# =====================================================
# HEALTH CHECKS
# =====================================================

run_health_checks() {
    log "Running comprehensive health checks..."
    
    local failed_checks=0
    
    # Backend health checks
    local backend_services=("8080" "8081" "8082" "8083" "8084" "8085")
    for port in "${backend_services[@]}"; do
        if curl -f -s "http://localhost:$port/health" > /dev/null; then
            success "Backend service on port $port is healthy"
        else
            error "Backend service on port $port is unhealthy"
            ((failed_checks++))
        fi
    done
    
    # Edge health check
    if curl -f -s "https://arbitragex.workers.dev/health" > /dev/null; then
        success "Edge workers are healthy"
    else
        warning "Edge workers health check failed"
        ((failed_checks++))
    fi
    
    # Frontend health check
    if curl -f -s "https://arbitragex.app" > /dev/null; then
        success "Frontend is accessible"
    else
        warning "Frontend health check failed"
        ((failed_checks++))
    fi
    
    # Database health check
    if docker-compose -f "$DEPLOY_DIR/backend/docker-compose.prod.yml" exec -T postgres pg_isready -U arbitragex_user > /dev/null; then
        success "PostgreSQL is healthy"
    else
        error "PostgreSQL health check failed"
        ((failed_checks++))
    fi
    
    # Redis health check
    if docker-compose -f "$DEPLOY_DIR/backend/docker-compose.prod.yml" exec -T redis redis-cli ping | grep -q PONG; then
        success "Redis is healthy"
    else
        error "Redis health check failed"
        ((failed_checks++))
    fi
    
    if [ $failed_checks -eq 0 ]; then
        success "All health checks passed"
        return 0
    else
        error "$failed_checks health checks failed"
        return 1
    fi
}

# =====================================================
# E2E TESTING
# =====================================================

run_e2e_tests() {
    log "Running E2E tests..."
    
    cd "$DEPLOY_DIR/frontend"
    
    # Install test dependencies
    npm install -g @playwright/test
    npx playwright install
    
    # Run E2E tests
    PLAYWRIGHT_BASE_URL="https://arbitragex.app" \
    API_BASE_URL="https://api.arbitragex.dev" \
    npx playwright test --config=e2e/playwright.config.ts
    
    if [ $? -eq 0 ]; then
        success "E2E tests passed"
    else
        error "E2E tests failed"
    fi
}

# =====================================================
# PERFORMANCE TESTING
# =====================================================

run_performance_tests() {
    log "Running performance tests..."
    
    # Install k6
    if ! command -v k6 &> /dev/null; then
        log "Installing k6..."
        sudo gpg -k
        sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
        echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" | sudo tee /etc/apt/sources.list.d/k6.list
        sudo apt-get update
        sudo apt-get install k6
    fi
    
    # Run performance tests
    cd "$DEPLOY_DIR/backend"
    k6 run --out json=performance-results.json \
        --env BASE_URL=https://api.arbitragex.dev \
        --env EDGE_URL=https://arbitragex.workers.dev \
        --env TEST_TYPE=load \
        performance/load-test.js
    
    # Validate SLOs
    node scripts/validate-slos.js performance-results.json
    
    if [ $? -eq 0 ]; then
        success "Performance tests passed"
    else
        error "Performance tests failed"
    fi
}

# =====================================================
# ROLLBACK FUNCTIONALITY
# =====================================================

rollback_deployment() {
    log "Rolling back deployment..."
    
    # Find latest backup
    local latest_backup=$(ls -t "$BACKUP_DIR" | head -n1)
    
    if [ -z "$latest_backup" ]; then
        error "No backup found for rollback"
    fi
    
    log "Rolling back to: $latest_backup"
    restore_from_backup "$BACKUP_DIR/$latest_backup"
    
    success "Rollback completed"
}

# =====================================================
# MAIN DEPLOYMENT FUNCTION
# =====================================================

full_deployment() {
    log "Starting full ArbitrageX deployment..."
    
    # Prerequisites
    check_prerequisites
    create_directories
    
    # Backup current deployment
    backup_current_deployment
    
    # Deploy all components
    deploy_backend
    deploy_edge
    deploy_frontend
    
    # Setup databases and monitoring
    setup_databases
    setup_monitoring
    
    # Health checks
    if ! run_health_checks; then
        warning "Health checks failed, consider rollback"
        return 1
    fi
    
    # Run tests
    run_e2e_tests
    run_performance_tests
    
    success "Full deployment completed successfully!"
    
    # Display access information
    echo ""
    echo "🎉 ArbitrageX Supreme V3.0 Deployment Complete!"
    echo "=============================================="
    echo "Frontend: https://arbitragex.app"
    echo "Backend API: https://api.arbitragex.dev"
    echo "Edge Workers: https://arbitragex.workers.dev"
    echo "Grafana: http://localhost:3000"
    echo "Prometheus: http://localhost:9090"
    echo "=============================================="
}

# =====================================================
# COMMAND LINE INTERFACE
# =====================================================

show_help() {
    echo "ArbitrageX Supreme V3.0 - Deployment Script"
    echo "Usage: $0 [COMMAND]"
    echo ""
    echo "Commands:"
    echo "  deploy-full      Full deployment (backend + edge + frontend)"
    echo "  deploy-backend   Deploy backend only"
    echo "  deploy-edge      Deploy edge workers only"
    echo "  deploy-frontend  Deploy frontend only"
    echo "  start            Start all services"
    echo "  stop             Stop all services"
    echo "  restart          Restart all services"
    echo "  health           Run health checks"
    echo "  test-e2e         Run E2E tests"
    echo "  test-perf        Run performance tests"
    echo "  rollback         Rollback to previous deployment"
    echo "  backup           Create backup of current deployment"
    echo "  help             Show this help message"
    echo ""
    echo "Environment Variables:"
    echo "  GRAFANA_ADMIN_PASSWORD  Grafana admin password"
    echo "  JWT_SECRET              JWT secret for authentication"
    echo "  API_KEY                 API key for services"
    echo "  CLOUDFLARE_API_TOKEN    Cloudflare API token"
    echo ""
}

# Main script logic
case "${1:-help}" in
    deploy-full)
        full_deployment
        ;;
    deploy-backend)
        check_prerequisites
        backup_current_deployment
        deploy_backend
        ;;
    deploy-edge)
        check_prerequisites
        deploy_edge
        ;;
    deploy-frontend)
        check_prerequisites
        deploy_frontend
        ;;
    start)
        start_all_services
        ;;
    stop)
        stop_all_services
        ;;
    restart)
        restart_all_services
        ;;
    health)
        run_health_checks
        ;;
    test-e2e)
        run_e2e_tests
        ;;
    test-perf)
        run_performance_tests
        ;;
    rollback)
        rollback_deployment
        ;;
    backup)
        backup_current_deployment
        ;;
    help|*)
        show_help
        ;;
esac
