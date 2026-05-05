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
FORCE=false; WITH_TUI=false; SKIP_CLI=false; TUI_ONLY=false
for arg in "$@"; do
    case "$arg" in
        --force)    FORCE=true ;;
        --with-tui|--tui) WITH_TUI=true ;;
        --skip-cli) SKIP_CLI=true ;;
        --tui-only) TUI_ONLY=true; SKIP_CLI=true ;;
        --help|-h)
            echo "Usage: bash install.sh [flags]"
            echo ""
            echo "Flags:"
            echo "  --with-tui (or --tui)    Also build and install the TUI dashboard"
            echo "  --tui-only               Only install the TUI (skip CLI)"
            echo "  --force                  Force reinstall, overwrite existing"
            echo "  --skip-cli               Skip CLI build (use pre-built or skip)"
            echo ""
            echo "Run as normal user. Password asked once at the beginning."
            exit 0
            ;;
    esac
done

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
BIN_KERNEL="${CLASH_BASE_DIR}/bin/mihomo"
KERNEL_NAME="${KERNEL_NAME:-mihomo}"

# ── .env load ──
[ -f "$SCRIPT_DIR/.env" ] && . "$SCRIPT_DIR/.env" 2>/dev/null || true
KERNEL_NAME="${KERNEL_NAME:-mihomo}"
INIT_TYPE="${INIT_TYPE:-}"

# ── Detect init system ──
detect_init() {
    [ -n "$INIT_TYPE" ] && return
    if [ -f /run/systemd/system ] || grep -q systemd /proc/1/exe 2>/dev/null; then
        INIT_TYPE="systemd"
    elif grep -q 'docker\|kubepods\|containerd' /proc/1/cgroup 2>/dev/null; then
        INIT_TYPE="nohup"
    else
        INIT_TYPE="systemd"
    fi
}
detect_init

_log_info "kernel: $KERNEL_NAME  |  init: $INIT_TYPE  |  install: $CLASH_BASE_DIR"

# ═══════════════════════════════
#  Kill stale processes
# ═══════════════════════════════
for name in mihomo clash; do
    pgrep -x "$name" >/dev/null 2>&1 && { _sudo pkill -9 -x "$name" 2>/dev/null || true; sleep 0.5; }
done

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
_download_kernel() {
    local ver="${VERSION_MIHOMO:-v1.19.17}"
    local arch="linux-amd64"
    case "$(uname -m)" in
        aarch64|arm64) arch="linux-arm64" ;;
        armv7l)        arch="linux-armv7" ;;
    esac
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
    local arch="amd64"
    case "$(uname -m)" in aarch64|arm64) arch="arm64" ;; esac
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

# ═══════════════════════════════════════════════
#  Function: install CLI (clashctl)
# ═══════════════════════════════════════════════
_install_cli() {
    if [ -x /usr/local/bin/clashctl ]; then
        _log_info "clashctl already installed"
        return 0
    fi

    if command -v go >/dev/null 2>&1; then
        _log_info "building clashctl..."
        GOPROXY="${GOPROXY:-https://goproxy.cn,direct}" go build -ldflags="-s -w" -o /tmp/clashctl ./cmd/clashctl/ && {
            _sudo install -D /tmp/clashctl /usr/local/bin/clashctl; rm -f /tmp/clashctl
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
        _log_info "force reinstall — preserving binaries, refreshing config..."
        _sudo mkdir -p /usr/local/bin 2>/dev/null || true
        for bin in mihomo yq; do
            [ -f "${CLASH_BASE_DIR}/bin/${bin}" ] && _sudo cp "${CLASH_BASE_DIR}/bin/${bin}" "/usr/local/bin/${bin}" 2>/dev/null && _log_info "preserved ${bin}" || true
        done
        _sudo rm -rf "${CLASH_BASE_DIR}/resources" "${CLASH_BASE_DIR}/logs" "${CLASH_BASE_DIR}/runtime" 2>/dev/null || true
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
        _sudo cp "/usr/local/bin/${name}" "$dst" 2>/dev/null || true
        _sudo chmod 755 "$dst" 2>/dev/null || true
        _log_ok "${name} copied from /usr/local/bin/"
        return 0
    fi
    "$@"
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
/bin/cp -rf "$SCRIPT_DIR/resources/." "$CLASH_BASE_DIR/resources/" 2>/dev/null || true
/bin/cp -f "$SCRIPT_DIR/.env" "$CLASH_BASE_DIR/.env" 2>/dev/null || true
touch "${CLASH_BASE_DIR}/resources/config.yaml"

# ── Write install markers ──
mkdir -p "$REAL_HOME/.config/clashctl"
echo "CLASH_BASE_DIR=$CLASH_BASE_DIR" > "$REAL_HOME/.config/clashctl/install.env"
_sudo mkdir -p /etc/clashctl 2>/dev/null
echo "CLASH_BASE_DIR=$CLASH_BASE_DIR" | _sudo tee /etc/clashctl/install.env >/dev/null 2>&1

# ── Download resources ──
_sync_or_download "$KERNEL_NAME" "$BIN_KERNEL" _download_kernel
_sync_or_download yq "${CLASH_BASE_DIR}/bin/yq" _download_yq
_download_geodata

# ── Set kernel capabilities (for TUN mode) ──
command -v setcap >/dev/null 2>&1 && \
    _sudo setcap cap_net_admin,cap_net_raw+ep "$BIN_KERNEL" 2>/dev/null && \
    _log_info "TUN capability granted" || true

# ── Install systemd service ──
if [ "$INIT_TYPE" = "systemd" ]; then
    _log_info "installing systemd service..."
    cat > /tmp/clashctl.service << SYSTEMD
[Unit]
Description=Clashctl Proxy Service (Mihomo)
After=network.target

[Service]
Type=simple
User=$REAL_USER
LimitNPROC=500
LimitNOFILE=1000000
CapabilityBoundingSet=CAP_NET_ADMIN CAP_NET_RAW CAP_NET_BIND_SERVICE
AmbientCapabilities=CAP_NET_ADMIN CAP_NET_RAW CAP_NET_BIND_SERVICE
Restart=always
RestartSec=3
ExecStart=$BIN_KERNEL -d ${CLASH_BASE_DIR}/resources -f ${CLASH_BASE_DIR}/resources/runtime.yaml
ExecStop=/bin/kill -SIGTERM \$MAINPID
StandardOutput=append:${CLASH_BASE_DIR}/logs/mihomo.log
StandardError=append:${CLASH_BASE_DIR}/logs/mihomo.log

[Install]
WantedBy=multi-user.target
SYSTEMD
    _sudo mv /tmp/clashctl.service /etc/systemd/system/clashctl.service 2>/dev/null || true
    _sudo systemctl daemon-reload 2>/dev/null || true
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
    /usr/local/bin/clashctl config merge 2>/dev/null || true
    RANDOM_SECRET=$(tr -dc 'a-zA-Z0-9' < /dev/urandom 2>/dev/null | head -c8 || echo "clashctl")
    /usr/local/bin/clashctl secret "$RANDOM_SECRET" 2>/dev/null || true
    _log_ok "install complete"

    [ -n "${CLASH_CONFIG_URL:-}" ] && {
        _log_info "downloading subscription..."
        /usr/local/bin/clashctl sub add "$CLASH_CONFIG_URL" 2>/dev/null || true
    }
    for f in "${CLASH_BASE_DIR}/resources/configs"/*.yaml "${CLASH_BASE_DIR}/resources/configs"/*.yml; do
        [ -f "$f" ] || continue
        /usr/local/bin/clashctl sub add "file://$f" 2>/dev/null || true
    done

    echo ''
    _log_info "quick start:"
    echo '  clashctl start             start proxy'
    echo '  clashctl sub add <url>     add subscription'
    echo '  eval $(clashctl env)       load proxy env'
    echo '  clashctl tui               launch TUI dashboard'
    [ -x /usr/local/bin/clash-tui ] && echo '  clash-tui                  launch TUI directly'
else
    _log_warn "clashctl not installed — build manually with Go: bash install.sh"
fi
