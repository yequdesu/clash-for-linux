#!/bin/bash
set -euo pipefail

# Cross-compile clash-tui for Linux from any platform
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

TARGET="x86_64-unknown-linux-musl"
BIN="target/${TARGET}/release/clash-tui"
OUT="${SCRIPT_DIR}/clash-tui-linux"

echo "=== clash-tui cross-compiler ==="
echo "Target: ${TARGET}"
echo ""

if ! rustup target list --installed 2>/dev/null | grep -q "$TARGET"; then
    echo "[+] Adding target: $TARGET"
    rustup target add "$TARGET"
fi

echo "[+] Building release for $TARGET ..."

if command -v cargo-zigbuild &>/dev/null 2>&1; then
    cargo zigbuild --release --target "$TARGET"
else
    cargo build --release --target "$TARGET"
fi

if [[ -f "$BIN" ]]; then
    cp "$BIN" "$OUT"
    chmod +x "$OUT"
    echo ""
    echo "[+] Done: $OUT"
    ls -lh "$OUT"
    echo ""
    echo "Deploy to server:"
    echo "  scp $OUT user@server:~/.local/bin/clash-tui"
else
    echo "[x] Build failed — binary not found"
    exit 1
fi
