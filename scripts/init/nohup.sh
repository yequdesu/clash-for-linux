#!/usr/bin/env bash
# nohup fallback script for non-systemd systems

KERNEL_BIN="${CLASH_BASE_DIR}/bin/${KERNEL_NAME:-mihomo}"
LOG_FILE="${CLASH_BASE_DIR}/logs/${KERNEL_NAME:-mihomo}.log"
RESOURCES_DIR="${CLASH_BASE_DIR}/resources"

case "$1" in
    start)
        nohup "$KERNEL_BIN" -d "$RESOURCES_DIR" -f "${RESOURCES_DIR}/runtime.yaml" >> "$LOG_FILE" 2>&1 &
        echo "PID: $!"
        ;;
    stop)
        pkill -9 -f "$KERNEL_BIN" 2>/dev/null || true
        pkill -9 -x "${KERNEL_NAME:-mihomo}" 2>/dev/null || true
        ;;
    status)
        pgrep -f "$KERNEL_BIN" >/dev/null 2>&1 && echo "running" || echo "stopped"
        ;;
esac
