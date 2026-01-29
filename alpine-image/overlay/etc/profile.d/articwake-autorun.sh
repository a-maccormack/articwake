#!/bin/sh
# This runs when any user logs in - we use it to trigger setup once
MARKER="/var/lib/articwake/.profile-setup-done"
BOOT_MARKER="/media/mmcblk0p1/articwake/.setup-triggered"

# Only run once
[ -f "$MARKER" ] && return
[ -f "$BOOT_MARKER" ] && return

# Mount boot partition if needed
if [ ! -d "/media/mmcblk0p1/articwake" ]; then
    mkdir -p /media/mmcblk0p1
    mount /dev/mmcblk0p1 /media/mmcblk0p1 2>/dev/null || true
fi

# Write marker to boot partition
mkdir -p /media/mmcblk0p1/articwake
echo "profile.d triggered at $(date)" > /media/mmcblk0p1/articwake/profile-debug.log
sync

# Run the actual setup script if it exists
if [ -x /etc/local.d/articwake-setup.start ]; then
    echo "Running articwake setup..."
    /etc/local.d/articwake-setup.start &
fi

mkdir -p /var/lib/articwake
touch "$MARKER"
