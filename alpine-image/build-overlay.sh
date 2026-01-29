#!/bin/sh
# Build the articwake apkovl overlay
# This creates a minimal Alpine overlay that runs our setup script on first boot

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
OVERLAY_DIR="$SCRIPT_DIR/overlay"

# Convert output path to absolute
OUTPUT_ARG="${1:-$SCRIPT_DIR/articwake.apkovl.tar.gz}"
case "$OUTPUT_ARG" in
  /*) OUTPUT_FILE="$OUTPUT_ARG" ;;
  *)  OUTPUT_FILE="$(pwd)/$OUTPUT_ARG" ;;
esac

# Create temporary directory for overlay contents
TMPDIR=$(mktemp -d)
trap "rm -rf $TMPDIR" EXIT

# Copy overlay structure
cp -a "$OVERLAY_DIR"/* "$TMPDIR/"

# Ensure correct permissions - directories need to be accessible
chmod 755 "$TMPDIR/etc"
chmod 755 "$TMPDIR/etc/local.d"
chmod 755 "$TMPDIR/etc/local.d/articwake-setup.start"
chmod 755 "$TMPDIR/etc/runlevels"
chmod 755 "$TMPDIR/etc/runlevels/default"
chmod 755 "$TMPDIR/etc/profile.d"
chmod 755 "$TMPDIR/etc/profile.d/articwake-autorun.sh"
chmod 755 "$TMPDIR/etc/init.d"
chmod 755 "$TMPDIR/etc/init.d/articwake-setup"

# Create the apkovl tarball with root ownership
# Alpine expects paths relative to / without leading slash
tar --owner=root --group=root -czf "$OUTPUT_FILE" -C "$TMPDIR" .

echo "Created overlay: $OUTPUT_FILE"
ls -la "$OUTPUT_FILE"
