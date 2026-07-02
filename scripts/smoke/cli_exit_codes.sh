#!/usr/bin/env bash
# CLI process exit-code smoke test.
#
# This builds clashctl and verifies representative success/failure paths as a
# real process, including paths that call os.Exit through ilog.Fatal.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

log() { printf '[cli-exit-codes] %s\n' "$*"; }
fail() { printf '[cli-exit-codes] ERROR: %s\n' "$*" >&2; exit 1; }

require_cmd() {
    command -v "$1" >/dev/null 2>&1 || fail "missing command: $1"
}

expect_exit() {
    local want="$1"
    shift
    set +e
    "$@" >"$WORK_DIR/stdout" 2>"$WORK_DIR/stderr"
    local got=$?
    set -e
    if [ "$got" -ne "$want" ]; then
        printf '[cli-exit-codes] command: %s\n' "$*" >&2
        printf '[cli-exit-codes] exit: got %s want %s\n' "$got" "$want" >&2
        printf '[cli-exit-codes] stdout:\n' >&2
        cat "$WORK_DIR/stdout" >&2
        printf '[cli-exit-codes] stderr:\n' >&2
        cat "$WORK_DIR/stderr" >&2
        exit 1
    fi
}

seed_base() {
    rm -rf "$CLASH_BASE_DIR"
    mkdir -p "$CLASH_BASE_DIR/resources" "$CLASH_BASE_DIR/logs"
    cat > "$CLASH_BASE_DIR/resources/runtime.yaml" <<'YAML'
mixed-port: 7890
external-controller: 127.0.0.1:9090
YAML
}

require_cmd go

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

BIN="$WORK_DIR/clashctl"
CLASH_BASE_DIR="$WORK_DIR/install"
export CLASH_BASE_DIR
export KERNEL_NAME="${KERNEL_NAME:-mihomo}"
export SERVICE_NAME="${SERVICE_NAME:-clashctl}"
export INIT_TYPE="${INIT_TYPE:-nohup}"

log "building clashctl"
go build -trimpath -o "$BIN" ./cmd/clashctl

log "checking parent commands"
seed_base
expect_exit 1 "$BIN"
expect_exit 1 "$BIN" config
expect_exit 1 "$BIN" sub
expect_exit 1 "$BIN" node
expect_exit 1 "$BIN" geodata

log "checking successful empty-state/status commands"
seed_base
expect_exit 0 "$BIN" sub list
expect_exit 0 "$BIN" sub log
expect_exit 0 "$BIN" proxy on
expect_exit 0 "$BIN" proxy off

log "checking invalid and failed commands"
seed_base
expect_exit 1 "$BIN" proxy invalid
expect_exit 1 "$BIN" secret show
expect_exit 1 "$BIN" sub update
expect_exit 1 "$BIN" sub update not-a-number

log "checking profile read failure"
seed_base
printf 'profiles: [\n' > "$CLASH_BASE_DIR/resources/profiles.yaml"
expect_exit 1 "$BIN" sub list

log "ok"
