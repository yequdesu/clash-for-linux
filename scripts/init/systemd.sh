#!/bin/bash
# ============================================================
# Clash-Terminal — systemd service 管理器
# 用法: bash scripts/init/systemd.sh [install|remove|start|stop|status]
# ============================================================

set -euo pipefail

CLASH_BASE_DIR="${HOME}/.clashctl"
CLASH_BIN_DIR="${CLASH_BASE_DIR}/bin"
CLASH_RESOURCES_DIR="${CLASH_BASE_DIR}/resources"
CLASH_LOGS_DIR="${CLASH_BASE_DIR}/logs"
CLASH_RUNTIME_DIR="${CLASH_BASE_DIR}/runtime"
SERVICE_NAME="clashctl"
SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}.service"

GREEN='\033[32m'
YELLOW='\033[33m'
RED='\033[31m'
CYAN='\033[36m'
NC='\033[0m'

need_sudo() {
    echo -e "${YELLOW}⚠ 此操作需要 sudo 权限${NC}"
}

generate_service() {
    cat << SERVICE_EOF
[Unit]
Description=Clashctl Proxy Service (Mihomo)
After=network.target NetworkManager.service systemd-networkd.service iwd.service

[Service]
Type=simple
User=${USER}
LimitNPROC=500
LimitNOFILE=1000000
CapabilityBoundingSet=CAP_NET_ADMIN CAP_NET_RAW CAP_NET_BIND_SERVICE
AmbientCapabilities=CAP_NET_ADMIN CAP_NET_RAW CAP_NET_BIND_SERVICE
Restart=always
RestartSec=3
ExecStartPre=/usr/bin/sleep 1s
ExecStart=${CLASH_BIN_DIR}/mihomo -d ${CLASH_RESOURCES_DIR} -f ${CLASH_RESOURCES_DIR}/runtime.yaml
ExecStop=/bin/kill -SIGTERM \$MAINPID
StandardOutput=append:${CLASH_LOGS_DIR}/mihomo.log
StandardError=append:${CLASH_LOGS_DIR}/mihomo.log

[Install]
WantedBy=multi-user.target
SERVICE_EOF
}

install_service() {
    echo -e "${CYAN}安装 systemd 服务...${NC}"
    need_sudo
    
    generate_service | sudo tee "$SERVICE_FILE" > /dev/null
    sudo systemctl daemon-reload
    
    echo -e "${GREEN}✓ 服务已安装${NC}"
    echo -e "  启动: sudo systemctl start ${SERVICE_NAME}"
    echo -e "  自启: sudo systemctl enable ${SERVICE_NAME}"
}

remove_service() {
    echo -e "${CYAN}移除 systemd 服务...${NC}"
    need_sudo
    
    sudo systemctl stop "$SERVICE_NAME" 2>/dev/null || true
    sudo systemctl disable "$SERVICE_NAME" 2>/dev/null || true
    sudo rm -f "$SERVICE_FILE"
    sudo systemctl daemon-reload
    
    echo -e "${GREEN}✓ 服务已移除${NC}"
}

start_service() {
    echo -e "${CYAN}启动服务...${NC}"
    need_sudo
    
    sudo systemctl start "$SERVICE_NAME"
    echo -e "${GREEN}✓ 服务已启动${NC}"
}

stop_service() {
    echo -e "${CYAN}停止服务...${NC}"
    need_sudo
    
    sudo systemctl stop "$SERVICE_NAME"
    echo -e "${GREEN}✓ 服务已停止${NC}"
}

status_service() {
    systemctl status "$SERVICE_NAME" 2>/dev/null || echo -e "${YELLOW}服务未安装或未运行${NC}"
}

# 主逻辑
case "${1:-}" in
    install)
        install_service
        ;;
    remove)
        remove_service
        ;;
    start)
        start_service
        ;;
    stop)
        stop_service
        ;;
    status)
        status_service
        ;;
    *)
        echo "用法: $0 [install|remove|start|stop|status]"
        exit 1
        ;;
esac
