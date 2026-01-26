# articwake

[![CI](https://github.com/a-maccormack/articwake/actions/workflows/test.yml/badge.svg)](https://github.com/a-maccormack/articwake/actions/workflows/test.yml)
[![Release](https://img.shields.io/github/v/release/a-maccormack/articwake)](https://github.com/a-maccormack/articwake/releases/latest)

A Rust-based web service for Raspberry Pi Zero 2 W that provides remote Wake-on-LAN and LUKS unlock functionality for homelab servers.

*Cold server → wake it from the arctic.*

## Features

- **Wake-on-LAN**: Send magic packets to wake your server
- **LUKS Unlock**: Remotely send passphrase to dropbear initrd for disk decryption
- **Status Monitoring**: Check if server is reachable and SSH port is open
- **PIN Authentication**: Argon2-hashed PIN with session tokens
- **Rate Limiting**: 10 auth attempts per minute
- **Embedded UI**: Responsive web interface, mobile-friendly

## Requirements

- Rust 1.85+ (edition 2024)
- Target server with:
  - Wake-on-LAN enabled in BIOS
  - dropbear SSH in initrd (port 2222) for LUKS unlock
  - Network cable connected (WOL doesn't work over WiFi)

## Configuration

Set these environment variables:

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `ARTICWAKE_BIND_HOST` | No | `127.0.0.1` | Bind address |
| `ARTICWAKE_PORT` | No | `8080` | HTTP port |
| `ARTICWAKE_HOMELAB_MAC` | **Yes** | - | Target server MAC address |
| `ARTICWAKE_HOMELAB_IP` | **Yes** | - | Target server IP (Tailscale or LAN) |
| `ARTICWAKE_HOMELAB_BROADCAST` | No | `255.255.255.255` | WOL broadcast address |
| `ARTICWAKE_SSH_PORT` | No | `2222` | dropbear SSH port |
| `ARTICWAKE_SSH_KEY_PATH` | No | `/etc/secrets/articwake-key` | Path to SSH private key |
| `ARTICWAKE_PIN_HASH_PATH` | No | `/var/lib/articwake/pin.hash` | Path to Argon2 PIN hash file |

## Setup

### 1. Generate SSH keypair

```bash
ssh-keygen -t ed25519 -f articwake-key -N ""
```

Add the public key to your server's initrd SSH authorized keys.

### 2. Create PIN hash

```bash
# Install argon2 CLI tool, then:
echo -n "your-pin" | argon2 $(openssl rand -base64 16) -id -e > pin.hash
```

### 3. Build

```bash
cargo build --release

# For Raspberry Pi Zero 2 W (aarch64):
cargo build --release --target aarch64-unknown-linux-gnu
```

### 4. Run

```bash
export ARTICWAKE_HOMELAB_MAC="aa:bb:cc:dd:ee:ff"
export ARTICWAKE_HOMELAB_IP="100.x.y.z"
./target/release/articwake
```

## API Endpoints

| Endpoint | Method | Auth | Description |
|----------|--------|------|-------------|
| `/` | GET | No | Serve embedded web UI |
| `/api/auth` | POST | No | Verify PIN, return bearer token |
| `/api/status` | GET | Yes | Server reachability + SSH port status |
| `/api/wol` | POST | Yes | Send Wake-on-LAN magic packet |
| `/api/unlock` | POST | Yes | SSH to dropbear, send passphrase |

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

## Security

- Bind to `127.0.0.1` by default (expose via Tailscale)
- PIN hashed with Argon2id
- Session tokens expire after 15 minutes
- Rate limiting: 10 auth attempts per minute per IP
- SSH key should be mode 0600, root-only
- Passphrase never stored, only transmitted over WireGuard/Tailscale

## License

MIT
