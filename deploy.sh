#!/usr/bin/env bash
set -euo pipefail

INSTALL_DIR="${INSTALL_DIR:-$HOME/bin}"
BINARY="agent-search"

echo "Building $BINARY (release)..."
cargo build --release

mkdir -p "$INSTALL_DIR"
cp "target/release/$BINARY" "$INSTALL_DIR/$BINARY"

echo "Installed $BINARY -> $INSTALL_DIR/$BINARY"
"$INSTALL_DIR/$BINARY" --version 2>/dev/null || true
