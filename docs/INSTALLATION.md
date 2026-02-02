# SUMM Daemon Installation Guide

This guide covers installing SUMM Daemon on various platforms.

## Prerequisites

### tmux (Required)

SUMM Daemon uses tmux to manage persistent CLI sessions.

```bash
# Debian/Ubuntu
sudo apt update
sudo apt install tmux

# RHEL/CentOS/Fedora
sudo dnf install tmux

# Arch Linux
sudo pacman -S tmux

# macOS
brew install tmux
```

Verify tmux is installed:

```bash
tmux -V
# Should show: tmux 3.0 or higher
```

### Rust (for building from source)

If building from source, install Rust via rustup:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

## Installation Methods

### Method 1: Install from Crates.io (Future)

```bash
cargo install summ-daemon
cargo install summ-cli
```

### Method 2: Build from Source

```bash
# Clone the repository
git clone https://github.com/your-org/SUMM-Daemon.git
cd SUMM-Daemon

# Build release binaries
cargo build --release

# Install to ~/.cargo/bin
install -m 755 target/release/summ-daemon ~/.cargo/bin/
install -m 755 target/release/summ ~/.cargo/bin/

# Verify installation
summ-daemon --version
summ --version
```

### Method 3: Systemd User Service (Linux)

For automatic startup on login:

```bash
# From the repository root
./scripts/install.sh
```

This script:
- Copies the systemd unit file to `~/.config/systemd/user/`
- Reloads the systemd daemon
- Enables the service to start on login

After installation:

```bash
# Start the daemon immediately
systemctl --user start summ-daemon

# Check status
systemctl --user status summ-daemon

# View logs
journalctl --user -u summ-daemon -f
```

## Uninstallation

### Remove Systemd Service

```bash
./scripts/uninstall.sh
```

### Remove Binaries

```bash
rm -f ~/.cargo/bin/summ-daemon
rm -f ~/.cargo/bin/summ
```

### Remove Data Directory

```bash
# This will remove all sessions and configuration
rm -rf ~/.summ-daemon
```

## Upgrading

```bash
# Stop the daemon first
summ daemon stop

# Build and install new version
cargo build --release
install -m 755 target/release/summ-daemon ~/.cargo/bin/
install -m 755 target/release/summ ~/.cargo/bin/

# Restart
summ daemon start
```

## Verification

After installation, verify everything works:

```bash
# Check daemon is running
summ daemon status

# List sessions (should be empty initially)
summ list

# View help
summ --help
```
