#!/usr/bin/env bash

# Homebrew postinstall script for nushell
# This script creates a symlink from 'nushell' to 'nu' for convenience

set -euo pipefail

# Get the bin directory where Homebrew installed the binary
BIN_DIR="$1"
NU_BINARY="$BIN_DIR/nu"
NUSHELL_SYMLINK="$BIN_DIR/nushell"

# Create symlink if nu binary exists and nushell symlink doesn't exist
if [ -f "$NU_BINARY" ] && [ ! -L "$NUSHELL_SYMLINK" ] && [ ! -f "$NUSHELL_SYMLINK" ]; then
    ln -sf "$NU_BINARY" "$NUSHELL_SYMLINK"
    echo "Created symlink: $NUSHELL_SYMLINK -> $NU_BINARY"
    echo "You can now use either 'nu' or 'nushell' to start the shell"
else
    echo "Symlink creation skipped (already exists or nu binary not found)"
fi 