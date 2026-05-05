#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════
# Linux CLI&TUI Clash — updater
# ═══════════════════════════════════════════════════════════════
# Run as NORMAL USER (NOT sudo):
#     bash update.sh
#
# The script will ask for sudo password ONCE if binaries need
# to be reinstalled to /usr/local/bin.
# ═══════════════════════════════════════════════════════════════
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"; cd "$SCRIPT_DIR"

_log_ok()   { printf '\r\033[32m[+]\033[0m %s\r\n' "$*"; }
_log_info() { printf '\r\033[36m[i]\033[0m %s\r\n' "$*"; }
_log_warn() { printf '\r\033[33m[!]\033[0m %s\r\n' "$*"; }
_sudo()     { [ "$(id -u)" -eq 0 ] && "$@" || sudo "$@"; }

# ── Pre-flight: cache sudo ──
if [ "$(id -u)" -ne 0 ]; then
    sudo -v || _log_warn "sudo auth failed — binary install may be skipped"
    ( while true; do sudo -v; sleep 60; done ) &
    SUDO_KEEPER=$!
    trap 'kill $SUDO_KEEPER 2>/dev/null' EXIT
fi

_log_info "stopping kernel..."
if command -v clashctl >/dev/null 2>&1; then
    clashctl stop 2>/dev/null || true
fi

# ── Git pull ──
CHANGED=""
if [ -d .git ]; then
    git fetch origin 2>/dev/null || true
    CHANGED=$(git diff --name-only HEAD origin/main 2>/dev/null || git diff --name-only HEAD origin/master 2>/dev/null || echo "")
    if [ -n "$CHANGED" ]; then
        _log_info "pulling updates..."
        git pull 2>/dev/null || _log_warn "git pull failed — update manually"
    else
        _log_info "already up to date"
    fi
fi

# ── Rebuild CLI ──
if echo "$CHANGED" | grep -q '\.go$'; then
    _log_info "rebuilding clashctl..."
    go build -ldflags="-s -w" -o /tmp/clashctl ./cmd/clashctl/ && {
        _sudo install -D /tmp/clashctl /usr/local/bin/clashctl; rm -f /tmp/clashctl
        _log_ok "clashctl updated"
    }
fi

# ── Rebuild TUI ──
if echo "$CHANGED" | grep -q '\.rs$'; then
    _log_info "rebuilding clash-tui..."
    ( cd tui && cargo build --release 2>&1 | tail -3 )
    if [ -f tui/target/release/clash-tui ]; then
        _sudo install -D tui/target/release/clash-tui /usr/local/bin/clash-tui
        _log_ok "clash-tui updated"
    fi
fi

# ── Restart with updated binary ──
if command -v clashctl >/dev/null 2>&1; then
    clashctl start 2>/dev/null || true
    _log_info "checking kernel update..."
    clashctl upgrade-kernel 2>/dev/null || _log_info "kernel is up to date"
else
    _log_warn "clashctl not found — start kernel manually"
fi

_log_ok "update complete"
