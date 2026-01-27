# articwake

[![CI](https://github.com/a-maccormack/articwake/actions/workflows/test.yml/badge.svg)](https://github.com/a-maccormack/articwake/actions/workflows/test.yml)
[![Release](https://img.shields.io/github/v/release/a-maccormack/articwake)](https://github.com/a-maccormack/articwake/releases/latest)

A Rust-based web service for Raspberry Pi Zero 2 W that provides remote Wake-on-LAN and LUKS unlock functionality for homelab servers.

_Cold server → wake it from the arctic._

## Features

- **Wake-on-LAN**: Send magic packets to wake your server
- **LUKS Unlock**: Remotely send passphrase to dropbear initrd for disk decryption
- **Status Monitoring**: Check if server is reachable and SSH port is open
- **PIN Authentication**: Argon2-hashed PIN with session tokens
- **Rate Limiting**: 10 auth attempts per minute
- **Embedded UI**: Responsive web interface, mobile-friendly
- **Ready-to-flash SD Image**: Alpine Linux image for Pi Zero 2 W

## Quick Start (Raspberry Pi Zero 2 W)

The easiest way to deploy articwake is using the pre-built Alpine Linux SD card image.

### 1. Download and flash the image

```bash
# Download from GitHub releases
wget https://github.com/a-maccormack/articwake/releases/latest/download/articwake-alpine-rpi.img.gz

# Flash to SD card (replace /dev/sdX with your SD card device)
gunzip -c articwake-alpine-rpi.img.gz | sudo dd of=/dev/sdX bs=4M status=progress
sync
```

### 2. Configure secrets

Mount the SD card and create the required configuration files:

```bash
sudo mount /dev/sdX1 /mnt

# Copy example configs
sudo cp /mnt/articwake/config.env.example /mnt/articwake/config.env
sudo cp /mnt/articwake/wifi.conf.example /mnt/articwake/wifi.conf

# Edit config.env with your homelab details
sudo nano /mnt/articwake/config.env
# Set: ARTICWAKE_HOMELAB_MAC="aa:bb:cc:dd:ee:ff"
# Set: ARTICWAKE_HOMELAB_IP="100.x.y.z" (Tailscale IP recommended)

# Edit wifi.conf with your WiFi credentials
sudo nano /mnt/articwake/wifi.conf
# Set: ssid="YourNetworkName"
# Set: psk="YourPassword"

# Copy your SSH key (must be authorized in homelab's initrd)
sudo cp ~/.ssh/homelab_unlock_key /mnt/articwake/ssh_key
sudo chmod 600 /mnt/articwake/ssh_key

# Create PIN file (will be hashed and deleted on first boot)
echo -n "your-pin-here" | sudo tee /mnt/articwake/pin

# Optional: Enable Tailscale (get key from https://login.tailscale.com/admin/settings/keys)
echo "tskey-auth-xxxxx" | sudo tee /mnt/articwake/tailscale_authkey

sudo umount /mnt
```

### 3. Boot and access

1. Insert SD card into Pi Zero 2 W and power on
2. Wait ~2-3 minutes for first boot setup
3. Access the web UI at `http://<pi-ip>:8080` or via Tailscale

## Manual Installation

For other platforms or custom deployments:

### Requirements

- Rust 1.85+ (edition 2024)
- Target server with:
  - Wake-on-LAN enabled in BIOS
  - dropbear SSH in initrd (port 2222) for LUKS unlock

### Configuration

Set these environment variables:

| Variable                      | Required | Default                       | Description                         |
| ----------------------------- | -------- | ----------------------------- | ----------------------------------- |
| `ARTICWAKE_BIND_HOST`         | No       | `127.0.0.1`                   | Bind address                        |
| `ARTICWAKE_PORT`              | No       | `8080`                        | HTTP port                           |
| `ARTICWAKE_HOMELAB_MAC`       | **Yes**  | -                             | Target server MAC address           |
| `ARTICWAKE_HOMELAB_IP`        | **Yes**  | -                             | Target server IP (Tailscale or LAN) |
| `ARTICWAKE_HOMELAB_BROADCAST` | No       | `255.255.255.255`             | WOL broadcast address               |
| `ARTICWAKE_SSH_PORT`          | No       | `2222`                        | dropbear SSH port                   |
| `ARTICWAKE_SSH_KEY_PATH`      | No       | `/etc/secrets/articwake-key`  | Path to SSH private key             |
| `ARTICWAKE_PIN_HASH_PATH`     | No       | `/var/lib/articwake/pin.hash` | Path to Argon2 PIN hash file        |

### Setup

#### 1. Generate SSH keypair

```bash
ssh-keygen -t ed25519 -f articwake-key -N ""
```

Add the public key to your server's initrd SSH authorized keys.

#### 2. Create PIN hash

```bash
# Using articwake's built-in hash command:
echo -n "your-pin" | ./articwake hash-pin > pin.hash

# Or using argon2 CLI tool:
echo -n "your-pin" | argon2 $(openssl rand -base64 16) -id -e > pin.hash
```

#### 3. Build

```bash
cargo build --release

# For Raspberry Pi Zero 2 W (aarch64):
cargo build --release --target aarch64-unknown-linux-gnu
```

#### 4. Run

```bash
export ARTICWAKE_HOMELAB_MAC="aa:bb:cc:dd:ee:ff"
export ARTICWAKE_HOMELAB_IP="100.x.y.z"
./target/release/articwake
```

## Pre-built Binaries

Download from [GitHub Releases](https://github.com/a-maccormack/articwake/releases/latest):

| Binary | Platform | Notes |
|--------|----------|-------|
| `articwake-x86_64-linux` | x86_64 Linux | Standard servers |
| `articwake-aarch64-linux` | ARM64 Linux (glibc) | Raspberry Pi 4/5, ARM servers |
| `articwake-aarch64-linux-musl` | ARM64 Linux (musl) | Alpine Linux, static binary |
| `articwake-armv7-linux` | ARMv7 Linux | Raspberry Pi 2/3 (32-bit) |
| `articwake-alpine-rpi.img.gz` | Pi Zero 2 W | Ready-to-flash SD card image |

## API Endpoints

| Endpoint      | Method | Auth | Description                           |
| ------------- | ------ | ---- | ------------------------------------- |
| `/`           | GET    | No   | Serve embedded web UI                 |
| `/api/auth`   | POST   | No   | Verify PIN, return bearer token       |
| `/api/status` | GET    | Yes  | Server reachability + SSH port status |
| `/api/wol`    | POST   | Yes  | Send Wake-on-LAN magic packet         |
| `/api/unlock` | POST   | Yes  | SSH to dropbear, send passphrase      |

### Authentication

```bash
# Get token
curl -X POST http://localhost:8080/api/auth \
  -H "Content-Type: application/json" \
  -d '{"pin": "your-pin"}'

# Use token
curl http://localhost:8080/api/status \
  -H "Authorization: Bearer <token>"
```

## CLI Commands

```bash
# Run the web server (default)
articwake

# Hash a PIN for the pin.hash file
echo -n "your-pin" | articwake hash-pin
```

## Security

- Bind to `127.0.0.1` by default (expose via Tailscale)
- PIN hashed with Argon2id
- Session tokens expire after 15 minutes
- Rate limiting: 10 auth attempts per minute per IP
- SSH key should be mode 0600, root-only
- Passphrase never stored, only transmitted over WireGuard/Tailscale
- First-boot script securely deletes plaintext secrets after processing

## License

[MIT](LICENSE)
