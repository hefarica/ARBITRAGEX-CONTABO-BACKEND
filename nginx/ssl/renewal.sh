#!/bin/bash

# SSL Certificate Renewal Script for ArbitrageX
# Automatically renews Let's Encrypt certificates

set -e

DOMAIN="${DOMAIN:-arbitragex.io}"
CERT_DIR="/etc/nginx/ssl"
LOG_FILE="/var/log/ssl-renewal.log"

echo "🔄 Starting SSL certificate renewal process..." | tee -a "$LOG_FILE"
date | tee -a "$LOG_FILE"

# Function to log with timestamp
log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a "$LOG_FILE"
}

# Check if certificates need renewal (30 days before expiry)
log "🔍 Checking certificate expiration..."
if ! certbot certificates | grep -q "VALID: 30 days"; then
    log "📅 Certificates are still valid for more than 30 days"
    exit 0
fi

log "⚠️  Certificates need renewal"

# Attempt renewal
log "🔄 Attempting certificate renewal..."
if certbot renew --quiet --no-self-upgrade; then
    log "✅ Certificate renewal successful"
    
    # Copy renewed certificates
    log "📋 Copying renewed certificates..."
    cp "/etc/letsencrypt/live/$DOMAIN/fullchain.pem" "$CERT_DIR/arbitragex.crt"
    cp "/etc/letsencrypt/live/$DOMAIN/privkey.pem" "$CERT_DIR/arbitragex.key"
    cp "/etc/letsencrypt/live/$DOMAIN/chain.pem" "$CERT_DIR/arbitragex-chain.crt"
    
    # Set proper permissions
    chmod 644 "$CERT_DIR/arbitragex.crt"
    chmod 600 "$CERT_DIR/arbitragex.key"
    chmod 644 "$CERT_DIR/arbitragex-chain.crt"
    
    # Test nginx configuration
    log "🧪 Testing nginx configuration..."
    if nginx -t; then
        log "✅ Nginx configuration is valid"
        
        # Reload nginx
        log "🔄 Reloading nginx..."
        if systemctl reload nginx; then
            log "✅ Nginx reloaded successfully"
        else
            log "❌ Failed to reload nginx"
            exit 1
        fi
    else
        log "❌ Nginx configuration test failed"
        exit 1
    fi
    
    # Send notification (optional)
    if command -v curl &> /dev/null && [ -n "$WEBHOOK_URL" ]; then
        log "📢 Sending renewal notification..."
        curl -X POST "$WEBHOOK_URL" \
            -H "Content-Type: application/json" \
            -d "{\"text\":\"✅ SSL certificates renewed successfully for $DOMAIN\"}" \
            || log "⚠️  Failed to send notification"
    fi
    
    log "🎉 Certificate renewal completed successfully!"
    
else
    log "❌ Certificate renewal failed"
    
    # Send failure notification
    if command -v curl &> /dev/null && [ -n "$WEBHOOK_URL" ]; then
        curl -X POST "$WEBHOOK_URL" \
            -H "Content-Type: application/json" \
            -d "{\"text\":\"❌ SSL certificate renewal failed for $DOMAIN\"}" \
            || log "⚠️  Failed to send failure notification"
    fi
    
    exit 1
fi

log "📊 Certificate status after renewal:"
certbot certificates | tee -a "$LOG_FILE"
