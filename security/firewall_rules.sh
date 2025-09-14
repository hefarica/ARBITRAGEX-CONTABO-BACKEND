#!/bin/bash

# ArbitrageX Firewall Configuration Script
# Configure UFW for production security

set -euo pipefail

# Check if running as root
if [[ $EUID -ne 0 ]]; then
   echo "This script must be run as root"
   exit 1
fi

echo "Configuring firewall rules for ArbitrageX..."

# Reset firewall to defaults
ufw --force reset

# Default policies
ufw default deny incoming
ufw default allow outgoing
ufw default deny forward

# Allow SSH (adjust port as needed)
ufw allow 22/tcp comment 'SSH'

# Allow HTTP and HTTPS
ufw allow 80/tcp comment 'HTTP'
ufw allow 443/tcp comment 'HTTPS'

# Internal services (only from Docker network)
# PostgreSQL
ufw allow from 172.20.0.0/16 to any port 5432 comment 'PostgreSQL from Docker'

# Redis
ufw allow from 172.20.0.0/16 to any port 6379 comment 'Redis from Docker'

# Geth node (only from internal)
ufw allow from 172.20.0.0/16 to any port 8545 comment 'Geth RPC from Docker'
ufw allow from 172.20.0.0/16 to any port 8546 comment 'Geth WS from Docker'

# Monitoring (restricted access)
ufw allow from 10.0.0.0/8 to any port 9090 comment 'Prometheus'
ufw allow from 10.0.0.0/8 to any port 3001 comment 'Grafana'

# Rate limiting for API endpoints
# Using iptables for more advanced rules
iptables -A INPUT -p tcp --dport 80 -m state --state NEW -m recent --set
iptables -A INPUT -p tcp --dport 80 -m state --state NEW -m recent --update --seconds 60 --hitcount 100 -j DROP

iptables -A INPUT -p tcp --dport 443 -m state --state NEW -m recent --set
iptables -A INPUT -p tcp --dport 443 -m state --state NEW -m recent --update --seconds 60 --hitcount 100 -j DROP

# DDoS protection
iptables -A INPUT -p tcp --dport 80 -m connlimit --connlimit-above 50 -j REJECT
iptables -A INPUT -p tcp --dport 443 -m connlimit --connlimit-above 50 -j REJECT

# Save iptables rules
iptables-save > /etc/iptables/rules.v4

# Enable UFW
ufw --force enable

echo "Firewall configuration completed!"
ufw status verbose



