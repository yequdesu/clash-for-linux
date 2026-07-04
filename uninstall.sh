#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════
# Linux CLI&TUI Clash — uninstaller
# ═══════════════════════════════════════════════════════════════
# Run as NORMAL USER (NOT sudo):
#     bash uninstall.sh
#
# The script will ask for sudo password ONCE to remove system
# binaries, systemd service, and privileged directories.
# ═══════════════════════════════════════════════════════════════
set -euo pipefail

if command -v clashctl >/dev/null 2>&1 && clashctl self uninstall --help >/dev/null 2>&1; then
    if [ ! -t 0 ] && [ "$#" -eq 0 ]; then
        if [ "$(id -u)" -eq 0 ]; then
            exec clashctl self uninstall --yes
        fi
        exec sudo -E clashctl self uninstall --yes
    fi
    exec clashctl self uninstall "$@"
fi

_log_ok()   { printf '\r\033[32m[+]\033[0m %s\r\n' "$*"; }
_log_info() { printf '\r\033[36m[i]\033[0m %s\r\n' "$*"; }
_log_warn() { printf '\r\033[33m[!]\033[0m %s\r\n' "$*"; }
_sudo()     { [ "$(id -u)" -eq 0 ] && "$@" || sudo "$@"; }
_proxy_unset_cmd() {
    printf 'unset http_proxy HTTP_PROXY https_proxy HTTPS_PROXY all_proxy ALL_PROXY no_proxy NO_PROXY'
}
_active_proxy_env() {
    local out="" key
    for key in http_proxy HTTP_PROXY https_proxy HTTPS_PROXY all_proxy ALL_PROXY no_proxy NO_PROXY; do
        if [ -n "${!key:-}" ]; then
            out="${out}${out:+, }${key}"
        fi
    done
    printf '%s' "$out"
}
_warn_active_proxy_env() {
    local names
    names="$(_active_proxy_env)"
    [ -n "$names" ] || return 0
    _log_warn "current shell proxy variables are active: $names"
    _log_warn "uninstall cannot clear variables exported in the parent shell"
    _log_info "after uninstall, run: $(_proxy_unset_cmd)"
    if [ -t 0 ]; then
        printf '\r\033[36m[i]\033[0m Continue uninstall with active shell proxy variables? [Y/n] '
        read -r answer
        case "${answer:-y}" in
            [Yy]|yes|YES) ;;
            *) _log_warn "cancelled"; exit 1 ;;
        esac
    fi
}
_remind_proxy_cleanup() {
    local names
    names="$(_active_proxy_env)"
    [ -n "$names" ] || return 0
    _log_warn "shell proxy variables may still be active in this terminal: $names"
    _log_info "restore current shell network with: $(_proxy_unset_cmd)"
    _log_info "or open a new shell session"
}

_load_install_env() {
    local file="$1"
    [ -f "$file" ] || return 0
    while IFS='=' read -r key val; do
        key="${key%$'\r'}"
        val="${val%$'\r'}"
        case "$key" in
            CLASH_BASE_DIR) CLASH_BASE_DIR="$val" ;;
            SERVICE_NAME|CLASH_SERVICE_NAME) SERVICE_NAME="$val" ;;
            KERNEL_NAME) KERNEL_NAME="$val" ;;
        esac
    done < "$file"
}

_remove_systemd_unit() {
    local service="$1"
    [ -n "$service" ] || return 0
    if [ -f "/etc/systemd/system/${service}.service" ]; then
        _log_info "removing systemd service: ${service}.service"
        _sudo systemctl stop "$service" 2>/dev/null || true
        _sudo systemctl disable "$service" 2>/dev/null || true
        _sudo rm -f "/etc/systemd/system/${service}.service"
        _sudo systemctl daemon-reload 2>/dev/null || true
    fi
}

_pid_matches_kernel() {
    local pid="$1"
    local kernel_bin="${CLASH_BASE_DIR}/bin/${KERNEL_NAME}"
    local expected exe exe_real cmd0 cmd0_real
    expected="$(readlink -f "$kernel_bin" 2>/dev/null || printf '%s' "$kernel_bin")"

    exe="$(readlink "/proc/${pid}/exe" 2>/dev/null || true)"
    exe_real="$(readlink -f "/proc/${pid}/exe" 2>/dev/null || true)"
    case "$exe" in
        "$expected"|"$expected (deleted)"|"$kernel_bin"|"$kernel_bin (deleted)") return 0 ;;
    esac
    [ -n "$exe_real" ] && [ "$exe_real" = "$expected" ] && return 0

    if [ -r "/proc/${pid}/cmdline" ]; then
        cmd0="$(tr '\0' '\n' < "/proc/${pid}/cmdline" 2>/dev/null | sed -n '1p')"
        if [ -n "$cmd0" ]; then
            cmd0_real="$(readlink -f "$cmd0" 2>/dev/null || printf '%s' "$cmd0")"
            [ "$cmd0_real" = "$expected" ] && return 0
        fi
    fi
    return 1
}

_stop_pid_file() {
    local pid_file="$1"
    [ -f "$pid_file" ] || return 0
    local pid
    pid="$(cat "$pid_file" 2>/dev/null || true)"
    case "$pid" in
        ""|*[!0-9]*)
            _log_warn "invalid pid file, removing: $pid_file"
            _sudo rm -f "$pid_file" 2>/dev/null || rm -f "$pid_file" 2>/dev/null || true
            return 0
            ;;
    esac
    if kill -0 "$pid" 2>/dev/null; then
        if ! _pid_matches_kernel "$pid"; then
            _log_warn "pid file points to non-managed process $pid; not killing"
            _sudo rm -f "$pid_file" 2>/dev/null || rm -f "$pid_file" 2>/dev/null || true
            return 0
        fi
        _log_info "stopping managed kernel pid $pid"
        _sudo kill "$pid" 2>/dev/null || true
        sleep 1
        if kill -0 "$pid" 2>/dev/null; then
            _log_warn "managed kernel pid $pid did not exit after SIGTERM; sending SIGKILL"
            _sudo kill -9 "$pid" 2>/dev/null || true
        fi
    fi
    _sudo rm -f "$pid_file" 2>/dev/null || rm -f "$pid_file" 2>/dev/null || true
}

REAL_USER="${SUDO_USER:-${USER:-$(id -un 2>/dev/null || printf root)}}"
REAL_HOME="$(eval echo ~"$REAL_USER")"
_load_install_env /etc/clashctl/install.env
_load_install_env "$REAL_HOME/.config/clashctl/install.env"
CLASH_BASE_DIR="${CLASH_BASE_DIR:-$REAL_HOME/clashctl}"
KERNEL_NAME="${KERNEL_NAME:-mihomo}"
SERVICE_NAME="${SERVICE_NAME:-clashctl}"
_valid_name() { printf '%s' "$1" | grep -Eq '^[A-Za-z0-9_.@-]+$'; }
_valid_name "$KERNEL_NAME" || KERNEL_NAME="mihomo"
_valid_name "$SERVICE_NAME" || SERVICE_NAME="clashctl"
case ":$PATH:" in *:/usr/local/bin:*) ;; *) export PATH="/usr/local/bin:$PATH" ;; esac

echo ""
_log_info "Linux CLI & TUI Clash — Uninstaller"
_log_info "You will be asked for your sudo password ONCE."
_warn_active_proxy_env

if [ "$(id -u)" -ne 0 ]; then
    sudo -v || _log_warn "sudo auth failed — some items may not be removed"
    ( while true; do sudo -v; sleep 60; done ) &
    SUDO_KEEPER=$!
    trap 'kill $SUDO_KEEPER 2>/dev/null' EXIT
fi

# ── Stop kernel ──
_log_info "stopping kernel..."
if command -v clashctl >/dev/null 2>&1; then
    clashctl stop 2>/dev/null || true
fi
_stop_pid_file "${CLASH_BASE_DIR}/runtime/${KERNEL_NAME}.pid"

# ── Remove systemd service ──
_remove_systemd_unit "$SERVICE_NAME"
[ "$SERVICE_NAME" = "clashctl" ] || _remove_systemd_unit clashctl

# ── Remove binaries ──
for bin in clashctl clash-tui; do
    if [ -f "/usr/local/bin/$bin" ]; then
        _sudo rm -f "/usr/local/bin/$bin"
        _log_ok "removed /usr/local/bin/$bin"
    fi
done

# ── Prompt to keep kernel/yq binaries ──
for bin in "$KERNEL_NAME" yq; do
    local_path="${CLASH_BASE_DIR}/bin/${bin}"
    system_path="/usr/local/bin/${bin}"
    if [ -f "$local_path" ]; then
        printf '\r\033[36m[i]\033[0m Keep %s binary for future use? [Y/n] ' "$bin"
        read -r answer
        case "${answer:-y}" in
            [Yy]|yes|YES)
                _sudo cp "$local_path" "$system_path" 2>/dev/null || true
                _sudo chmod 755 "$system_path" 2>/dev/null || true
                _log_ok "kept $bin at $system_path"
                ;;
            *)
                _log_info "removing $bin binary"
                _sudo rm -f "$system_path" 2>/dev/null || true
                ;;
        esac
    fi
done

# ── Clean shell RC ──
for rc in "$REAL_HOME/.zshrc" "$REAL_HOME/.bashrc"; do
    if [ -f "$rc" ]; then
        sed -i '/# clashctl START/,/# clashctl END/d' "$rc" 2>/dev/null || true
        _log_info "cleaned $rc"
    fi
done
rm -f "$REAL_HOME/.config/fish/conf.d/clashctl.fish" 2>/dev/null || true

# ── Clean cron ──
if command -v crontab >/dev/null 2>&1; then
    crontab -l 2>/dev/null | grep -v 'clashctl sub update' | crontab - 2>/dev/null || true
    _log_info "cleaned crontab"
fi

# ── Remove install directories ──
for dir in "$CLASH_BASE_DIR" "$REAL_HOME/.config/clashctl" "$REAL_HOME/.config/clash-tui" /etc/clashctl; do
    if [ -d "$dir" ]; then
        _sudo rm -rf "$dir" 2>/dev/null || rm -rf "$dir" 2>/dev/null || true
        _log_ok "removed $dir"
    fi
done

echo ''
_log_ok "uninstall complete"
_remind_proxy_cleanup
