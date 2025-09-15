#!/bin/bash

# ArbitrageX Supreme V3.0 - Production Start Script
# Starts all services in the correct order with health checks

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
COMPOSE_FILE="docker-compose.yml"
PROD_COMPOSE_FILE="docker-compose.prod.yml"
LOG_DIR="./logs"
HEALTH_CHECK_TIMEOUT=60
HEALTH_CHECK_INTERVAL=5

# Functions
log() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1"
}

success() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')] ✅${NC} $1"
}

warning() {
    echo -e "${YELLOW}[$(date +'%Y-%m-%d %H:%M:%S')] ⚠️${NC} $1"
}

error() {
    echo -e "${RED}[$(date +'%Y-%m-%d %H:%M:%S')] ❌${NC} $1"
}

# Check if running in production mode
PRODUCTION_MODE=${PRODUCTION:-false}
if [ "$PRODUCTION_MODE" = "true" ]; then
    COMPOSE_FILE="$PROD_COMPOSE_FILE"
    log "Starting in PRODUCTION mode"
else
    log "Starting in DEVELOPMENT mode"
fi

# Create log directory
mkdir -p "$LOG_DIR"

# Check prerequisites
log "Checking prerequisites..."

# Check if Docker is running
if ! docker info >/dev/null 2>&1; then
    error "Docker is not running. Please start Docker and try again."
    exit 1
fi

# Check if Docker Compose is available
if ! command -v docker-compose >/dev/null 2>&1; then
    error "Docker Compose is not installed. Please install Docker Compose and try again."
    exit 1
fi

# Check if compose file exists
if [ ! -f "$COMPOSE_FILE" ]; then
    error "Compose file $COMPOSE_FILE not found."
    exit 1
fi

# Check if .env file exists
if [ ! -f ".env" ]; then
    warning ".env file not found. Copying from .env.example..."
    if [ -f ".env.example" ]; then
        cp .env.example .env
        warning "Please edit .env file with your configuration before running again."
        exit 1
    else
        error ".env.example file not found. Please create .env file manually."
        exit 1
    fi
fi

success "Prerequisites check passed"

# Stop any existing containers
log "Stopping existing containers..."
docker-compose -f "$COMPOSE_FILE" down --remove-orphans >/dev/null 2>&1 || true

# Pull latest images if in production
if [ "$PRODUCTION_MODE" = "true" ]; then
    log "Pulling latest images..."
    docker-compose -f "$COMPOSE_FILE" pull
fi

# Build images
log "Building images..."
docker-compose -f "$COMPOSE_FILE" build --no-cache

# Start infrastructure services first
log "Starting infrastructure services..."
docker-compose -f "$COMPOSE_FILE" up -d postgres redis

# Wait for database to be ready
log "Waiting for PostgreSQL to be ready..."
timeout=0
while [ $timeout -lt $HEALTH_CHECK_TIMEOUT ]; do
    if docker-compose -f "$COMPOSE_FILE" exec -T postgres pg_isready -U arbitragex >/dev/null 2>&1; then
        success "PostgreSQL is ready"
        break
    fi
    sleep $HEALTH_CHECK_INTERVAL
    timeout=$((timeout + HEALTH_CHECK_INTERVAL))
done

if [ $timeout -ge $HEALTH_CHECK_TIMEOUT ]; then
    error "PostgreSQL failed to start within $HEALTH_CHECK_TIMEOUT seconds"
    exit 1
fi

# Wait for Redis to be ready
log "Waiting for Redis to be ready..."
timeout=0
while [ $timeout -lt $HEALTH_CHECK_TIMEOUT ]; do
    if docker-compose -f "$COMPOSE_FILE" exec -T redis redis-cli ping >/dev/null 2>&1; then
        success "Redis is ready"
        break
    fi
    sleep $HEALTH_CHECK_INTERVAL
    timeout=$((timeout + HEALTH_CHECK_INTERVAL))
done

if [ $timeout -ge $HEALTH_CHECK_TIMEOUT ]; then
    error "Redis failed to start within $HEALTH_CHECK_TIMEOUT seconds"
    exit 1
fi

# Run database migrations
log "Running database migrations..."
docker-compose -f "$COMPOSE_FILE" run --rm api-server sqlx migrate run --database-url="$DATABASE_URL" || {
    warning "Database migrations failed or no migrations to run"
}

# Start core services
log "Starting core services..."
docker-compose -f "$COMPOSE_FILE" up -d searcher-rs selector-api

# Wait for core services to be healthy
log "Waiting for core services to be healthy..."
sleep 10

# Start additional services
log "Starting additional services..."
docker-compose -f "$COMPOSE_FILE" up -d recon relays-client api-server

# Start monitoring services if in production
if [ "$PRODUCTION_MODE" = "true" ]; then
    log "Starting monitoring services..."
    docker-compose -f "$COMPOSE_FILE" up -d prometheus grafana
fi

# Final health check
log "Performing final health check..."
sleep 15

# Check API server health
if curl -f http://localhost:8080/health >/dev/null 2>&1; then
    success "API Server is healthy"
else
    warning "API Server health check failed"
fi

# Check searcher service
if docker-compose -f "$COMPOSE_FILE" ps searcher-rs | grep -q "Up"; then
    success "Searcher service is running"
else
    warning "Searcher service is not running"
fi

# Check selector-api service
if docker-compose -f "$COMPOSE_FILE" ps selector-api | grep -q "Up"; then
    success "Selector API service is running"
else
    warning "Selector API service is not running"
fi

# Show running services
log "Current service status:"
docker-compose -f "$COMPOSE_FILE" ps

# Show logs location
log "Logs are available in: $LOG_DIR"
log "To view logs: docker-compose -f $COMPOSE_FILE logs -f [service_name]"

# Show useful commands
echo ""
success "🚀 ArbitrageX Supreme V3.0 started successfully!"
echo ""
log "Useful commands:"
echo "  • View all logs:     docker-compose -f $COMPOSE_FILE logs -f"
echo "  • View API logs:     docker-compose -f $COMPOSE_FILE logs -f api-server"
echo "  • View searcher logs: docker-compose -f $COMPOSE_FILE logs -f searcher-rs"
echo "  • Stop all services: docker-compose -f $COMPOSE_FILE down"
echo "  • Restart service:   docker-compose -f $COMPOSE_FILE restart [service_name]"
echo ""
log "API Endpoints:"
echo "  • Health Check:      http://localhost:8080/health"
echo "  • Opportunities:     http://localhost:8080/api/v1/opportunities"
echo "  • Executions:        http://localhost:8080/api/v1/executions"
echo "  • Metrics:           http://localhost:8080/api/v1/metrics"
echo "  • WebSocket:         ws://localhost:8080/ws"

if [ "$PRODUCTION_MODE" = "true" ]; then
    echo "  • Grafana:           http://localhost:3000 (admin/admin)"
    echo "  • Prometheus:        http://localhost:9090"
fi

echo ""
log "🎯 ArbitrageX Supreme V3.0 is ready for MEV arbitrage!"
