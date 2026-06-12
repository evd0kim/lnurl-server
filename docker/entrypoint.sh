#!/bin/bash
set -e

# Set default values for environment variables
# Note: Don't prefix with ENV_ - supervisor adds that automatically
export PHOENIXD_NETWORK="${PHOENIXD_NETWORK:-testnet}"
export PHOENIXD_API_PASSWORD="${PHOENIXD_API_PASSWORD:-phoenix123}"
export RUST_LOG="${RUST_LOG:-info}"
export LNURL_DOMAIN="${LNURL_DOMAIN:-localhost}"
export PHOENIXD_URL="${PHOENIXD_URL:-http://127.0.0.1:9740}"

# Validate required environment variables
if [ -z "$LNURL_DOMAIN" ]; then
    echo "ERROR: LNURL_DOMAIN environment variable is required"
    echo "Example: export LNURL_DOMAIN=example.com"
    exit 1
fi

# Create data directory if it doesn't exist
mkdir -p /app/data
chmod 755 /app/data

echo "=========================================="
echo "LNURL Server with Phoenixd Docker Image"
echo "=========================================="
echo ""
echo "Configuration:"
echo "  Phoenixd Network: $PHOENIXD_NETWORK"
echo "  Phoenixd URL: $PHOENIXD_URL"
echo "  Phoenixd API Password: (set)"
echo "  LNURL Domain: $LNURL_DOMAIN"
echo "  Log Level: $RUST_LOG"
echo "  LNURL Server Port: 3000"
echo "  Phoenixd Port: 9740 (internal)"
echo "  Data Directory: /app/data"
echo ""

# Function to handle graceful shutdown
cleanup() {
    echo ""
    echo "Shutting down services..."
    kill -TERM "$SUPERVISORD_PID" 2>/dev/null || true
    wait "$SUPERVISORD_PID" 2>/dev/null || true
    echo "Services stopped gracefully"
    exit 0
}

# Set up signal handlers for graceful shutdown
trap cleanup SIGTERM SIGINT

# Start supervisord in foreground mode
echo "Starting services with supervisor..."
exec /usr/bin/supervisord -c /etc/supervisor/supervisord.conf
