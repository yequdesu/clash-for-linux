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

_gh_download() {
    local raw_url="$1"
    local output="$2"
    local desc="$3"
    _log_info "downloading ${desc}..."
    if curl -fSL --progress-bar "$raw_url" -o "$output" 2>/dev/null; then
        return 0
    fi
    _log_info "direct GitHub failed, trying gh-proxy.org..."
    curl -fSL --progress-bar "https://gh-proxy.org/${raw_url}" -o "$output"
}

_release_arch() {
    case "$(uname -m)" in
        x86_64|amd64) printf 'amd64' ;;
        aarch64|arm64) printf 'arm64' ;;
        *) return 1 ;;
    esac
}

_install_release_tui() {
    [ "${CLASHCTL_SKIP_RELEASE:-}" = "true" ] && return 1
    if ! command -v sha256sum >/dev/null 2>&1; then
        _log_info "sha256sum not found, using source build"
        return 1
    fi

    local arch
    if ! arch="$(_release_arch)"; then
        _log_info "no release artifact for architecture $(uname -m), using source build"
        return 1
    fi

    local base="${CLASHCTL_RELEASE_BASE_URL:-https://github.com/yequdesu/clash-for-linux/releases/latest/download}"
    local artifact="clash-for-linux-${arch}.tar.gz"
    local tmpdir
    tmpdir="$(mktemp -d)"

    if ! _gh_download "${base}/${artifact}" "${tmpdir}/${artifact}" "clash-tui release ${arch}"; then
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
    if [ ! -f "${tmpdir}/clash-tui-linux-${arch}" ]; then
        rm -rf "$tmpdir"
        _log_warn "release artifact does not contain clash-tui, using source build"
        return 1
    fi

    _sudo install -D "${tmpdir}/clash-tui-linux-${arch}" /usr/local/bin/clash-tui
    rm -rf "$tmpdir"
    _log_ok "clash-tui installed from release artifact"
    return 0
}

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

if _install_release_tui; then
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
