#!/bin/bash
# Wrapper script for cargo build that automatically copies the binary to binary/
# This script should be marked as executable: chmod +x build.sh

set -e

# Parse command-line arguments to detect build profile
PROFILE="debug"
CARGO_ARGS=()

for arg in "$@"; do
    if [ "$arg" = "--release" ]; then
        PROFILE="release"
    fi
    CARGO_ARGS+=("$arg")
done

# Run cargo build with provided arguments
echo "Building with: cargo build ${CARGO_ARGS[*]}"
cargo build "${CARGO_ARGS[@]}"

# Create binary directory if it doesn't exist
mkdir -p binary

# Copy the built binary
BINARY_PATH="target/$PROFILE/herakles-node-exporter"
if [ -f "$BINARY_PATH" ]; then
    cp "$BINARY_PATH" binary/herakles-node-exporter
    chmod +x binary/herakles-node-exporter
    echo "✓ Binary copied to binary/herakles-node-exporter"
else
    echo "✗ Binary not found at $BINARY_PATH"
    exit 1
fi
