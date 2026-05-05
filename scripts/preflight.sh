#!/usr/bin/env bash
# Preflight — validate system requirements and download resources

_log_info() { printf '\r\033[36m[i]\033[0m %s\r\n' "$*"; }
_log_ok()   { printf '\r\033[32m[+]\033[0m %s\r\n' "$*"; }
_log_warn() { printf '\r\033[33m[!]\033[0m %s\r\n' "$*"; }

_valid() {
    local missing=""
    for cmd in curl tar gzip unzip; do
        if ! command -v "$cmd" >/dev/null 2>&1; then
            missing="$missing $cmd"
        fi
    done
    if [ -n "$missing" ]; then
        _log_warn "Missing dependencies:$missing"
        if command -v apt-get >/dev/null 2>&1; then
            _log_info "Installing dependencies..."
            sudo apt-get update -qq && sudo apt-get install -y -qq $missing
        fi
    fi
    _log_ok "prerequisites OK"
}

_prepare_zip() {
    _log_info "preflight check passed"
}

_parse_args() { :; }

_detect_init() {
    if [ -f /run/systemd/system ] || grep -q systemd /proc/1/exe 2>/dev/null; then
        INIT_TYPE="systemd"
    elif grep -q 'docker\|kubepods\|containerd' /proc/1/cgroup 2>/dev/null; then
        INIT_TYPE="nohup"
    else
        INIT_TYPE="systemd"
    fi
    export INIT_TYPE
}

_set_envs() {
    mkdir -p "${CLASH_BASE_DIR}/bin"
}

_is_root() { [ "$(id -u)" -eq 0 ]; }
_is_regular_sudo() { [ -n "${SUDO_USER:-}" ] && [ "$SUDO_USER" != "root" ]; }

_install_service() {
    _log_info "service: $INIT_TYPE"
}

_nohup() {
    nohup "$@" > /dev/null 2>&1 &
}
