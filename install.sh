#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════
# Linux CLI&TUI Clash — installer
# ═══════════════════════════════════════════════════════════════
# Run as NORMAL USER (NOT sudo):
#     bash install.sh
#     bash install.sh --with-tui
#     bash install.sh --tui-only
#     bash install.sh --force
#
# The script will ask for sudo password ONCE at the start and
# cache it for all privileged operations (install to /usr/local/bin,
# systemd service, setcap, etc.).
# ═══════════════════════════════════════════════════════════════
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"; cd "$SCRIPT_DIR"

_log_fatal() { printf '\r\n\033[31m[x]\033[0m %s\r\n\r\n' "$*" >&2; exit 1; }
_log_warn()  { printf '\r\033[33m[!]\033[0m %s\r\n' "$*" >&2; }
_log_info()  { printf '\r\033[36m[i]\033[0m %s\r\n' "$*"; }
_log_ok()    { printf '\r\033[32m[+]\033[0m %s\r\n' "$*"; }
_log_section(){ printf '\r\n\033[1m\033[36m=== %s ===\033[0m\r\n\r\n' "$1"; }

# ── sudo helper ──
_sudo() { [ "$(id -u)" -eq 0 ] && "$@" || sudo "$@"; }

# ═══════════════════════════════
#  Flags
# ═══════════════════════════════
FORCE=false; WITH_TUI=false; SKIP_CLI=false; TUI_ONLY=false; RESET_CONFIG=false
RELEASE_TOOLS_INSTALLED=false
CLASHCTL_SOURCE_URL=""
CLASH_TUI_SOURCE_URL=""
for arg in "$@"; do
    case "$arg" in
        --force)    FORCE=true ;;
        --reset-config) RESET_CONFIG=true ;;
        --with-tui|--tui) WITH_TUI=true ;;
        --skip-cli) SKIP_CLI=true ;;
        --tui-only) TUI_ONLY=true; SKIP_CLI=true; WITH_TUI=true ;;
        --help|-h)
            echo "Usage: bash install.sh [flags]"
            echo ""
            echo "Flags:"
            echo "  --with-tui (or --tui)    Also build and install the TUI dashboard"
            echo "  --tui-only               Only install the TUI (skip CLI)"
            echo "  --force                  Force reinstall, overwrite existing"
            echo "  --reset-config           With --force, reset resources/config/subscriptions"
            echo "  --skip-cli               Skip CLI build (use pre-built or skip)"
            echo ""
            echo "Run as normal user. Password asked once at the beginning."
            exit 0
            ;;
    esac
done

if $RESET_CONFIG && ! $FORCE; then
    _log_fatal "--reset-config requires --force"
fi

# ═══════════════════════════════
#  Preflight: sudo credential
# ═══════════════════════════════
echo ""
_log_section "Linux CLI & TUI Clash — Installer"
_log_info "This script installs system binaries and may set up a systemd service."
_log_info "You will be asked for your sudo password ONCE now."

if [ "$(id -u)" -ne 0 ]; then
    sudo -v || _log_fatal "sudo authentication failed. Please re-run and enter your password."
    # Keep sudo alive in background during the entire script
    ( while true; do sudo -v; sleep 60; done ) &
    SUDO_KEEPER=$!
    trap 'kill $SUDO_KEEPER 2>/dev/null' EXIT
fi

# ═══════════════════════════════
#  Resolve paths
# ═══════════════════════════════
REAL_USER="${SUDO_USER:-$USER}"
REAL_HOME="$(eval echo ~"$REAL_USER")"
CLASH_BASE_DIR="${CLASH_BASE_DIR:-$REAL_HOME/clashctl}"

# ── .env load ──
[ -f "$SCRIPT_DIR/.env" ] && . "$SCRIPT_DIR/.env" 2>/dev/null || true
CLASH_BASE_DIR="${CLASH_BASE_DIR:-$REAL_HOME/clashctl}"
KERNEL_NAME="${KERNEL_NAME:-mihomo}"
SERVICE_NAME="${SERVICE_NAME:-${CLASH_SERVICE_NAME:-clashctl}}"
INIT_TYPE="${INIT_TYPE:-}"
case "$KERNEL_NAME" in ""|*[!a-zA-Z0-9_.@-]*) _log_fatal "invalid KERNEL_NAME: $KERNEL_NAME" ;; esac
case "$SERVICE_NAME" in ""|*[!a-zA-Z0-9_.@-]*) _log_fatal "invalid SERVICE_NAME: $SERVICE_NAME" ;; esac
BIN_KERNEL="${CLASH_BASE_DIR}/bin/${KERNEL_NAME}"

_pid_matches_kernel() {
    local pid="$1"
    local expected exe exe_real cmd0 cmd0_real
    expected="$(readlink -f "$BIN_KERNEL" 2>/dev/null || printf '%s' "$BIN_KERNEL")"

    exe="$(readlink "/proc/${pid}/exe" 2>/dev/null || true)"
    exe_real="$(readlink -f "/proc/${pid}/exe" 2>/dev/null || true)"
    case "$exe" in
        "$expected"|"$expected (deleted)"|"$BIN_KERNEL"|"$BIN_KERNEL (deleted)") return 0 ;;
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

# ── Detect init system ──
detect_init() {
    [ -n "$INIT_TYPE" ] && return
    if [ -d /run/systemd/system ] || grep -q systemd /proc/1/exe 2>/dev/null; then
        INIT_TYPE="systemd"
    elif grep -q 'docker\|kubepods\|containerd' /proc/1/cgroup 2>/dev/null; then
        INIT_TYPE="nohup"
    else
        INIT_TYPE="nohup"
    fi
}
detect_init

_log_info "kernel: $KERNEL_NAME  |  service: $SERVICE_NAME  |  init: $INIT_TYPE  |  install: $CLASH_BASE_DIR"

# ═══════════════════════════════
#  Stop this installation's existing kernel only
# ═══════════════════════════════
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
        _log_info "stopping existing managed kernel pid $pid"
        _sudo kill "$pid" 2>/dev/null || true
        sleep 1
        if kill -0 "$pid" 2>/dev/null; then
            _log_warn "managed kernel pid $pid did not exit after SIGTERM; sending SIGKILL"
            _sudo kill -9 "$pid" 2>/dev/null || true
        fi
    fi
    _sudo rm -f "$pid_file" 2>/dev/null || rm -f "$pid_file" 2>/dev/null || true
}

_stop_existing_kernel() {
    if command -v clashctl >/dev/null 2>&1; then
        CLASH_BASE_DIR="$CLASH_BASE_DIR" SERVICE_NAME="$SERVICE_NAME" KERNEL_NAME="$KERNEL_NAME" INIT_TYPE="$INIT_TYPE" clashctl stop || \
            _log_warn "clashctl stop failed, falling back to direct service/pid cleanup"
    fi
    if command -v systemctl >/dev/null 2>&1 && [ -f "/etc/systemd/system/${SERVICE_NAME}.service" ]; then
        _sudo systemctl stop "$SERVICE_NAME" || _log_warn "systemctl stop ${SERVICE_NAME} failed, falling back to pid cleanup"
    fi
    _stop_pid_file "${CLASH_BASE_DIR}/runtime/${KERNEL_NAME}.pid"
}
_stop_existing_kernel

# Clean old RC cruft
for rc in "$REAL_HOME/.zshrc" "$REAL_HOME/.bashrc"; do
    [ -f "$rc" ] && sed -i '/# clashctl START/,/# clashctl END/d' "$rc" 2>/dev/null || true
done
rm -f "$REAL_HOME/.config/fish/conf.d/clashctl.fish" 2>/dev/null || true

# ═══════════════════════════════════════════════
#  Mirror fallback — try direct GitHub first, then gh-proxy.org
# ═══════════════════════════════════════════════
_gh_download() {
    local raw_url="$1"
    local output="$2"
    local desc="$3"
    _log_info "downloading ${desc}..."
    if curl -fSL --progress-bar "$raw_url" -o "$output" 2>/dev/null; then
        return 0
    fi
    _log_info "direct GitHub failed, trying gh-proxy.org..."
    if curl -fSL --progress-bar "https://gh-proxy.org/${raw_url}" -o "$output"; then
        return 0
    fi
    _log_warn "${desc} download failed"
    return 1
}

# ═══════════════════════════════════════════════
#  Function: download kernel binary
# ═══════════════════════════════════════════════
_mihomo_arch() {
    local arch="linux-amd64"
    case "$(uname -m)" in
        aarch64|arm64) arch="linux-arm64" ;;
        armv7l)        arch="linux-armv7" ;;
    esac
    printf '%s' "$arch"
}

_yq_arch() {
    local arch="amd64"
    case "$(uname -m)" in aarch64|arm64) arch="arm64" ;; esac
    printf '%s' "$arch"
}

_release_arch() {
    case "$(uname -m)" in
        x86_64|amd64) printf 'amd64' ;;
        aarch64|arm64) printf 'arm64' ;;
        *) return 1 ;;
    esac
}

_download_kernel() {
    local ver="${VERSION_MIHOMO:-v1.19.17}"
    local arch
    arch="$(_mihomo_arch)"
    local filename="mihomo-${arch}-${ver}.gz"
    local raw_url="https://github.com/MetaCubeX/mihomo/releases/download/${ver}/${filename}"
    if _gh_download "$raw_url" /tmp/mihomo.gz "$KERNEL_NAME ${ver}"; then
        gunzip -f /tmp/mihomo.gz
        _sudo install -D /tmp/mihomo "$BIN_KERNEL"
        _sudo chmod 755 "$BIN_KERNEL"
        rm -f /tmp/mihomo
        _log_ok "$KERNEL_NAME ${ver} installed"
        return 0
    fi
    return 1
}

# ═══════════════════════════════════════════════
#  Function: download yq
# ═══════════════════════════════════════════════
_download_yq() {
    local ver="${VERSION_YQ:-v4.49.2}"
    local arch
    arch="$(_yq_arch)"
    local raw_url="https://github.com/mikefarah/yq/releases/download/${ver}/yq_linux_${arch}"
    if _gh_download "$raw_url" /tmp/yq "yq ${ver}"; then
        _sudo install -D /tmp/yq "${CLASH_BASE_DIR}/bin/yq"
        _sudo chmod 755 "${CLASH_BASE_DIR}/bin/yq"
        rm -f /tmp/yq
        _log_ok "yq ${ver} installed"
        return 0
    fi
    return 1
}

# ═══════════════════════════════════════════════
#  Function: download geodata databases
# ═══════════════════════════════════════════════
_download_geodata() {
    local geover="${VERSION_GEODATA:-20250101}"
    local raw_base="https://github.com/MetaCubeX/meta-rules-dat/releases/download/${geover}"
    local dest="${CLASH_BASE_DIR}/resources"

    for f in Country.mmdb geosite.dat geoip.dat; do
        if [ -f "${dest}/${f}" ]; then
            _log_info "geodata ${f} already exists, skipping"
            continue
        fi
        _gh_download "${raw_base}/${f}" "${dest}/${f}" "${f}" || true
    done
}

_json_escape() {
    printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
}

_write_install_state() {
    local state="${CLASH_BASE_DIR}/install-state.json"
    local tmp
    tmp="$(mktemp)"

    local installed_at
    installed_at="$(date -u +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || date)"

    local clashctl_installed=false
    local clashctl_version=""
    if [ -x /usr/local/bin/clashctl ]; then
        clashctl_installed=true
        clashctl_version="$(/usr/local/bin/clashctl version 2>/dev/null | head -n1 || true)"
    fi

    local tui_installed=false
    if [ -x /usr/local/bin/clash-tui ]; then
        tui_installed=true
    fi

    local mihomo_ver="${VERSION_MIHOMO:-v1.19.17}"
    local mihomo_asset="mihomo-$(_mihomo_arch)-${mihomo_ver}.gz"
    local mihomo_url="https://github.com/MetaCubeX/mihomo/releases/download/${mihomo_ver}/${mihomo_asset}"
    local yq_ver="${VERSION_YQ:-v4.49.2}"
    local yq_url="https://github.com/mikefarah/yq/releases/download/${yq_ver}/yq_linux_$(_yq_arch)"
    local geodata_ver="${VERSION_GEODATA:-20250101}"
    local geodata_url="https://github.com/MetaCubeX/meta-rules-dat/releases/download/${geodata_ver}"

    cat > "$tmp" << STATE
{
  "schema_version": 1,
  "installed_at": "$(_json_escape "$installed_at")",
  "source_dir": "$(_json_escape "$SCRIPT_DIR")",
  "base_dir": "$(_json_escape "$CLASH_BASE_DIR")",
  "service_name": "$(_json_escape "$SERVICE_NAME")",
  "kernel_name": "$(_json_escape "$KERNEL_NAME")",
  "init_type": "$(_json_escape "$INIT_TYPE")",
  "force": $FORCE,
  "reset_config": $RESET_CONFIG,
  "components": {
    "mihomo": {
      "version": "$(_json_escape "$mihomo_ver")",
      "source_url": "$(_json_escape "$mihomo_url")",
      "path": "$(_json_escape "$BIN_KERNEL")"
    },
    "yq": {
      "version": "$(_json_escape "$yq_ver")",
      "source_url": "$(_json_escape "$yq_url")",
      "path": "$(_json_escape "${CLASH_BASE_DIR}/bin/yq")"
    },
    "geodata": {
      "version": "$(_json_escape "$geodata_ver")",
      "source_url": "$(_json_escape "$geodata_url")",
      "path": "$(_json_escape "${CLASH_BASE_DIR}/resources")"
    },
    "clashctl": {
      "installed": $clashctl_installed,
      "version": "$(_json_escape "$clashctl_version")",
      "source_url": "$(_json_escape "$CLASHCTL_SOURCE_URL")",
      "path": "/usr/local/bin/clashctl"
    },
    "clash_tui": {
      "installed": $tui_installed,
      "source_url": "$(_json_escape "$CLASH_TUI_SOURCE_URL")",
      "path": "/usr/local/bin/clash-tui"
    }
  }
}
STATE

    _sudo install -D -m 0644 "$tmp" "$state"
    rm -f "$tmp"
    if [ -n "${SUDO_USER:-}" ] && [ "$SUDO_USER" != "root" ]; then
        _sudo chown "$SUDO_USER:$SUDO_USER" "$state" 2>/dev/null || true
    fi
    _log_ok "install state written: $state"
}

_install_release_artifacts() {
    local install_cli="${1:-true}"
    local install_tui="${2:-false}"
    [ "${CLASHCTL_SKIP_RELEASE:-}" = "true" ] && return 1

    local arch
    if ! arch="$(_release_arch)"; then
        _log_info "no release artifact for architecture $(uname -m), using source build"
        return 1
    fi
    if ! command -v sha256sum >/dev/null 2>&1; then
        _log_info "sha256sum not found, using source build"
        return 1
    fi

    local base="${CLASHCTL_RELEASE_BASE_URL:-https://github.com/yequdesu/clash-for-linux/releases/latest/download}"
    local artifact="clash-for-linux-${arch}.tar.gz"
    local tmpdir
    tmpdir="$(mktemp -d)"

    if ! _gh_download "${base}/${artifact}" "${tmpdir}/${artifact}" "clash-for-linux release ${arch}"; then
        rm -rf "$tmpdir"
        return 1
    fi
    if ! _gh_download "${base}/SHA256SUMS" "${tmpdir}/SHA256SUMS" "release checksums"; then
        rm -rf "$tmpdir"
        _log_warn "release checksum unavailable, using source build"
        return 1
    fi
    if ! ( cd "$tmpdir" && awk -v file="$artifact" '$2 == file {print; found=1} END {exit found ? 0 : 1}' SHA256SUMS > SHA256SUMS.one && sha256sum -c SHA256SUMS.one ); then
        rm -rf "$tmpdir"
        _log_warn "release checksum verification failed, using source build"
        return 1
    fi
    if ! tar -xzf "${tmpdir}/${artifact}" -C "$tmpdir"; then
        rm -rf "$tmpdir"
        _log_warn "release artifact extraction failed, using source build"
        return 1
    fi

    if $install_cli && [ -f "${tmpdir}/clashctl-linux-${arch}" ]; then
        _sudo install -D "${tmpdir}/clashctl-linux-${arch}" /usr/local/bin/clashctl
        CLASHCTL_SOURCE_URL="${base}/${artifact}"
        _log_ok "clashctl installed from release artifact"
    fi
    if $install_tui && [ -f "${tmpdir}/clash-tui-linux-${arch}" ]; then
        _sudo install -D "${tmpdir}/clash-tui-linux-${arch}" /usr/local/bin/clash-tui
        CLASH_TUI_SOURCE_URL="${base}/${artifact}"
        _log_ok "clash-tui installed from release artifact"
    fi

    rm -rf "$tmpdir"
    if { $install_cli && [ -x /usr/local/bin/clashctl ]; } || { $install_tui && [ -x /usr/local/bin/clash-tui ]; }; then
        RELEASE_TOOLS_INSTALLED=true
        return 0
    fi
    return 1
}

# ═══════════════════════════════════════════════
#  Function: install CLI (clashctl)
# ═══════════════════════════════════════════════
_install_cli() {
    if [ -x /usr/local/bin/clashctl ]; then
        _log_info "clashctl already installed"
        return 0
    fi

    if _install_release_artifacts true "$WITH_TUI" && [ -x /usr/local/bin/clashctl ]; then
        return 0
    fi

    if command -v go >/dev/null 2>&1; then
        _log_info "building clashctl..."
        GOPROXY="${GOPROXY:-https://goproxy.cn,direct}" go build -ldflags="-s -w" -o /tmp/clashctl ./cmd/clashctl/ && {
            _sudo install -D /tmp/clashctl /usr/local/bin/clashctl; rm -f /tmp/clashctl
            CLASHCTL_SOURCE_URL="local-source:${SCRIPT_DIR}"
            _log_ok "clashctl installed"
            return 0
        } || { _log_warn "clashctl build failed"; return 1; }
    fi

    _log_info "Go not found — installing Go 1.24.1..."
    local go_arch="linux-amd64"
    case "$(uname -m)" in aarch64|arm64) go_arch="linux-arm64" ;; armv*) go_arch="linux-armv6l" ;; esac
    if curl -fSL --progress-bar "https://go.dev/dl/go1.24.1.${go_arch}.tar.gz" -o /tmp/go.tar.gz; then
        _sudo tar -C /usr/local -xzf /tmp/go.tar.gz; rm -f /tmp/go.tar.gz
        export PATH="/usr/local/go/bin:$PATH"
        _log_ok "Go 1.24.1 installed"
        GOPROXY="${GOPROXY:-https://goproxy.cn,direct}" go build -ldflags="-s -w" -o /tmp/clashctl ./cmd/clashctl/ && {
            _sudo install -D /tmp/clashctl /usr/local/bin/clashctl; rm -f /tmp/clashctl
            CLASHCTL_SOURCE_URL="local-source:${SCRIPT_DIR}"
            _log_ok "clashctl installed"
            return 0
        }
    fi
    _log_warn "clashctl not built — install Go manually"
    return 1
}

# ═══════════════════════════════════════════════
#  Function: install TUI (clash-tui)
# ═══════════════════════════════════════════════
_install_tui() {
    echo ""
    _log_section "Installing TUI Dashboard"

    if [ -x /usr/local/bin/clash-tui ]; then
        _log_info "clash-tui already installed"
        return 0
    fi

    if _install_release_artifacts false true && [ -x /usr/local/bin/clash-tui ]; then
        return 0
    fi

    if ! command -v cargo >/dev/null 2>&1; then
        if command -v rustup >/dev/null 2>&1; then
            _log_info "rustup found, ensuring toolchain..."
            rustup default stable 2>/dev/null || true
        elif curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y 2>/dev/null; then
            source "$REAL_HOME/.cargo/env"
            _log_ok "Rust installed"
        else
            _log_warn "Rust install failed — install rustup manually, then re-run"
            return 1
        fi
    fi

    # Ensure cargo is in PATH
    [ -f "$REAL_HOME/.cargo/env" ] && source "$REAL_HOME/.cargo/env"

    _log_info "building clash-tui (this may take 2-5 minutes)..."
    ( cd "$SCRIPT_DIR/tui" && cargo build --release 2>&1 | tail -5 )
    if [ -f tui/target/release/clash-tui ]; then
        _sudo install -D tui/target/release/clash-tui /usr/local/bin/clash-tui
        CLASH_TUI_SOURCE_URL="local-source:${SCRIPT_DIR}/tui"
        _log_ok "clash-tui installed"
        return 0
    fi
    _log_warn "clash-tui build failed"
    return 1
}

# ═══════════════════════════════════════════════
#  Reinstall check (must be AFTER fn defs)
# ═══════════════════════════════════════════════
if [ -d "$CLASH_BASE_DIR" ] && [ -f "$CLASH_BASE_DIR/bin/yq" ]; then
    if $FORCE; then
        _log_info "force reinstall — preserving user config by default"
        _sudo mkdir -p /usr/local/bin 2>/dev/null || true
        for bin in "$KERNEL_NAME" yq; do
            [ -f "${CLASH_BASE_DIR}/bin/${bin}" ] && _sudo cp "${CLASH_BASE_DIR}/bin/${bin}" "/usr/local/bin/${bin}" 2>/dev/null && _log_info "preserved ${bin}" || true
        done
        if $RESET_CONFIG; then
            _log_warn "resetting resources, logs, and runtime because --reset-config was provided"
            _sudo rm -rf "${CLASH_BASE_DIR}/resources" "${CLASH_BASE_DIR}/logs" "${CLASH_BASE_DIR}/runtime" 2>/dev/null || true
        else
            _sudo rm -rf "${CLASH_BASE_DIR}/runtime" 2>/dev/null || true
        fi
    else
        _log_warn "already installed at $CLASH_BASE_DIR"
        if $WITH_TUI; then
            _install_tui
        fi
        _log_info "use --force to reinstall"
        exit 1
    fi
fi

# Helper: prefer system binary over download
_sync_or_download() {
    local name="$1"; local dst="$2"; shift 2
    if [ -f "$dst" ] && [ -x "$dst" ]; then
        _log_info "${name} already exists, skipping download"
        return 0
    fi
    if [ -f "/usr/local/bin/${name}" ] && [ -x "/usr/local/bin/${name}" ]; then
        if _sudo install -D -m 0755 "/usr/local/bin/${name}" "$dst"; then
            _log_ok "${name} copied from /usr/local/bin/"
            return 0
        fi
        _log_warn "${name} copy from /usr/local/bin failed, downloading fresh"
    fi
    "$@"
}

_copy_resource_if_missing() {
    local src="$1"
    local dst="$2"
    if $RESET_CONFIG || [ ! -e "$dst" ]; then
        /bin/cp -f "$src" "$dst" || _log_fatal "copy resource failed: $src -> $dst"
    fi
}

# ═══════════════════════════════════════════════
#  Main install flow
# ═══════════════════════════════════════════════

# --tui-only: just install TUI and exit
if $TUI_ONLY; then
    _install_tui
    exit $?
fi

# ── Create directories ──
mkdir -p "${CLASH_BASE_DIR}/bin"
mkdir -p "${CLASH_BASE_DIR}/resources/profiles"
mkdir -p "${CLASH_BASE_DIR}/resources/configs"
mkdir -p "${CLASH_BASE_DIR}/logs"
mkdir -p "${CLASH_BASE_DIR}/runtime"

# ── Copy resources ──
_copy_resource_if_missing "$SCRIPT_DIR/resources/mixin.yaml" "$CLASH_BASE_DIR/resources/mixin.yaml"
_copy_resource_if_missing "$SCRIPT_DIR/resources/profiles.yaml" "$CLASH_BASE_DIR/resources/profiles.yaml"
_copy_resource_if_missing "$SCRIPT_DIR/.env" "$CLASH_BASE_DIR/.env"
if $RESET_CONFIG || [ ! -e "${CLASH_BASE_DIR}/resources/config.yaml" ]; then
    : > "${CLASH_BASE_DIR}/resources/config.yaml"
fi

# ── Write install markers ──
mkdir -p "$REAL_HOME/.config/clashctl"
{
    echo "CLASH_BASE_DIR=$CLASH_BASE_DIR"
    echo "SERVICE_NAME=$SERVICE_NAME"
    echo "KERNEL_NAME=$KERNEL_NAME"
    echo "INIT_TYPE=$INIT_TYPE"
} > "$REAL_HOME/.config/clashctl/install.env"
_sudo mkdir -p /etc/clashctl
{
    echo "CLASH_BASE_DIR=$CLASH_BASE_DIR"
    echo "SERVICE_NAME=$SERVICE_NAME"
    echo "KERNEL_NAME=$KERNEL_NAME"
    echo "INIT_TYPE=$INIT_TYPE"
} | _sudo tee /etc/clashctl/install.env >/dev/null

# ── Download resources ──
_sync_or_download "$KERNEL_NAME" "$BIN_KERNEL" _download_kernel
_sync_or_download yq "${CLASH_BASE_DIR}/bin/yq" _download_yq
test -x "$BIN_KERNEL" || _log_fatal "kernel binary missing after install: $BIN_KERNEL"
test -x "${CLASH_BASE_DIR}/bin/yq" || _log_fatal "yq binary missing after install: ${CLASH_BASE_DIR}/bin/yq"
_download_geodata

# ── Set kernel capabilities (for TUN mode) ──
command -v setcap >/dev/null 2>&1 && \
    _sudo setcap cap_net_admin,cap_net_raw,cap_net_bind_service+ep "$BIN_KERNEL" 2>/dev/null && \
    _log_info "TUN capability granted" || true

# ── Install systemd service ──
if [ "$INIT_TYPE" = "systemd" ]; then
    _log_info "installing systemd service..."
    cat > "/tmp/${SERVICE_NAME}.service" << SYSTEMD
[Unit]
Description=Clashctl Proxy Service (Mihomo)
After=network.target
StartLimitIntervalSec=60
StartLimitBurst=3

[Service]
Type=simple
User=$REAL_USER
LimitNOFILE=1000000
CapabilityBoundingSet=CAP_NET_ADMIN CAP_NET_RAW CAP_NET_BIND_SERVICE
AmbientCapabilities=CAP_NET_ADMIN CAP_NET_RAW CAP_NET_BIND_SERVICE
Restart=always
RestartSec=3
ExecStart=$BIN_KERNEL -d ${CLASH_BASE_DIR}/resources -f ${CLASH_BASE_DIR}/resources/runtime.yaml
ExecStop=/bin/kill -SIGTERM \$MAINPID
StandardOutput=append:${CLASH_BASE_DIR}/logs/${KERNEL_NAME}.log
StandardError=append:${CLASH_BASE_DIR}/logs/${KERNEL_NAME}.log

[Install]
WantedBy=multi-user.target
SYSTEMD
    _sudo install -D -m 0644 "/tmp/${SERVICE_NAME}.service" "/etc/systemd/system/${SERVICE_NAME}.service"
    rm -f "/tmp/${SERVICE_NAME}.service"
    _sudo systemctl daemon-reload
    _log_ok "systemd service installed"
fi

# ── Fix ownership ──
if [ -n "${SUDO_USER:-}" ] && [ "$SUDO_USER" != "root" ]; then
    _sudo chown -R "$SUDO_USER:$SUDO_USER" "$CLASH_BASE_DIR" 2>/dev/null || true
fi

# ── PATH setup ──
case ":$PATH:" in *:/usr/local/bin:*) ;; *)
    for rc in "$REAL_HOME/.zshrc" "$REAL_HOME/.bashrc"; do
        [ -f "$rc" ] && grep -q '/usr/local/bin' "$rc" 2>/dev/null || echo 'export PATH="/usr/local/bin:$PATH"' >> "$rc"
    done
    export PATH="/usr/local/bin:$PATH"
esac

# ── Install components ──
$SKIP_CLI || _install_cli
$WITH_TUI && _install_tui

# ── Post-install config ──
if [ -x /usr/local/bin/clashctl ]; then
    CLASH_BASE_DIR="$CLASH_BASE_DIR" SERVICE_NAME="$SERVICE_NAME" KERNEL_NAME="$KERNEL_NAME" INIT_TYPE="$INIT_TYPE" /usr/local/bin/clashctl config merge || _log_fatal "initial config merge failed"
    RANDOM_SECRET="$(od -An -N32 -tx1 /dev/urandom 2>/dev/null | tr -d ' \n')"
    [ -n "$RANDOM_SECRET" ] || RANDOM_SECRET="$(tr -dc 'a-zA-Z0-9' < /dev/urandom 2>/dev/null | head -c32 || echo "clashctl")"
    CLASH_BASE_DIR="$CLASH_BASE_DIR" SERVICE_NAME="$SERVICE_NAME" KERNEL_NAME="$KERNEL_NAME" INIT_TYPE="$INIT_TYPE" /usr/local/bin/clashctl secret "$RANDOM_SECRET" || _log_fatal "initial API secret write failed"
    _log_ok "install complete"

    [ -n "${CLASH_CONFIG_URL:-}" ] && {
        _log_info "downloading subscription..."
        CLASH_BASE_DIR="$CLASH_BASE_DIR" SERVICE_NAME="$SERVICE_NAME" KERNEL_NAME="$KERNEL_NAME" INIT_TYPE="$INIT_TYPE" /usr/local/bin/clashctl sub add "$CLASH_CONFIG_URL" || _log_warn "initial subscription download failed"
    }
    for f in "${CLASH_BASE_DIR}/resources/configs"/*.yaml "${CLASH_BASE_DIR}/resources/configs"/*.yml; do
        [ -f "$f" ] || continue
        CLASH_BASE_DIR="$CLASH_BASE_DIR" SERVICE_NAME="$SERVICE_NAME" KERNEL_NAME="$KERNEL_NAME" INIT_TYPE="$INIT_TYPE" /usr/local/bin/clashctl sub add "file://$f" || _log_warn "initial local subscription import failed: $f"
    done

    _write_install_state

    echo ''
    _log_info "quick start:"
    echo '  clashctl start             start proxy'
    echo '  clashctl sub add <url>     add subscription'
    echo '  eval $(clashctl env)       load proxy env'
    echo '  clashctl tui               launch TUI dashboard'
    [ -x /usr/local/bin/clash-tui ] && echo '  clash-tui                  launch TUI directly'
else
    _write_install_state
    _log_warn "clashctl not installed — build manually with Go: bash install.sh"
fi
