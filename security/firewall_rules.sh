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
sudo ufw default deny incoming
sudo ufw default allow outgoing

# Allow SSH (adjust port as needed)
sudo ufw allow 22/tcp

# Allow HTTP and HTTPS
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp

# Allow Geth node ports
sudo ufw allow 8545
sudo ufw allow 8546

# Allow additional port
sudo ufw allow 7233

# Enable UFW
sudo ufw --force enable

echo "Firewall configuration completed!"
ufw status verbose
