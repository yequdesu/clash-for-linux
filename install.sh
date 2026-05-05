#!/usr/bin/env bash
# Linux CLI&TUI Clash — installer
# Usage: bash install.sh [--force] [--with-tui] [--skip-cli] [--tui-only]

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"; cd "$SCRIPT_DIR"

stty sane 2>/dev/null || true

_log_fatal() { printf '\r\n\033[31m[x]\033[0m %s\r\n\r\n' "$*" >&2; exit 1; }
_log_warn()  { printf '\r\033[33m[!]\033[0m %s\r\n' "$*" >&2; }
_log_info()  { printf '\r\033[36m[i]\033[0m %s\r\n' "$*"; }
_log_ok()    { printf '\r\033[32m[+]\033[0m %s\r\n' "$*"; }
_log_section(){ printf '\r\n\033[1m\033[36m=== %s ===\033[0m\r\n\r\n' "$1"; }

_sudo()   { [ "$(id -u)" -eq 0 ] && "$@" || sudo "$@"; }

FORCE=false; WITH_TUI=false; SKIP_CLI=false; TUI_ONLY=false
for arg in "$@"; do
    case "$arg" in
        --force)    FORCE=true ;;
        --with-tui) WITH_TUI=true ;;
        --skip-cli) SKIP_CLI=true ;;
        --tui-only) TUI_ONLY=true; SKIP_CLI=true ;;
        --help|-h)  echo "Usage: bash install.sh [--force] [--with-tui] [--skip-cli] [--tui-only]"; exit 0 ;;
    esac
done

[ -n "${SUDO_USER:-}" ] && [ "$SUDO_USER" != "root" ] && exec su "$SUDO_USER" -c "cd '$SCRIPT_DIR' && bash install.sh $*"

for rc in "$HOME/.zshrc" "$HOME/.bashrc"; do
    [ -f "$rc" ] && sed -i '/# clashctl START/,/# clashctl END/d' "$rc" 2>/dev/null || true
done
rm -f "$HOME/.config/fish/conf.d/clashctl.fish" 2>/dev/null || true

for name in mihomo clash; do
    pgrep -x "$name" >/dev/null 2>&1 && { _sudo pkill -9 -x "$name" 2>/dev/null || true; sleep 0.5; }
done

# ── resolve base dir ──
CLASH_BASE_DIR="${CLASH_BASE_DIR:-$HOME/clashctl}"
BIN_KERNEL="${CLASH_BASE_DIR}/bin/mihomo"
KERNEL_NAME="${KERNEL_NAME:-mihomo}"

# ── .env load ──
[ -f "$SCRIPT_DIR/.env" ] && . "$SCRIPT_DIR/.env" 2>/dev/null
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

# ═══════════════════════════════════════════════
#  CLI install
# ═══════════════════════════════════════════════

_install_cli() {
    if [ -f bin/clashctl ]; then
        _sudo install -D bin/clashctl /usr/local/bin/clashctl
        _log_ok "clashctl installed (pre-built)"
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
    case "$(uname -m)" in aarch64) go_arch="linux-arm64" ;; armv*) go_arch="linux-armv6l" ;; esac
    if curl -fsSL "https://golang.google.cn/dl/go1.24.1.${go_arch}.tar.gz" -o /tmp/go.tar.gz; then
        _sudo tar -C /usr/local -xzf /tmp/go.tar.gz; rm -f /tmp/go.tar.gz
        export PATH="/usr/local/go/bin:$PATH"
        _log_ok "Go installed"
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
#  TUI install
# ═══════════════════════════════════════════════

_install_tui() {
    if [ -f bin/clash-tui ]; then
        _sudo install -D bin/clash-tui /usr/local/bin/clash-tui
        _log_ok "clash-tui installed (pre-built)"
        return 0
    fi

    if [ -x /usr/local/bin/clash-tui ]; then
        _log_info "clash-tui already installed"
        return 0
    fi

    if ! command -v cargo >/dev/null 2>&1; then
        if command -v rustup >/dev/null 2>&1; then
            :
        elif curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y 2>/dev/null; then
            source "$HOME/.cargo/env"
            _log_ok "Rust installed"
        else
            _log_warn "Rust install failed — TUI skipped"
            return 1
        fi
    fi

    _log_info "building clash-tui..."
    ( cd "$SCRIPT_DIR/tui" && cargo build --release 2>&1 | tail -3 )
    if [ -f tui/target/release/clash-tui ]; then
        _sudo install -D tui/target/release/clash-tui /usr/local/bin/clash-tui
        _log_ok "clash-tui installed"
        return 0
    fi
    _log_warn "clash-tui build failed"
    return 1
}

# ═══════════════════════════════════════════════
#  Main
# ═══════════════════════════════════════════════

if $TUI_ONLY; then
    _install_tui
    exit $?
fi

if [ -d "$CLASH_BASE_DIR" ] && [ -f "$CLASH_BASE_DIR/bin/yq" ]; then
    if $FORCE; then
        _log_info "force reinstall — removing previous installation"
        _sudo rm -rf "$CLASH_BASE_DIR" 2>/dev/null || true
    else
        _log_warn "already installed at $CLASH_BASE_DIR"
        if $WITH_TUI; then _install_tui; fi
        _log_info "use --force to reinstall CLI"
        _log_info "or: bash uninstall.sh && bash install.sh"
        exit 1
    fi
fi

_log_section "Linux CLI&TUI Clash"
_log_info "kernel: $KERNEL_NAME  |  init: $INIT_TYPE"
_log_info "install: $CLASH_BASE_DIR"

mkdir -p "${CLASH_BASE_DIR}/bin"
mkdir -p "${CLASH_BASE_DIR}/resources/profiles"
mkdir -p "${CLASH_BASE_DIR}/resources/configs"
mkdir -p "${CLASH_BASE_DIR}/logs"
mkdir -p "${CLASH_BASE_DIR}/runtime"

/bin/cp -rf "$SCRIPT_DIR/resources/." "$CLASH_BASE_DIR/resources/" 2>/dev/null || true
/bin/cp -f "$SCRIPT_DIR/.env" "$CLASH_BASE_DIR/.env" 2>/dev/null || true
touch "${CLASH_BASE_DIR}/resources/config.yaml"

mkdir -p "$HOME/.config/clashctl"
echo "CLASH_BASE_DIR=$CLASH_BASE_DIR" > "$HOME/.config/clashctl/install.env"
_sudo mkdir -p /etc/clashctl 2>/dev/null
echo "CLASH_BASE_DIR=$CLASH_BASE_DIR" | _sudo tee /etc/clashctl/install.env >/dev/null 2>&1

# ── Download kernel & resources ──
download_kernel_binary() {
    local ver="${VERSION_MIHOMO:-v1.19.17}"
    local arch="linux-amd64"
    case "$(uname -m)" in
        aarch64|arm64) arch="linux-arm64" ;;
        armv7l) arch="linux-armv7" ;;
    esac
    local base_url="${URL_GH_PROXY:-https://gh-proxy.org}/https://github.com/MetaCubeX/mihomo/releases/download/${ver}"
    local filename="mihomo-${arch}-${ver}.gz"
    _log_info "downloading $KERNEL_NAME ${ver}..."
    if curl -fsSL "${base_url}/${filename}" -o /tmp/mihomo.gz; then
        gunzip -f /tmp/mihomo.gz
        _sudo install -D /tmp/mihomo "$BIN_KERNEL"
        _sudo chmod 755 "$BIN_KERNEL"
        rm -f /tmp/mihomo
        _log_ok "$KERNEL_NAME ${ver} installed"
        return 0
    fi
    _log_warn "kernel download failed — please install manually"
    return 1
}

download_yq() {
    local ver="${VERSION_YQ:-v4.49.2}"
    local arch="amd64"
    case "$(uname -m)" in aarch64|arm64) arch="arm64" ;; esac
    local url="https://github.com/mikefarah/yq/releases/download/${ver}/yq_linux_${arch}"
    _log_info "downloading yq ${ver}..."
    if curl -fsSL "$url" -o /tmp/yq; then
        _sudo install -D /tmp/yq "${CLASH_BASE_DIR}/bin/yq"
        _sudo chmod 755 "${CLASH_BASE_DIR}/bin/yq"
        rm -f /tmp/yq
        _log_ok "yq ${ver} installed"
        return 0
    fi
    _log_warn "yq download failed"
    return 1
}

if [ ! -f "$BIN_KERNEL" ]; then
    download_kernel_binary
fi

if [ ! -f "${CLASH_BASE_DIR}/bin/yq" ]; then
    download_yq
fi

# ── set capabilities ──
command -v setcap >/dev/null 2>&1 && \
    _sudo setcap cap_net_admin,cap_net_raw+ep "$BIN_KERNEL" 2>/dev/null && \
    _log_info "Tun capability granted" || true

# ── install systemd service ──
if [ "$INIT_TYPE" = "systemd" ]; then
    _log_info "installing systemd service..."
    cat > /tmp/clashctl.service << SYSTEMD
[Unit]
Description=Clashctl Proxy Service (Mihomo)
After=network.target

[Service]
Type=simple
User=$USER
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

# ── PATH setup ──
case ":$PATH:" in *:/usr/local/bin:*) ;; *)
    for rc in "$HOME/.zshrc" "$HOME/.bashrc"; do
        [ -f "$rc" ] && grep -q '/usr/local/bin' "$rc" 2>/dev/null || echo 'export PATH="/usr/local/bin:$PATH"' >> "$rc"
    done
    export PATH="/usr/local/bin:$PATH"
esac

# ── install components ──
$SKIP_CLI || _install_cli
$WITH_TUI && _install_tui

# ── post-install ──
if [ -x /usr/local/bin/clashctl ]; then
    /usr/local/bin/clashctl config merge 2>/dev/null || true
    RANDOM_SECRET=$(tr -dc 'a-zA-Z0-9' < /dev/urandom 2>/dev/null | head -c8 || echo "clashctl")
    /usr/local/bin/clashctl secret "$RANDOM_SECRET" 2>/dev/null || true
    _log_ok "install complete"

    # Auto-import subscriptions
    [ -n "${CLASH_CONFIG_URL:-}" ] && { _log_info "downloading subscription..."; /usr/local/bin/clashctl sub add "$CLASH_CONFIG_URL" 2>/dev/null; }
    for f in "${CLASH_BASE_DIR}/resources/configs"/*.yaml "${CLASH_BASE_DIR}/resources/configs"/*.yml; do
        [ -f "$f" ] || continue
        /usr/local/bin/clashctl sub add "file://$f" 2>/dev/null
    done

    echo ''
    _log_info "usage:"
    echo '  clashctl start             start proxy'
    echo '  clashctl sub add <url>     add subscription'
    echo '  eval $(clashctl env)       load proxy env'
    echo '  clashctl tui               launch dashboard'
    [ -x /usr/local/bin/clash-tui ] && echo '  clash-tui                  launch TUI directly'
else
    _log_warn "clashctl not installed — run with Go: bash install.sh"
fi
