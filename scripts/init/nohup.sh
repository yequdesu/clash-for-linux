#!/usr/bin/env bash
# nohup fallback script for non-systemd systems

KERNEL_BIN="${CLASH_BASE_DIR}/bin/${KERNEL_NAME:-mihomo}"
LOG_FILE="${CLASH_BASE_DIR}/logs/${KERNEL_NAME:-mihomo}.log"
PID_FILE="${CLASH_BASE_DIR}/runtime/${KERNEL_NAME:-mihomo}.pid"
RESOURCES_DIR="${CLASH_BASE_DIR}/resources"

pid_matches_kernel() {
    local pid="$1"
    local expected exe exe_real cmd0 cmd0_real
    expected="$(readlink -f "$KERNEL_BIN" 2>/dev/null || printf '%s' "$KERNEL_BIN")"

    exe="$(readlink "/proc/${pid}/exe" 2>/dev/null || true)"
    exe_real="$(readlink -f "/proc/${pid}/exe" 2>/dev/null || true)"
    case "$exe" in
        "$expected"|"$expected (deleted)"|"$KERNEL_BIN"|"$KERNEL_BIN (deleted)") return 0 ;;
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

case "$1" in
    start)
        mkdir -p "$(dirname "$LOG_FILE")" "$(dirname "$PID_FILE")"
        nohup "$KERNEL_BIN" -d "$RESOURCES_DIR" -f "${RESOURCES_DIR}/runtime.yaml" >> "$LOG_FILE" 2>&1 &
        echo "$!" > "$PID_FILE"
        echo "PID: $!"
        ;;
    stop)
        if [ -f "$PID_FILE" ]; then
            pid="$(cat "$PID_FILE" 2>/dev/null || true)"
            case "$pid" in ""|*[!0-9]*) echo "invalid pid file: $PID_FILE" >&2; rm -f "$PID_FILE"; exit 1 ;; esac
            if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
                if ! pid_matches_kernel "$pid"; then
                    echo "pid file points to non-managed process: $pid" >&2
                    rm -f "$PID_FILE"
                    exit 1
                fi
                kill "$pid" 2>/dev/null || true
                sleep 1
                kill -0 "$pid" 2>/dev/null && kill -9 "$pid" 2>/dev/null || true
            fi
            rm -f "$PID_FILE"
        else
            echo "pid file missing: $PID_FILE" >&2
            exit 1
        fi
        ;;
    status)
        if [ -f "$PID_FILE" ] && pid="$(cat "$PID_FILE" 2>/dev/null)" && case "$pid" in ""|*[!0-9]*) false ;; *) true ;; esac && kill -0 "$pid" 2>/dev/null && pid_matches_kernel "$pid"; then
            echo "running"
        else
            echo "stopped"
        fi
        ;;
esac
