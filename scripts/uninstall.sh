#!/bin/bash
# SUMM Daemon Uninstallation Script
# This script stops and disables the systemd service and removes unit files

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
SYSTEMD_DIR="$HOME/.config/systemd/user"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

echo_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

echo_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if service is installed
if [ ! -f "$SYSTEMD_DIR/summ-daemon.service" ]; then
    echo_warn "summ-daemon.service is not installed"
    exit 0
fi

# Stop the service if running
if systemctl --user is-active summ-daemon.service &>/dev/null; then
    echo_info "Stopping summ-daemon service..."
    systemctl --user stop summ-daemon.service
fi

# Disable the service
if systemctl --user is-enabled summ-daemon.service &>/dev/null; then
    echo_info "Disabling summ-daemon service..."
    systemctl --user disable summ-daemon.service
fi

# Remove the unit file
echo_info "Removing systemd unit file..."
rm -f "$SYSTEMD_DIR/summ-daemon.service"

# Reload systemd daemon
echo_info "Reloading systemd daemon..."
systemctl --user daemon-reload

# Reset failed state if any
systemctl --user reset-failed 2>/dev/null || true

echo_info "Uninstallation complete!"
echo ""
echo "Note: This only removes the systemd service. To completely remove SUMM Daemon:"
echo "  1. Remove binaries: rm -f ~/.cargo/bin/summ ~/.cargo/bin/summ-daemon"
echo "  2. Remove data directory: rm -rf ~/.summ-daemon"
