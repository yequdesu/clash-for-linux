#!/usr/bin/env bash
# Cross-compile clash-tui for Linux (musl static binary)
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

TARGET="${1:-x86_64-unknown-linux-musl}"
OUTPUT="clash-tui-linux"

echo "==> Building clash-tui for $TARGET ..."

if [ ! -f src/main.rs ]; then
    echo "Error: not in tui/ directory"
    exit 1
fi

cargo build --release --target "$TARGET" 2>&1

if [ -f "target/$TARGET/release/clash-tui" ]; then
    cp "target/$TARGET/release/clash-tui" "$OUTPUT"
    strip "$OUTPUT" 2>/dev/null || true
    echo "==> Built: $OUTPUT ($(du -h "$OUTPUT" | cut -f1))"
else
    echo "Error: build failed"
    exit 1
fi
