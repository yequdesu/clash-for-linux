#!/usr/bin/env bash
# Linux CLI&TUI Clash — uninstaller

_log_ok()   { printf '\r\033[32m[+]\033[0m %s\r\n' "$*"; }
_log_info() { printf '\r\033[36m[i]\033[0m %s\r\n' "$*"; }
_log_warn() { printf '\r\033[33m[!]\033[0m %s\r\n' "$*"; }
_sudo()     { [ "$(id -u)" -eq 0 ] && "$@" || sudo "$@"; }

CLASH_BASE_DIR="${CLASH_BASE_DIR:-$HOME/clashctl}"
case ":$PATH:" in *:/usr/local/bin:*) export PATH="/usr/local/bin:$PATH" ;; esac

_log_info "stopping kernel..."
if command -v clashctl >/dev/null 2>&1; then
    clashctl stop 2>/dev/null || true
fi

for name in mihomo clash; do
    pgrep -x "$name" >/dev/null 2>&1 && { _sudo pkill -9 -x "$name" 2>/dev/null || true; sleep 0.3; }
done

# Remove systemd service
if [ -f /etc/systemd/system/clashctl.service ]; then
    _log_info "removing systemd service..."
    _sudo systemctl stop clashctl 2>/dev/null || true
    _sudo systemctl disable clashctl 2>/dev/null || true
    _sudo rm -f /etc/systemd/system/clashctl.service
    _sudo systemctl daemon-reload 2>/dev/null || true
fi

# Remove binaries
for bin in clashctl clash-tui; do
    if [ -f "/usr/local/bin/$bin" ]; then
        _sudo rm -f "/usr/local/bin/$bin"
        _log_ok "removed /usr/local/bin/$bin"
    fi
done

# Clean shell RC
for rc in "$HOME/.zshrc" "$HOME/.bashrc"; do
    if [ -f "$rc" ]; then
        sed -i '/# clashctl START/,/# clashctl END/d' "$rc" 2>/dev/null || true
        _log_info "cleaned $rc"
    fi
done
rm -f "$HOME/.config/fish/conf.d/clashctl.fish" 2>/dev/null || true

# Clean cron
if command -v crontab >/dev/null 2>&1; then
    crontab -l 2>/dev/null | grep -v 'clashctl sub update' | crontab - 2>/dev/null || true
    _log_info "cleaned crontab"
fi

# Remove install directories
for dir in "$CLASH_BASE_DIR" "$HOME/.config/clashctl" "$HOME/.config/clash-tui" /etc/clashctl; do
    if [ -d "$dir" ]; then
        _sudo rm -rf "$dir" 2>/dev/null || rm -rf "$dir" 2>/dev/null
        _log_ok "removed $dir"
    fi
done

echo ''
_log_ok "uninstall complete"
