#!/usr/bin/env bash
# build.sh — Build aktags release binary
# Run this on your dev machine or in CI before pushing to BlueAK
set -euo pipefail

BINARY="aktags"
TARGET="x86_64-unknown-linux-gnu"

echo "Building $BINARY..."
cargo build --release --target "$TARGET"

BINARY_PATH="target/$TARGET/release/$BINARY"
if [[ -f "$BINARY_PATH" ]]; then
    echo "✓ Binary: $BINARY_PATH ($(du -sh "$BINARY_PATH" | cut -f1))"
    # Copy to a predictable output location for the BlueAK Containerfile
    cp "$BINARY_PATH" "dist/$BINARY"
    echo "✓ Copied to dist/$BINARY"
else
    echo "✗ Build failed"
    exit 1
fi
