#!/bin/bash
# ============================================================
# Clash-Terminal — nohup 启动脚本 (systemd fallback)
# 用法: bash scripts/init/nohup.sh [start|stop|status]
# ============================================================

set -euo pipefail

CLASH_BASE_DIR="${HOME}/.clashctl"
CLASH_BIN_DIR="${CLASH_BASE_DIR}/bin"
CLASH_RESOURCES_DIR="${CLASH_BASE_DIR}/resources"
CLASH_LOGS_DIR="${CLASH_BASE_DIR}/logs"
CLASH_RUNTIME_DIR="${CLASH_BASE_DIR}/runtime"

MHDM_PATH="${CLASH_BIN_DIR}/mihomo"
PID_FILE="${CLASH_RUNTIME_DIR}/mihomo.pid"
LOG_FILE="${CLASH_LOGS_DIR}/mihomo.log"

GREEN='\033[32m'
YELLOW='\033[33m'
RED='\033[31m'
NC='\033[0m'

start_nohup() {
    if [ -f "$PID_FILE" ] && kill -0 $(cat "$PID_FILE") 2>/dev/null; then
        echo -e "${YELLOW}Mihomo 已在运行${NC}"
        return 1
    fi
    
    if [ ! -f "$MHDM_PATH" ]; then
        echo -e "${RED}Mihomo 内核未找到: $MHDM_PATH${NC}"
        return 1
    fi
    
    mkdir -p "$CLASH_LOGS_DIR" "$CLASH_RUNTIME_DIR"
    
    nohup "$MHDM_PATH" \
        -d "$CLASH_RESOURCES_DIR" \
        -f "${CLASH_RESOURCES_DIR}/runtime.yaml" \
        > "$LOG_FILE" 2>&1 &
    
    local pid=$!
    echo "$pid" > "$PID_FILE"
    echo -e "${GREEN}✓ Mihomo 已启动 (PID: $pid)${NC}"
}

stop_nohup() {
    if [ -f "$PID_FILE" ]; then
        local pid=$(cat "$PID_FILE")
        if kill -0 "$pid" 2>/dev/null; then
            kill "$pid"
            echo -e "${GREEN}✓ Mihomo 已停止${NC}"
        fi
        rm -f "$PID_FILE"
    else
        pkill -9 mihomo 2>/dev/null || true
        echo -e "${GREEN}✓ Mihomo 已停止${NC}"
    fi
}

status_nohup() {
    if [ -f "$PID_FILE" ] && kill -0 $(cat "$PID_FILE") 2>/dev/null; then
        echo -e "${GREEN}● 运行中 (PID: $(cat $PID_FILE))${NC}"
    else
        echo -e "${YELLOW}○ 未运行${NC}"
    fi
}

case "${1:-}" in
    start)
        start_nohup
        ;;
    stop)
        stop_nohup
        ;;
    status)
        status_nohup
        ;;
    *)
        echo "用法: $0 [start|stop|status]"
        exit 1
        ;;
esac
