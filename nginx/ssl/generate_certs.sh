#!/bin/bash

# SSL Certificate Generation Script for ArbitrageX
# Uses Let's Encrypt for production certificates

set -e

DOMAIN="${DOMAIN:-arbitragex.io}"
EMAIL="${EMAIL:-admin@arbitragex.io}"
STAGING="${STAGING:-false}"
CERT_DIR="/etc/nginx/ssl"

echo "🔐 Generating SSL certificates for ArbitrageX..."

# Create certificate directory
mkdir -p "$CERT_DIR"

# Install certbot if not present
if ! command -v certbot &> /dev/null; then
    echo "📦 Installing certbot..."
    apt-get update
    apt-get install -y certbot python3-certbot-nginx
fi

# Determine staging flag
STAGING_FLAG=""
if [ "$STAGING" = "true" ]; then
    STAGING_FLAG="--staging"
    echo "⚠️  Using Let's Encrypt staging environment"
fi

# Generate certificates for main domain and subdomains
DOMAINS=(
    "$DOMAIN"
    "api.$DOMAIN"
    "ws.$DOMAIN"
    "admin.$DOMAIN"
    "monitor.$DOMAIN"
)

# Build domain arguments
DOMAIN_ARGS=""
for domain in "${DOMAINS[@]}"; do
    DOMAIN_ARGS="$DOMAIN_ARGS -d $domain"
done

echo "🌐 Requesting certificates for domains: ${DOMAINS[*]}"

# Request certificate
certbot certonly \
    --nginx \
    --email "$EMAIL" \
    --agree-tos \
    --no-eff-email \
    $STAGING_FLAG \
    $DOMAIN_ARGS

# Copy certificates to nginx directory
echo "📋 Copying certificates to nginx directory..."
cp "/etc/letsencrypt/live/$DOMAIN/fullchain.pem" "$CERT_DIR/arbitragex.crt"
cp "/etc/letsencrypt/live/$DOMAIN/privkey.pem" "$CERT_DIR/arbitragex.key"
cp "/etc/letsencrypt/live/$DOMAIN/chain.pem" "$CERT_DIR/arbitragex-chain.crt"

# Set proper permissions
chmod 644 "$CERT_DIR/arbitragex.crt"
chmod 600 "$CERT_DIR/arbitragex.key"
chmod 644 "$CERT_DIR/arbitragex-chain.crt"

# Generate DH parameters for enhanced security
if [ ! -f "$CERT_DIR/dhparam.pem" ]; then
    echo "🔒 Generating DH parameters (this may take a while)..."
    openssl dhparam -out "$CERT_DIR/dhparam.pem" 2048
    chmod 644 "$CERT_DIR/dhparam.pem"
fi

echo "✅ SSL certificates generated successfully!"
echo "📍 Certificates location: $CERT_DIR"
echo "🔄 Don't forget to reload nginx: nginx -s reload"

# Test nginx configuration
echo "🧪 Testing nginx configuration..."
nginx -t

echo "🎉 SSL setup completed successfully!"
