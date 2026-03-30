#!/bin/bash
# Expand existing SSL certificate to include all required subdomains

set -e

echo "=========================================="
echo "Expand SSL Certificate for All Subdomains"
echo "=========================================="
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then 
    echo "❌ This script must be run as root (use sudo)"
    exit 1
fi

# All subdomains that need to be covered
DOMAINS=(
    "synergy-network.io"
    "www.synergy-network.io"
    "api.synergy-network.io"
    "rpc.synergy-network.io"
    "ws.synergy-network.io"
    "explorer.synergy-network.io"
    "indexer.synergy-network.io"
    "validators.synergy-network.io"
    "status.synergy-network.io"
    "assets.synergy-network.io"
    "docs.synergy-network.io"
    "testbeta-core-rpc.synergy-network.io"
    "testbeta-core-ws.synergy-network.io"
    "testbeta-evm-rpc.synergy-network.io"
    "testbeta-evm-ws.synergy-network.io"
    "testbeta-api.synergy-network.io"
    "testbeta-explorer.synergy-network.io"
    "testbeta-indexer.synergy-network.io"
    "testbeta-wallet-api.synergy-network.io"
    "testbeta-faucet.synergy-network.io"
    "testbeta-sxcp-api.synergy-network.io"
    "testbeta-sxcp-ws.synergy-network.io"
    "testbeta-synq-verify.synergy-network.io"
    "testbeta-atlas-api.synergy-network.io"
    "testbeta.synergy-network.io"
)

echo "📋 Expanding certificate to include all subdomains..."
echo ""

# Build certbot command
CERTBOT_CMD="certbot certonly --webroot -w /var/www/letsencrypt --expand"
for domain in "${DOMAINS[@]}"; do
    CERTBOT_CMD="$CERTBOT_CMD -d $domain"
done
CERTBOT_CMD="$CERTBOT_CMD --email justin@synergy-network.io --agree-tos --non-interactive"

echo "Running: $CERTBOT_CMD"
echo ""

eval $CERTBOT_CMD || {
    echo ""
    echo "❌ Certificate expansion failed."
    echo "   Make sure all domains point to this server and nginx is running."
    echo "   You may need to get a wildcard certificate instead."
    exit 1
}

echo ""
echo "✅ Certificate expanded successfully!"
echo ""
echo "📝 Next steps:"
echo "   1. Test nginx: sudo nginx -t"
echo "   2. Reload nginx: sudo systemctl reload nginx"
echo "   3. Verify: Check browser console - ERR_CERT errors should be gone"
