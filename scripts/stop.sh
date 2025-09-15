#!/bin/bash

# ArbitrageX Supreme V3.0 - Production Stop Script
# Gracefully stops all services with proper cleanup

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
GRACEFUL_TIMEOUT=30

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
    log "Stopping PRODUCTION mode services"
else
    log "Stopping DEVELOPMENT mode services"
fi

# Parse command line arguments
FORCE_STOP=false
REMOVE_VOLUMES=false
REMOVE_IMAGES=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --force|-f)
            FORCE_STOP=true
            shift
            ;;
        --volumes|-v)
            REMOVE_VOLUMES=true
            shift
            ;;
        --images|-i)
            REMOVE_IMAGES=true
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --force, -f     Force stop containers (kill instead of graceful stop)"
            echo "  --volumes, -v   Remove volumes after stopping"
            echo "  --images, -i    Remove images after stopping"
            echo "  --help, -h      Show this help message"
            echo ""
            echo "Environment Variables:"
            echo "  PRODUCTION=true    Stop production services"
            exit 0
            ;;
        *)
            error "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Check if Docker is running
if ! docker info >/dev/null 2>&1; then
    error "Docker is not running."
    exit 1
fi

# Check if compose file exists
if [ ! -f "$COMPOSE_FILE" ]; then
    error "Compose file $COMPOSE_FILE not found."
    exit 1
fi

# Show current status
log "Current service status:"
docker-compose -f "$COMPOSE_FILE" ps

# Stop services gracefully in reverse order
if [ "$FORCE_STOP" = "true" ]; then
    log "Force stopping all services..."
    docker-compose -f "$COMPOSE_FILE" kill
else
    log "Gracefully stopping services..."
    
    # Stop application services first
    log "Stopping application services..."
    docker-compose -f "$COMPOSE_FILE" stop api-server relays-client recon selector-api searcher-rs
    
    # Stop monitoring services if they exist
    if [ "$PRODUCTION_MODE" = "true" ]; then
        log "Stopping monitoring services..."
        docker-compose -f "$COMPOSE_FILE" stop grafana prometheus || true
    fi
    
    # Stop infrastructure services last
    log "Stopping infrastructure services..."
    docker-compose -f "$COMPOSE_FILE" stop redis postgres
fi

# Remove containers
log "Removing containers..."
docker-compose -f "$COMPOSE_FILE" down --remove-orphans

# Remove volumes if requested
if [ "$REMOVE_VOLUMES" = "true" ]; then
    warning "Removing volumes (this will delete all data)..."
    docker-compose -f "$COMPOSE_FILE" down -v
    success "Volumes removed"
fi

# Remove images if requested
if [ "$REMOVE_IMAGES" = "true" ]; then
    log "Removing images..."
    docker-compose -f "$COMPOSE_FILE" down --rmi all
    success "Images removed"
fi

# Clean up dangling containers and networks
log "Cleaning up dangling resources..."
docker container prune -f >/dev/null 2>&1 || true
docker network prune -f >/dev/null 2>&1 || true

# Show final status
log "Final status:"
if docker-compose -f "$COMPOSE_FILE" ps | grep -q "Up"; then
    warning "Some services are still running:"
    docker-compose -f "$COMPOSE_FILE" ps
else
    success "All services stopped successfully"
fi

# Show disk space freed up
if [ "$REMOVE_VOLUMES" = "true" ] || [ "$REMOVE_IMAGES" = "true" ]; then
    log "Running docker system prune to free up space..."
    docker system prune -f >/dev/null 2>&1 || true
fi

echo ""
success "🛑 ArbitrageX Supreme V3.0 stopped successfully!"
echo ""
log "To start again:"
echo "  • Development: ./scripts/start.sh"
echo "  • Production:  PRODUCTION=true ./scripts/start.sh"
echo ""
log "To clean up everything:"
echo "  • Remove volumes: ./scripts/stop.sh --volumes"
echo "  • Remove images:  ./scripts/stop.sh --images"
echo "  • Full cleanup:   ./scripts/stop.sh --volumes --images"
