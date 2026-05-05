#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════
# Linux CLI&TUI Clash — TUI-only installer
# ═══════════════════════════════════════════════════════════════
# Run as NORMAL USER (NOT sudo):
#     bash install_tui.sh
#     bash install_tui.sh --force
# ═══════════════════════════════════════════════════════════════
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"; cd "$SCRIPT_DIR"

_log_fatal() { printf '\r\n\033[31m[x]\033[0m %s\r\n\r\n' "$*" >&2; exit 1; }
_log_warn()  { printf '\r\033[33m[!]\033[0m %s\r\n' "$*" >&2; }
_log_info()  { printf '\r\033[36m[i]\033[0m %s\r\n' "$*"; }
_log_ok()    { printf '\r\033[32m[+]\033[0m %s\r\n' "$*"; }
_log_section(){ printf '\r\n\033[1m\033[36m=== %s ===\033[0m\r\n\r\n' "$1"; }

_sudo() { [ "$(id -u)" -eq 0 ] && "$@" || sudo "$@"; }

FORCE=false
for arg in "$@"; do
    case "$arg" in
        --force) FORCE=true ;;
        --help|-h)
            echo "Usage: bash install_tui.sh [--force]"
            echo ""
            echo "Build and install the clash-tui dashboard."
            echo "Requires cargo (Rust toolchain). Will install rustup if missing."
            exit 0
            ;;
    esac
done

REAL_USER="${SUDO_USER:-$USER}"
REAL_HOME="$(eval echo ~"$REAL_USER")"

echo ""
_log_section "clash-tui — Installer"

if [ "$(id -u)" -ne 0 ]; then
    sudo -v || _log_fatal "sudo authentication failed."
    ( while true; do sudo -v; sleep 60; done ) &
    SUDO_KEEPER=$!
    trap 'kill $SUDO_KEEPER 2>/dev/null' EXIT
fi

# ── Force reinstall ──
if $FORCE && [ -f /usr/local/bin/clash-tui ]; then
    _log_info "removing previous installation..."
    _sudo rm -f /usr/local/bin/clash-tui
fi

if [ -x /usr/local/bin/clash-tui ] && ! $FORCE; then
    _log_info "clash-tui already installed — use --force to reinstall"
    exit 0
fi

# ── Ensure Rust toolchain ──
if ! command -v cargo >/dev/null 2>&1; then
    if command -v rustup >/dev/null 2>&1; then
        _log_info "rustup found, ensuring toolchain..."
        rustup default stable 2>/dev/null || true
    elif curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y 2>/dev/null; then
        source "$REAL_HOME/.cargo/env"
        _log_ok "Rust installed"
    else
        _log_fatal "Rust install failed — install rustup manually, then re-run"
    fi
fi

[ -f "$REAL_HOME/.cargo/env" ] && source "$REAL_HOME/.cargo/env"

# ── Build ──
_log_info "building clash-tui (this may take 2-5 minutes)..."
( cd "$SCRIPT_DIR/tui" && cargo build --release )
if [ -f tui/target/release/clash-tui ]; then
    _sudo install -D tui/target/release/clash-tui /usr/local/bin/clash-tui
    _log_ok "clash-tui installed to /usr/local/bin/clash-tui"
else
    _log_fatal "clash-tui build failed"
fi
