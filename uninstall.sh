#!/bin/bash
# ============================================================
# Clash-Terminal — uninstall.sh
# 卸载脚本: 停止服务, 移除二进制, 清理配置
#
# sudo 说明: 停止/移除 systemd 服务和 /usr/local/bin 下文件需 sudo
# ============================================================

set -euo pipefail

RED='\033[31m'
GREEN='\033[32m'
YELLOW='\033[33m'
CYAN='\033[36m'
GRAY='\033[90m'
BOLD='\033[1m'
NC='\033[0m'

CLASH_BASE_DIR="${HOME}/.clashctl"
CLASH_CONFIG_DIR="${HOME}/.config/clashctl"

print_banner() {
    echo ""
    echo -e "${RED}╔══════════════════════════════════════════════╗${NC}"
    echo -e "${RED}║${NC}       ${BOLD}Clash-Terminal Uninstaller${NC}               ${RED}║${NC}"
    echo -e "${RED}╚══════════════════════════════════════════════╝${NC}"
    echo ""
}

ask_confirm() {
    local prompt="$1"
    read -r -p "$(echo -e "${YELLOW}${prompt} [y/N]:${NC} ")" answer
    [[ "$answer" =~ ^[Yy]([Ee][Ss])?$ ]]
}

stop_kernel() {
    echo ""
    echo -e "${RED}[1/7] 停止内核进程...${NC}"
    
    # Stop systemd service if exists
    if systemctl is-active --quiet clashctl 2>/dev/null; then
        echo -e "  ${YELLOW}⚠ 停止 systemd 服务需要 sudo 权限${NC}"
        if ask_confirm "  是否停止 systemd 服务?"; then
            sudo systemctl stop clashctl 2>/dev/null || true
            echo -e "  ${GREEN}✓ systemd 服务已停止${NC}"
        fi
    fi
    
    # Kill clashctl-managed mihomo by PID file (not system-wide pkill)
    local pid_file="${CLASH_BASE_DIR}/runtime/mihomo.pid"
    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file" 2>/dev/null)
        if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
            kill "$pid" 2>/dev/null || true
            sleep 1
            kill -9 "$pid" 2>/dev/null || true
        fi
        rm -f "$pid_file"
    fi
    
    echo -e "${GREEN}✓ 内核进程已停止${NC}"
}

remove_systemd() {
    echo ""
    echo -e "${RED}[2/7] 移除 systemd 服务...${NC}"
    
    if [ -f /etc/systemd/system/clashctl.service ]; then
        echo -e "  ${YELLOW}⚠ 移除 systemd 服务需要 sudo 权限${NC}"
        if ask_confirm "  是否移除 systemd 服务?"; then
            sudo systemctl disable clashctl 2>/dev/null || true
            sudo rm -f /etc/systemd/system/clashctl.service
            sudo systemctl daemon-reload
            echo -e "  ${GREEN}✓ systemd 服务已移除${NC}"
        fi
    else
        echo -e "  ${GRAY}未找到 systemd 服务${NC}"
    fi
}

remove_binaries() {
    echo ""
    echo -e "${RED}[3/7] 移除二进制文件...${NC}"
    
    if [ -f /usr/local/bin/clashctl ] || [ -f /usr/local/bin/clash-tui ]; then
        echo -e "  ${YELLOW}⚠ 移除 /usr/local/bin 下文件需要 sudo 权限${NC}"
        if ask_confirm "  是否移除?"; then
            sudo rm -f /usr/local/bin/clashctl
            sudo rm -f /usr/local/bin/clash-tui
            echo -e "  ${GREEN}✓ 二进制文件已移除${NC}"
        fi
    else
        echo -e "  ${GRAY}未找到系统级二进制文件${NC}"
    fi
}

clean_shell_rc() {
    echo ""
    echo -e "${RED}[4/7] 清理 Shell RC 文件...${NC}"
    
    for rc in "${HOME}/.bashrc" "${HOME}/.zshrc"; do
        if [ -f "$rc" ]; then
            if grep -q "# clashctl START" "$rc" 2>/dev/null; then
                sed -i '/# clashctl START/,/# clashctl END/d' "$rc"
                echo -e "  ${GREEN}✓ 已清理: $rc${NC}"
            fi
        fi
    done
}

clean_cron() {
    echo ""
    echo -e "${RED}[5/7] 清理 cron 定时任务...${NC}"
    
    if command -v crontab &>/dev/null; then
        local current=$(crontab -l 2>/dev/null || true)
        if echo "$current" | grep -q "clashctl"; then
            echo "$current" | grep -v "clashctl" | crontab - 2>/dev/null || true
            echo -e "  ${GREEN}✓ cron 任务已清理${NC}"
        else
            echo -e "  ${GRAY}未找到 cron 任务${NC}"
        fi
    fi
}

remove_data_dirs() {
    echo ""
    echo -e "${RED}[6/7] 移除数据目录...${NC}"
    
    if [ -d "$CLASH_BASE_DIR" ]; then
        if ask_confirm "  是否删除 ${CLASH_BASE_DIR}?"; then
            rm -rf "$CLASH_BASE_DIR"
            echo -e "  ${GREEN}✓ 已删除: $CLASH_BASE_DIR${NC}"
        else
            echo -e "  ${GRAY}保留: $CLASH_BASE_DIR${NC}"
        fi
    fi
    
    if [ -d "$CLASH_CONFIG_DIR" ]; then
        rm -rf "$CLASH_CONFIG_DIR"
        echo -e "  ${GREEN}✓ 已删除: $CLASH_CONFIG_DIR${NC}"
    fi
}

verify_cleanup() {
    echo ""
    echo -e "${RED}[7/7] 验证清理...${NC}"
    
    local has_issues=0
    
    # Check mihomo processes
    if pgrep -x mihomo > /dev/null 2>&1; then
        echo -e "  ${RED}⚠ 仍有 mihomo 进程运行${NC}"
        has_issues=1
    fi
    
    # Check binaries
    if [ -f /usr/local/bin/clashctl ]; then
        echo -e "  ${YELLOW}⚠ /usr/local/bin/clashctl 仍然存在${NC}"
        has_issues=1
    fi
    if [ -f /usr/local/bin/clash-tui ]; then
        echo -e "  ${YELLOW}⚠ /usr/local/bin/clash-tui 仍然存在${NC}"
        has_issues=1
    fi
    
    # Check data dir
    if [ -d "$CLASH_BASE_DIR" ]; then
        echo -e "  ${GRAY}  ${CLASH_BASE_DIR} 仍然存在${NC}"
    fi
    
    if [ "$has_issues" -eq 0 ]; then
        echo -e "  ${GREEN}✓ 清理完成, 未发现残留${NC}"
    fi
}

# ========== 主流程 ==========

main() {
    print_banner
    
    echo -e "${RED}⚠ 警告: 此操作将卸载 Clash-Terminal!${NC}"
    echo -e "${RED}   这将停止代理, 移除所有文件和配置${NC}"
    echo ""
    
    if ! ask_confirm "确认卸载?"; then
        echo "卸载已取消"
        exit 0
    fi
    
    stop_kernel
    remove_systemd
    remove_binaries
    clean_shell_rc
    clean_cron
    remove_data_dirs
    verify_cleanup
    
    echo ""
    echo -e "${GREEN}╔══════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║${NC}       ${BOLD}Clash-Terminal 卸载完成${NC}                ${GREEN}║${NC}"
    echo -e "${GREEN}╚══════════════════════════════════════════════╝${NC}"
    echo ""
}

main "$@"
