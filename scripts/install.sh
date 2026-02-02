#!/bin/bash
# SUMM Daemon Installation Script
# This script installs systemd unit files and enables the daemon service

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

# Check if systemd is running
if ! systemctl --user list-units &>/dev/null; then
    echo_error "systemd user instance is not running. Cannot install systemd service."
    echo_warn "You can still run summ-daemon manually: summ daemon start"
    exit 1
fi

# Create systemd user directory
echo_info "Creating systemd user directory: $SYSTEMD_DIR"
mkdir -p "$SYSTEMD_DIR"

# Install systemd unit file
echo_info "Installing systemd unit file..."
cp "$PROJECT_ROOT/systemd/summ-daemon.service" "$SYSTEMD_DIR/"

# Update ExecStart path if binaries are in a different location
if [ ! -f "$HOME/.cargo/bin/summ-daemon" ]; then
    # Try to find summ-daemon in PATH or current build
    if [ -f "$PROJECT_ROOT/target/release/summ-daemon" ]; then
        EXEPATH="$PROJECT_ROOT/target/release/summ-daemon"
    elif [ -f "$PROJECT_ROOT/target/debug/summ-daemon" ]; then
        EXEPATH="$PROJECT_ROOT/target/debug/summ-daemon"
    else
        echo_error "Cannot find summ-daemon binary. Please build the project first:"
        echo "  cargo build --release"
        exit 1
    fi

    echo_info "Updating ExecStart path to: $EXEPATH"
    sed -i "s|ExecStart=%h/.cargo/bin/summ-daemon|ExecStart=$EXEPATH|" "$SYSTEMD_DIR/summ-daemon.service"
fi

# Reload systemd daemon
echo_info "Reloading systemd daemon..."
systemctl --user daemon-reload

# Enable the service
echo_info "Enabling summ-daemon service..."
systemctl --user enable summ-daemon.service

echo_info "Installation complete!"
echo ""
echo "To start the daemon now, run:"
echo "  systemctl --user start summ-daemon"
echo ""
echo "To check daemon status, run:"
echo "  systemctl --user status summ-daemon"
echo ""
echo "Or use the CLI:"
echo "  summ daemon status"
