#!/bin/bash

# ArbitrageX Health Check Script
# Monitors all services and sends alerts if issues detected

set -euo pipefail

# Configuration
SLACK_WEBHOOK="${SLACK_WEBHOOK_URL:-}"
EMAIL_TO="${ALERT_EMAIL:-}"
CHECK_INTERVAL="${CHECK_INTERVAL:-60}"
ALERT_THRESHOLD="${ALERT_THRESHOLD:-3}"

# Service endpoints
declare -A SERVICES=(
    ["PostgreSQL"]="postgres-db:5432"
    ["Redis"]="redis-cache:6379"
    ["Selector API"]="http://localhost:8080/health"
    ["Sim CTL"]="http://localhost:8081/health"
    ["Geth Node"]="http://localhost:8545"
    ["Prometheus"]="http://localhost:9090/-/healthy"
    ["Grafana"]="http://localhost:3001/api/health"
    ["Temporal"]="http://localhost:7233/health"
)

# State tracking
declare -A FAILURE_COUNT
declare -A LAST_STATUS

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Functions
log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

# Check service health
check_service() {
    local name=$1
    local endpoint=$2
    local status="DOWN"
    local details=""
    
    if [[ $endpoint =~ ^https?:// ]]; then
        # HTTP health check
        if response=$(curl -s -f -w "\n%{http_code}" "$endpoint" 2>/dev/null); then
            http_code=$(echo "$response" | tail -n1)
            if [[ $http_code -eq 200 ]]; then
                status="UP"
                details="HTTP $http_code"
            fi
        fi
    elif [[ $endpoint =~ : ]]; then
        # TCP port check
        host=$(echo "$endpoint" | cut -d: -f1)
        port=$(echo "$endpoint" | cut -d: -f2)
        
        if docker exec "arbitragex-$host" echo "OK" &>/dev/null; then
            status="UP"
            details="Container running"
        elif timeout 2 bash -c "echo >/dev/tcp/$host/$port" 2>/dev/null; then
            status="UP"
            details="Port $port open"
        fi
    fi
    
    echo "$status|$details"
}

# Check system resources
check_system_resources() {
    info "Checking system resources..."
    
    # CPU usage
    cpu_usage=$(top -bn1 | grep "Cpu(s)" | awk '{print $2}' | cut -d'%' -f1)
    if (( $(echo "$cpu_usage > 80" | bc -l) )); then
        warning "High CPU usage: ${cpu_usage}%"
    fi
    
    # Memory usage
    mem_usage=$(free | grep Mem | awk '{print ($3/$2) * 100.0}')
    if (( $(echo "$mem_usage > 80" | bc -l) )); then
        warning "High memory usage: ${mem_usage}%"
    fi
    
    # Disk usage
    disk_usage=$(df -h / | awk 'NR==2 {print $5}' | sed 's/%//')
    if [[ $disk_usage -gt 80 ]]; then
        warning "High disk usage: ${disk_usage}%"
    fi
    
    # Docker disk usage
    docker_disk=$(docker system df --format "table {{.Type}}\t{{.Size}}" | grep -E "Images|Containers|Volumes" | awk '{sum += $2} END {print sum}')
    info "Docker disk usage: ${docker_disk:-0} GB"
}

# Check blockchain sync status
check_blockchain_sync() {
    info "Checking blockchain sync status..."
    
    if syncing=$(curl -s -X POST http://localhost:8545 \
        -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","method":"eth_syncing","params":[],"id":1}' \
        2>/dev/null | jq -r '.result'); then
        
        if [[ "$syncing" == "false" ]]; then
            log "Blockchain fully synced"
        else
            current_block=$(echo "$syncing" | jq -r '.currentBlock' | xargs printf "%d")
            highest_block=$(echo "$syncing" | jq -r '.highestBlock' | xargs printf "%d")
            progress=$((current_block * 100 / highest_block))
            warning "Blockchain syncing: ${progress}% (${current_block}/${highest_block})"
        fi
    fi
}

# Check MEV metrics
check_mev_metrics() {
    info "Checking MEV metrics..."
    
    if metrics=$(curl -s http://localhost:8080/metrics 2>/dev/null); then
        opportunities=$(echo "$metrics" | grep "arbitragex_opportunities_total" | awk '{print $2}')
        executions=$(echo "$metrics" | grep "arbitragex_executions_total" | awk '{print $2}')
        success_rate=$(echo "$metrics" | grep "arbitragex_success_rate" | awk '{print $2}')
        
        info "Opportunities detected: ${opportunities:-0}"
        info "Total executions: ${executions:-0}"
        info "Success rate: ${success_rate:-0}%"
    fi
}

# Send alert
send_alert() {
    local service=$1
    local status=$2
    local details=$3
    local message="ArbitrageX Alert: $service is $status. $details"
    
    # Slack notification
    if [[ -n "$SLACK_WEBHOOK" ]]; then
        curl -X POST "$SLACK_WEBHOOK" \
            -H 'Content-type: application/json' \
            --data "{
                \"text\": \"$message\",
                \"color\": \"$([ "$status" == "DOWN" ] && echo "danger" || echo "warning")\"
            }" 2>/dev/null
    fi
    
    # Email notification
    if [[ -n "$EMAIL_TO" ]]; then
        echo "$message" | mail -s "ArbitrageX Alert: $service" "$EMAIL_TO" 2>/dev/null || true
    fi
    
    # Log to systemd journal
    logger -t arbitragex-health "$message"
}

# Main health check loop
main() {
    log "Starting ArbitrageX health monitoring"
    
    while true; do
        echo -e "\n${BLUE}═══════════════════════════════════════${NC}"
        log "Running health checks..."
        
        all_healthy=true
        
        # Check each service
        for service in "${!SERVICES[@]}"; do
            endpoint="${SERVICES[$service]}"
            result=$(check_service "$service" "$endpoint")
            status=$(echo "$result" | cut -d'|' -f1)
            details=$(echo "$result" | cut -d'|' -f2)
            
            # Update failure count
            if [[ "$status" == "DOWN" ]]; then
                FAILURE_COUNT[$service]=$((${FAILURE_COUNT[$service]:-0} + 1))
                all_healthy=false
                
                if [[ ${FAILURE_COUNT[$service]} -ge $ALERT_THRESHOLD ]]; then
                    if [[ "${LAST_STATUS[$service]:-UP}" == "UP" ]]; then
                        error "$service is DOWN! $details"
                        send_alert "$service" "DOWN" "$details"
                    fi
                fi
            else
                FAILURE_COUNT[$service]=0
                if [[ "${LAST_STATUS[$service]:-UP}" == "DOWN" ]]; then
                    log "$service recovered! $details"
                    send_alert "$service" "RECOVERED" "$details"
                fi
            fi
            
            LAST_STATUS[$service]=$status
            
            # Display status
            if [[ "$status" == "UP" ]]; then
                echo -e "${GREEN}✓${NC} $service: ${GREEN}$status${NC} $details"
            else
                echo -e "${RED}✗${NC} $service: ${RED}$status${NC} $details"
            fi
        done
        
        # Additional checks
        echo -e "\n${BLUE}System Status:${NC}"
        check_system_resources
        check_blockchain_sync
        check_mev_metrics
        
        # Overall status
        if $all_healthy; then
            echo -e "\n${GREEN}✓ All systems operational${NC}"
        else
            echo -e "\n${RED}✗ Some services are experiencing issues${NC}"
        fi
        
        # Sleep before next check
        sleep "$CHECK_INTERVAL"
    done
}

# Handle script termination
trap 'log "Health monitoring stopped"; exit 0' SIGINT SIGTERM

# Run main function
main "$@"



