#!/bin/sh
# articwake first-boot configuration script for Alpine Linux
# This script runs during first boot via macmpi headless bootstrap
# https://github.com/macmpi/alpine-linux-headless-bootstrap

set -e

BOOT_DIR="/media/mmcblk0p1"
ARTICWAKE_CONF="$BOOT_DIR/articwake"
ARTICWAKE_BIN="/usr/local/bin/articwake"
ARTICWAKE_DATA="/var/lib/articwake"
ARTICWAKE_SECRETS="/etc/articwake"
LOG_FILE="/var/log/articwake-setup.log"

log() {
    echo "[articwake-setup] $(date '+%Y-%m-%d %H:%M:%S') $1" | tee -a "$LOG_FILE"
}

die() {
    log "ERROR: $1"
    exit 1
}

# Verify required configuration files exist
check_required_files() {
    log "Checking required configuration files..."
    local missing=""

    for file in config.env wifi.conf ssh_key pin; do
        if [ ! -f "$ARTICWAKE_CONF/$file" ]; then
            missing="$missing $file"
        fi
    done

    if [ -n "$missing" ]; then
        die "Missing required files in /boot/articwake/:$missing
Please add these files and reboot. See README.txt for format details."
    fi

    log "All required files present"
}

# Install required packages
install_packages() {
    log "Updating package index..."
    apk update

    log "Installing base packages..."
    apk add iputils wpa_supplicant wpa_supplicant-openrc dhcpcd dhcpcd-openrc

    # Install Tailscale if auth key provided
    if [ -f "$ARTICWAKE_CONF/tailscale_authkey" ]; then
        log "Tailscale auth key found, installing Tailscale..."
        apk add tailscale tailscale-openrc
    fi

    log "Packages installed"
}

# Configure WiFi
setup_wifi() {
    log "Configuring WiFi..."

    mkdir -p /etc/wpa_supplicant
    cp "$ARTICWAKE_CONF/wifi.conf" /etc/wpa_supplicant/wpa_supplicant.conf
    chmod 600 /etc/wpa_supplicant/wpa_supplicant.conf

    # Configure network interface
    cat > /etc/network/interfaces <<EOF
auto lo
iface lo inet loopback

auto wlan0
iface wlan0 inet dhcp
EOF

    # Enable services at boot
    rc-update add wpa_supplicant default
    rc-update add dhcpcd default

    # Start WiFi now
    log "Starting WiFi services..."
    rc-service wpa_supplicant start || log "Warning: wpa_supplicant start failed"
    rc-service dhcpcd start || log "Warning: dhcpcd start failed"

    # Wait for network connectivity
    log "Waiting for network connectivity..."
    local retries=30
    while [ $retries -gt 0 ]; do
        if ping -c 1 -W 2 1.1.1.1 >/dev/null 2>&1; then
            log "Network connected!"
            return 0
        fi
        retries=$((retries - 1))
        sleep 2
    done

    log "Warning: Could not confirm network connectivity (continuing anyway)"
}

# Setup articwake binary and configuration
setup_articwake() {
    log "Setting up articwake..."

    # Create directories
    mkdir -p "$ARTICWAKE_DATA"
    mkdir -p "$ARTICWAKE_SECRETS"
    chmod 700 "$ARTICWAKE_SECRETS"

    # Install binary from boot partition
    if [ -f "$BOOT_DIR/articwake-bin/articwake" ]; then
        cp "$BOOT_DIR/articwake-bin/articwake" "$ARTICWAKE_BIN"
        chmod 755 "$ARTICWAKE_BIN"
        log "Binary installed to $ARTICWAKE_BIN"
    else
        die "Binary not found at $BOOT_DIR/articwake-bin/articwake"
    fi

    # Copy SSH key
    cp "$ARTICWAKE_CONF/ssh_key" "$ARTICWAKE_SECRETS/ssh_key"
    chmod 600 "$ARTICWAKE_SECRETS/ssh_key"
    log "SSH key installed"

    # Hash PIN with Argon2
    log "Hashing PIN..."
    PIN=$(cat "$ARTICWAKE_CONF/pin")
    if [ -z "$PIN" ]; then
        die "PIN file is empty"
    fi

    echo -n "$PIN" | "$ARTICWAKE_BIN" hash-pin > "$ARTICWAKE_DATA/pin.hash"
    chmod 600 "$ARTICWAKE_DATA/pin.hash"
    log "PIN hashed and stored"

    # Create environment file for OpenRC service
    cat > /etc/conf.d/articwake <<EOF
# articwake configuration - generated on first boot
# Source the user config
. $ARTICWAKE_CONF/config.env

# Override paths for this system
export ARTICWAKE_SSH_KEY_PATH="$ARTICWAKE_SECRETS/ssh_key"
export ARTICWAKE_PIN_HASH_PATH="$ARTICWAKE_DATA/pin.hash"
EOF
    chmod 600 /etc/conf.d/articwake
    log "Configuration written to /etc/conf.d/articwake"
}

# Setup Tailscale if auth key provided
setup_tailscale() {
    if [ ! -f "$ARTICWAKE_CONF/tailscale_authkey" ]; then
        log "No Tailscale auth key, skipping Tailscale setup"
        return 0
    fi

    log "Configuring Tailscale..."

    # Enable and start tailscaled
    rc-update add tailscaled default
    rc-service tailscaled start

    # Wait for tailscaled to be ready
    sleep 5

    # Authenticate with auth key
    TS_AUTHKEY=$(cat "$ARTICWAKE_CONF/tailscale_authkey")
    if tailscale up --authkey="$TS_AUTHKEY" --ssh 2>&1 | tee -a "$LOG_FILE"; then
        log "Tailscale configured and connected!"
    else
        log "Warning: Tailscale authentication may have failed"
    fi
}

# Create OpenRC service for articwake
create_service() {
    log "Creating OpenRC service..."

    cat > /etc/init.d/articwake <<'SERVICEEOF'
#!/sbin/openrc-run

name="articwake"
description="Wake-on-LAN and LUKS unlock service"
command="/usr/local/bin/articwake"
command_background="yes"
pidfile="/run/${RC_SVCNAME}.pid"
output_log="/var/log/articwake.log"
error_log="/var/log/articwake.log"

depend() {
    need net
    after tailscaled
}

start_pre() {
    # Source configuration
    if [ -f /etc/conf.d/articwake ]; then
        . /etc/conf.d/articwake
    fi

    # Export all ARTICWAKE_ variables
    export ARTICWAKE_BIND_HOST="${ARTICWAKE_BIND_HOST:-0.0.0.0}"
    export ARTICWAKE_PORT="${ARTICWAKE_PORT:-8080}"
    export ARTICWAKE_HOMELAB_MAC
    export ARTICWAKE_HOMELAB_IP
    export ARTICWAKE_HOMELAB_BROADCAST="${ARTICWAKE_HOMELAB_BROADCAST:-255.255.255.255}"
    export ARTICWAKE_SSH_PORT="${ARTICWAKE_SSH_PORT:-2222}"
    export ARTICWAKE_SSH_KEY_PATH
    export ARTICWAKE_PIN_HASH_PATH
}
SERVICEEOF

    chmod 755 /etc/init.d/articwake
    rc-update add articwake default
    log "OpenRC service created and enabled"
}

# Securely delete sensitive plaintext files
cleanup_sensitive() {
    log "Securely deleting sensitive plaintext files..."

    for file in pin ssh_key tailscale_authkey; do
        if [ -f "$ARTICWAKE_CONF/$file" ]; then
            # Overwrite with random data before deletion
            dd if=/dev/urandom of="$ARTICWAKE_CONF/$file" bs=1k count=10 2>/dev/null || true
            sync
            rm -f "$ARTICWAKE_CONF/$file"
            log "Deleted $file"
        fi
    done

    log "Sensitive files deleted. config.env and wifi.conf retained for reference."
}

# Create flag to prevent re-running
mark_complete() {
    touch "$ARTICWAKE_DATA/.setup-complete"
    log "First-boot setup complete!"
}

# Main entry point
main() {
    log "========================================="
    log "Starting articwake first-boot setup"
    log "========================================="

    # Check if already configured
    if [ -f "$ARTICWAKE_DATA/.setup-complete" ]; then
        log "Setup already complete, skipping"
        exit 0
    fi

    check_required_files
    install_packages
    setup_wifi
    setup_articwake
    setup_tailscale
    create_service
    cleanup_sensitive
    mark_complete

    # Start articwake service
    log "Starting articwake service..."
    rc-service articwake start

    log "========================================="
    log "articwake is now running!"
    log "Access the web UI at http://$(hostname -I | awk '{print $1}'):8080"
    log "========================================="
}

main "$@"
