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
_log_fatal(){ printf '\r\n\033[31m[x]\033[0m %s\r\n\r\n' "$*" >&2; exit 1; }
_sudo()     { [ "$(id -u)" -eq 0 ] && "$@" || sudo "$@"; }

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

_install_release_artifacts() {
    local install_tui="$1"
    [ "${CLASHCTL_SKIP_RELEASE:-}" = "true" ] && return 1
    if ! command -v sha256sum >/dev/null 2>&1; then
        _log_info "sha256sum not found — falling back to local rebuild"
        return 1
    fi

    local arch
    if ! arch="$(_release_arch)"; then
        _log_info "no release artifact for architecture $(uname -m) — falling back to local rebuild"
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
        _log_warn "release checksum unavailable — falling back to local rebuild"
        return 1
    fi
    if ! ( cd "$tmpdir" && awk -v file="$artifact" '$2 == file {print; found=1} END {exit found ? 0 : 1}' SHA256SUMS > SHA256SUMS.one && sha256sum -c SHA256SUMS.one ); then
        rm -rf "$tmpdir"
        _log_warn "release checksum verification failed — falling back to local rebuild"
        return 1
    fi
    if ! tar -xzf "${tmpdir}/${artifact}" -C "$tmpdir"; then
        rm -rf "$tmpdir"
        _log_warn "release artifact extraction failed — falling back to local rebuild"
        return 1
    fi

    if [ -f "${tmpdir}/clashctl-linux-${arch}" ]; then
        _sudo install -D "${tmpdir}/clashctl-linux-${arch}" /usr/local/bin/clashctl
        _log_ok "clashctl updated from release artifact"
    else
        rm -rf "$tmpdir"
        return 1
    fi
    if $install_tui && [ -f "${tmpdir}/clash-tui-linux-${arch}" ]; then
        _sudo install -D "${tmpdir}/clash-tui-linux-${arch}" /usr/local/bin/clash-tui
        _log_ok "clash-tui updated from release artifact"
    fi

    rm -rf "$tmpdir"
    return 0
}

# ── Pre-flight: cache sudo ──
if [ "$(id -u)" -ne 0 ]; then
    sudo -v || _log_fatal "sudo authentication failed; cannot update /usr/local/bin binaries"
    ( while true; do sudo -v; sleep 60; done ) &
    SUDO_KEEPER=$!
    trap 'kill $SUDO_KEEPER 2>/dev/null' EXIT
fi

CLASHCTL_LIFECYCLE_INIT_TYPE="${INIT_TYPE:-}"
if [ -z "$CLASHCTL_LIFECYCLE_INIT_TYPE" ] && [ -r /etc/clashctl/install.env ]; then
    CLASHCTL_LIFECYCLE_INIT_TYPE="$(grep -E '^INIT_TYPE=' /etc/clashctl/install.env 2>/dev/null | tail -n1 | cut -d= -f2-)"
fi

_clashctl_lifecycle() {
    local clashctl_bin
    clashctl_bin="$(command -v clashctl 2>/dev/null || true)"
    [ -n "$clashctl_bin" ] || _log_fatal "clashctl not found"
    if [ "$CLASHCTL_LIFECYCLE_INIT_TYPE" = "systemd" ]; then
        _sudo "$clashctl_bin" "$@"
    else
        "$clashctl_bin" "$@"
    fi
}

WAS_RUNNING=false
if command -v clashctl >/dev/null 2>&1; then
    if _clashctl_lifecycle status 2>/dev/null | grep -q 'kernel: running'; then
        WAS_RUNNING=true
    fi
    if $WAS_RUNNING; then
        _log_info "stopping kernel..."
        _clashctl_lifecycle stop || _log_fatal "failed to stop running kernel before update"
    else
        _log_info "kernel is not running; update will not auto-start it"
    fi
fi

# ── Git pull ──
if [ -d .git ] && git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    _log_info "pulling updates..."
    git pull 2>/dev/null || _log_warn "git pull failed — continuing with local source"
else
    _log_info "not a git repository — rebuilding from local source"
fi

WANT_TUI=false
[ -x /usr/local/bin/clash-tui ] && WANT_TUI=true

if ! _install_release_artifacts "$WANT_TUI"; then
    # ── Always rebuild CLI ──
    if command -v go >/dev/null 2>&1; then
        _log_info "rebuilding clashctl..."
        GOPROXY="${GOPROXY:-https://goproxy.cn,direct}" go build -ldflags="-s -w" -o /tmp/clashctl ./cmd/clashctl/ || _log_fatal "clashctl build failed"
        _sudo install -D /tmp/clashctl /usr/local/bin/clashctl || _log_fatal "failed to install clashctl to /usr/local/bin"
        rm -f /tmp/clashctl
        _log_ok "clashctl updated"
    else
        _log_fatal "Go not found and no valid release artifact was installed"
    fi

    # ── Rebuild TUI only when it is installed ──
    if $WANT_TUI; then
        if command -v cargo >/dev/null 2>&1 || command -v rustup >/dev/null 2>&1; then
            if [ -f "$HOME/.cargo/env" ]; then source "$HOME/.cargo/env"; fi
            if command -v cargo >/dev/null 2>&1; then
                _log_info "rebuilding clash-tui..."
                ( cd tui && cargo build --release 2>&1 | tail -3 )
                if [ -f tui/target/release/clash-tui ]; then
                    _sudo install -D tui/target/release/clash-tui /usr/local/bin/clash-tui || _log_fatal "failed to install clash-tui to /usr/local/bin"
                    _log_ok "clash-tui updated"
                else
                    _log_warn "clash-tui build failed"
                fi
            fi
        else
            _log_info "cargo not found — skipping TUI rebuild (use 'bash install_tui.sh')"
        fi
    fi
fi

# ── Restart with updated binary ──
if command -v clashctl >/dev/null 2>&1; then
    if $WAS_RUNNING; then
        _clashctl_lifecycle start || _log_fatal "updated clashctl installed, but kernel restart failed"
    fi
    if [ "${CLASHCTL_AUTO_UPGRADE_KERNEL:-false}" = "true" ]; then
        _log_info "checking kernel update..."
        _clashctl_lifecycle upgrade-kernel || _log_fatal "kernel upgrade failed"
    else
        _log_info "kernel binary upgrade skipped; run 'clashctl upgrade-kernel' explicitly if needed"
    fi
else
    _log_fatal "clashctl not found after update"
fi

_log_ok "update complete"
