#!/usr/bin/env bash
# Cross-distro install smoke test.
#
# This intentionally avoids network downloads for Mihomo/yq/geodata by seeding
# deterministic fake binaries and data files. It validates installer idempotency,
# install-state generation, CLI config commands, doctor behavior, and uninstall.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DISTRO="${CLASHCTL_SMOKE_DISTRO:-unknown}"
CLASH_BASE_DIR="${CLASH_BASE_DIR:-/tmp/clashctl-smoke-${DISTRO}}"
export CLASH_BASE_DIR
export KERNEL_NAME="${KERNEL_NAME:-mihomo}"
export SERVICE_NAME="${SERVICE_NAME:-clashctl}"
export CLASHCTL_SKIP_RELEASE=true
export VERSION_GEODATA="${VERSION_GEODATA:-smoke}"

log() { printf '[smoke:%s] %s\n' "$DISTRO" "$*"; }
fail() { printf '[smoke:%s] ERROR: %s\n' "$DISTRO" "$*" >&2; exit 1; }

require_cmd() {
    command -v "$1" >/dev/null 2>&1 || fail "missing command: $1"
}

seed_fake_kernel() {
    install -d "$CLASH_BASE_DIR/bin"
    cat > "$CLASH_BASE_DIR/bin/${KERNEL_NAME}" <<'KERNEL'
#!/usr/bin/env bash
for arg in "$@"; do
    if [ "$arg" = "-t" ]; then
        exit 0
    fi
done
trap 'exit 0' TERM INT
while :; do sleep 60; done
KERNEL
    chmod 755 "$CLASH_BASE_DIR/bin/${KERNEL_NAME}"
}

seed_fake_yq() {
    install -d "$CLASH_BASE_DIR/bin"
    cat > "$CLASH_BASE_DIR/bin/yq" <<'YQ'
#!/usr/bin/env bash
set -euo pipefail
if [ "${1:-}" = "-i" ]; then
    expr="${2:-}"
    file="${3:-}"
    if [ -z "$file" ]; then
        exit 1
    fi
    if [[ "$expr" == *".secret"* ]]; then
        secret="${CLASHCTL_SECRET:-}"
        if grep -q '^secret:' "$file"; then
            sed -i "s|^secret:.*|secret: \"${secret}\"|" "$file"
        else
            printf '\nsecret: "%s"\n' "$secret" >> "$file"
        fi
    fi
    exit 0
fi
if [ "${1:-}" = "eval-all" ]; then
    mixin="${@: -1}"
    cat "$mixin"
    exit 0
fi
if [ "${1:-}" = "eval" ]; then
    exit 0
fi
echo "fake yq unsupported args: $*" >&2
exit 1
YQ
    chmod 755 "$CLASH_BASE_DIR/bin/yq"
}

seed_fake_resources() {
    install -d "$CLASH_BASE_DIR/resources" "$CLASH_BASE_DIR/bin/subconverter"
    printf 'smoke\n' > "$CLASH_BASE_DIR/resources/Country.mmdb"
    printf 'smoke\n' > "$CLASH_BASE_DIR/resources/geosite.dat"
    printf 'smoke\n' > "$CLASH_BASE_DIR/resources/geoip.dat"
    cat > "$CLASH_BASE_DIR/bin/subconverter/subconverter" <<'SUB'
#!/usr/bin/env bash
trap 'exit 0' TERM INT
while :; do sleep 60; done
SUB
    chmod 755 "$CLASH_BASE_DIR/bin/subconverter/subconverter"
}

run_doctor_allow_warnings() {
    set +e
    "$@"
    code=$?
    set -e
    if [ "$code" -eq 2 ]; then
        fail "fatal doctor result from: $*"
    fi
}

require_cmd go
require_cmd bash
require_cmd install

log "building local clashctl"
go build -trimpath -buildvcs=false -o /usr/local/bin/clashctl ./cmd/clashctl

log "seeding fake kernel/yq/geodata"
rm -rf "$CLASH_BASE_DIR"
seed_fake_kernel
seed_fake_yq
seed_fake_resources

log "running install.sh"
bash "$ROOT_DIR/install.sh" --force --skip-cli

test -x /usr/local/bin/clashctl || fail "clashctl was not installed"
test -f "$CLASH_BASE_DIR/install-state.json" || fail "install-state.json missing"
test -f "$CLASH_BASE_DIR/resources/runtime.yaml" || fail "runtime.yaml missing"

log "checking CLI commands"
/usr/local/bin/clashctl version
/usr/local/bin/clashctl status >/tmp/clashctl-smoke-status.txt
/usr/local/bin/clashctl config set-port 7897 --http 7898 --socks 7899
/usr/local/bin/clashctl config set-api 127.0.0.1:19090 --secret smoke-secret
/usr/local/bin/clashctl config set-dns-mode fake-ip
/usr/local/bin/clashctl config set-lan off
/usr/local/bin/clashctl proxy on >/tmp/clashctl-smoke-proxy-on.txt
grep -q 'http_proxy' /tmp/clashctl-smoke-proxy-on.txt || fail "proxy on did not print shell exports"
/usr/local/bin/clashctl proxy desktop status >/tmp/clashctl-smoke-desktop-status.txt

log "running doctor checks"
run_doctor_allow_warnings /usr/local/bin/clashctl doctor
run_doctor_allow_warnings /usr/local/bin/clashctl config doctor

log "running uninstall.sh"
printf 'n\nn\n' | bash "$ROOT_DIR/uninstall.sh"

test ! -d "$CLASH_BASE_DIR" || fail "base dir was not removed"
test ! -e /usr/local/bin/clashctl || fail "clashctl binary was not removed"

log "smoke test passed"
