================================================================================
                    articwake - Raspberry Pi Zero 2 W Setup
================================================================================

This SD card image contains Alpine Linux pre-configured for articwake.
On first boot, it will configure WiFi, hash your PIN, and start the service.

--------------------------------------------------------------------------------
REQUIRED FILES - Create these in /boot/articwake/ before first boot:
--------------------------------------------------------------------------------

1. config.env
   Environment variables for articwake. Copy from config.env.example.
   Required settings:
     ARTICWAKE_HOMELAB_MAC="aa:bb:cc:dd:ee:ff"
     ARTICWAKE_HOMELAB_IP="100.x.y.z"

2. wifi.conf
   WiFi credentials in wpa_supplicant format. Copy from wifi.conf.example.
   Required settings:
     ssid="YourNetworkName"
     psk="YourNetworkPassword"

3. ssh_key
   Ed25519 private key for SSH access to your homelab's initrd.
   This is the key authorized in your server's dropbear initrd.
   Generate with: ssh-keygen -t ed25519 -f ssh_key -N ""

4. pin
   Plain text file containing your desired PIN (e.g., just "1234").
   This will be hashed with Argon2 and securely deleted on first boot.

--------------------------------------------------------------------------------
OPTIONAL FILES:
--------------------------------------------------------------------------------

5. tailscale_authkey
   Tailscale authentication key for automatic Tailscale setup.
   Get from: https://login.tailscale.com/admin/settings/keys
   If not provided, articwake will only be accessible on your local network.

--------------------------------------------------------------------------------
SETUP STEPS:
--------------------------------------------------------------------------------

1. Flash this image to an SD card:
   gunzip -c articwake-alpine-rpi.img.gz | sudo dd of=/dev/sdX bs=4M status=progress

2. Mount the SD card:
   sudo mount /dev/sdX1 /mnt

3. Create the configuration files:
   sudo cp /mnt/articwake/config.env.example /mnt/articwake/config.env
   sudo cp /mnt/articwake/wifi.conf.example /mnt/articwake/wifi.conf
   # Edit both files with your actual values

4. Add your SSH key:
   sudo cp ~/.ssh/your_homelab_key /mnt/articwake/ssh_key
   sudo chmod 600 /mnt/articwake/ssh_key

5. Create your PIN file:
   echo -n "your-pin-here" | sudo tee /mnt/articwake/pin

6. Optionally add Tailscale auth key:
   echo "tskey-auth-xxxxx" | sudo tee /mnt/articwake/tailscale_authkey

7. Unmount and boot:
   sudo umount /mnt
   # Insert SD card into Pi Zero 2 W and power on

--------------------------------------------------------------------------------
FIRST BOOT:
--------------------------------------------------------------------------------

First boot takes approximately 2-3 minutes:
- Connects to WiFi
- Installs required packages
- Hashes your PIN (and securely deletes the plaintext)
- Configures and starts articwake service
- Optionally configures Tailscale

--------------------------------------------------------------------------------
ACCESS:
--------------------------------------------------------------------------------

After first boot, access the web UI at:
- Local: http://<pi-ip>:8080
- Tailscale: http://<tailscale-hostname>:8080

Find the Pi's IP address:
- Check your router's DHCP leases
- Or: tailscale status (if Tailscale configured)

SSH access is available:
- ssh root@<pi-ip> (Alpine default, consider changing)

--------------------------------------------------------------------------------
LOGS:
--------------------------------------------------------------------------------

First-boot setup log: /var/log/articwake-setup.log
Service log: /var/log/articwake.log

--------------------------------------------------------------------------------
TROUBLESHOOTING:
--------------------------------------------------------------------------------

Pi doesn't connect to WiFi:
- Check wifi.conf format (wpa_supplicant syntax)
- Verify SSID and password are correct
- Check country code matches your region

articwake won't start:
- Check /var/log/articwake-setup.log for errors
- Verify config.env has valid MAC and IP addresses
- Ensure ssh_key is a valid Ed25519 private key

Can't unlock homelab:
- Verify SSH key is authorized in homelab's initrd
- Check ARTICWAKE_SSH_PORT matches your dropbear port
- Test SSH manually: ssh -p 2222 -i /etc/articwake/ssh_key root@<homelab-ip>

================================================================================
                    https://github.com/a-maccormack/articwake
================================================================================
