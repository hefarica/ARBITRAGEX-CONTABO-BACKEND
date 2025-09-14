#!/bin/bash

# ArbitrageX SSL Certificate Setup
# Configure SSL with Let's Encrypt

set -euo pipefail

# Configuration
DOMAIN="${DOMAIN:-arbitragex.io}"
EMAIL="${EMAIL:-admin@arbitragex.io}"
NGINX_DIR="/etc/nginx"
SSL_DIR="$NGINX_DIR/ssl"

# Check if running as root
if [[ $EUID -ne 0 ]]; then
   echo "This script must be run as root"
   exit 1
fi

echo "Setting up SSL certificates for ArbitrageX..."

# Install certbot if not present
if ! command -v certbot &> /dev/null; then
    apt-get update
    apt-get install -y certbot python3-certbot-nginx
fi

# Create SSL directory
mkdir -p "$SSL_DIR"

# Generate strong DH parameters
if [[ ! -f "$SSL_DIR/dhparam.pem" ]]; then
    echo "Generating DH parameters (this may take a while)..."
    openssl dhparam -out "$SSL_DIR/dhparam.pem" 4096
fi

# Domains to secure
DOMAINS=(
    "$DOMAIN"
    "www.$DOMAIN"
    "api.$DOMAIN"
    "temporal.$DOMAIN"
    "grafana.$DOMAIN"
    "automation.$DOMAIN"
    "ai.$DOMAIN"
)

# Request certificates
certbot certonly \
    --nginx \
    --non-interactive \
    --agree-tos \
    --email "$EMAIL" \
    --domains "$(IFS=,; echo "${DOMAINS[*]}")"

# Create SSL configuration snippet
cat > "$NGINX_DIR/snippets/ssl-params.conf" <<EOF
# SSL Configuration
ssl_protocols TLSv1.2 TLSv1.3;
ssl_prefer_server_ciphers off;
ssl_ciphers ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384:ECDHE-ECDSA-CHACHA20-POLY1305:ECDHE-RSA-CHACHA20-POLY1305:DHE-RSA-AES128-GCM-SHA256:DHE-RSA-AES256-GCM-SHA384;

# SSL session cache
ssl_session_timeout 1d;
ssl_session_cache shared:SSL:50m;
ssl_session_tickets off;

# OCSP stapling
ssl_stapling on;
ssl_stapling_verify on;
resolver 8.8.8.8 8.8.4.4 valid=300s;
resolver_timeout 5s;

# Security headers
add_header Strict-Transport-Security "max-age=63072000; includeSubDomains; preload" always;
add_header X-Frame-Options "SAMEORIGIN" always;
add_header X-Content-Type-Options "nosniff" always;
add_header X-XSS-Protection "1; mode=block" always;
add_header Referrer-Policy "strict-origin-when-cross-origin" always;

# DH parameters
ssl_dhparam $SSL_DIR/dhparam.pem;
EOF

# Create certificate renewal script
cat > "/etc/cron.daily/certbot-renew" <<'EOF'
#!/bin/bash
certbot renew --quiet --post-hook "systemctl reload nginx"
EOF

chmod +x /etc/cron.daily/certbot-renew

# Update Nginx configuration to use certificates
for domain in "${DOMAINS[@]}"; do
    subdomain="${domain%%.*}"
    if [[ "$subdomain" == "$DOMAIN" ]]; then
        subdomain="main"
    fi
    
    # Create symbolic links for certificates
    ln -sf "/etc/letsencrypt/live/$DOMAIN/fullchain.pem" "$SSL_DIR/$subdomain.crt"
    ln -sf "/etc/letsencrypt/live/$DOMAIN/privkey.pem" "$SSL_DIR/$subdomain.key"
    ln -sf "/etc/letsencrypt/live/$DOMAIN/chain.pem" "$SSL_DIR/$subdomain-chain.crt"
done

# Test Nginx configuration
nginx -t

echo "SSL setup completed successfully!"
echo "Certificates installed for: ${DOMAINS[*]}"
echo ""
echo "Next steps:"
echo "1. Update nginx configs to use the new certificates"
echo "2. Restart nginx: systemctl restart nginx"
echo "3. Test SSL: https://www.ssllabs.com/ssltest/analyze.html?d=$DOMAIN"



