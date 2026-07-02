#!/usr/bin/env bash
# Behavior smoke for nohup fallback PID ownership checks.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORK_DIR="$(mktemp -d)"
export CLASH_BASE_DIR="$WORK_DIR/clashctl"
export KERNEL_NAME="sleep"

MANAGED_PID=""
UNMANAGED_PID=""

log() { printf '[nohup-pid-safety] %s\n' "$*"; }
fail() { printf '[nohup-pid-safety] ERROR: %s\n' "$*" >&2; exit 1; }

cleanup() {
    set +e
    [ -n "$MANAGED_PID" ] && kill "$MANAGED_PID" 2>/dev/null
    [ -n "$UNMANAGED_PID" ] && kill "$UNMANAGED_PID" 2>/dev/null
    rm -rf "$WORK_DIR"
}
trap cleanup EXIT

wait_pid_gone() {
    local pid="$1"
    local i
    for i in 1 2 3 4 5; do
        if ! kill -0 "$pid" 2>/dev/null; then
            return 0
        fi
        sleep 1
    done
    return 1
}

seed_fake_kernel() {
    install -d "$CLASH_BASE_DIR/bin" "$CLASH_BASE_DIR/resources"
    install -m 755 "$(command -v sleep)" "$CLASH_BASE_DIR/bin/${KERNEL_NAME}"
}

seed_fake_kernel

log "starting managed kernel binary"
PID_FILE="$CLASH_BASE_DIR/runtime/${KERNEL_NAME}.pid"
install -d "$(dirname "$PID_FILE")"
"$CLASH_BASE_DIR/bin/${KERNEL_NAME}" 60 &
MANAGED_PID="$!"
echo "$MANAGED_PID" > "$PID_FILE"
test -f "$PID_FILE" || fail "managed pid file was not created"
case "$MANAGED_PID" in
    ""|*[!0-9]*) fail "invalid managed pid: $MANAGED_PID" ;;
esac
kill -0 "$MANAGED_PID" 2>/dev/null || fail "managed kernel is not running"

bash "$ROOT_DIR/scripts/init/nohup.sh" status > "$WORK_DIR/status.out"
grep -q '^running$' "$WORK_DIR/status.out" || fail "status did not report running"

log "stopping managed kernel through nohup fallback"
bash "$ROOT_DIR/scripts/init/nohup.sh" stop
wait_pid_gone "$MANAGED_PID" || fail "managed kernel survived stop"
MANAGED_PID=""
test ! -e "$PID_FILE" || fail "managed pid file was not removed"

log "verifying unmanaged pid is not killed"
install -d "$CLASH_BASE_DIR/runtime"
(trap 'exit 0' TERM INT; while :; do sleep 1; done) &
UNMANAGED_PID="$!"
echo "$UNMANAGED_PID" > "$PID_FILE"

set +e
bash "$ROOT_DIR/scripts/init/nohup.sh" stop > "$WORK_DIR/unmanaged.out" 2> "$WORK_DIR/unmanaged.err"
code=$?
set -e

[ "$code" -ne 0 ] || fail "unmanaged pid stop unexpectedly succeeded"
grep -q 'non-managed process' "$WORK_DIR/unmanaged.err" || fail "unmanaged pid error was not explicit"
kill -0 "$UNMANAGED_PID" 2>/dev/null || fail "unmanaged process was killed"
test ! -e "$PID_FILE" || fail "unmanaged pid file was not removed"

kill "$UNMANAGED_PID" 2>/dev/null || true
wait "$UNMANAGED_PID" 2>/dev/null || true
UNMANAGED_PID=""

log "ok"
