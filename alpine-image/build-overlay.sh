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

# Ensure correct permissions
chmod 755 "$TMPDIR/etc/local.d/articwake-setup.start"

# Create the apkovl tarball
# Alpine expects paths relative to / without leading slash
tar -czf "$OUTPUT_FILE" -C "$TMPDIR" .

echo "Created overlay: $OUTPUT_FILE"
ls -la "$OUTPUT_FILE"
