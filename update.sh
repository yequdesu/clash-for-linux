#!/bin/bash
# ============================================================
# Clash-Terminal — update.sh
# 升级脚本: 更新代码, 重新编译, 检查内核更新
#
# sudo 说明: 重新编译后的二进制复制到 /usr/local/bin 需 sudo
# ============================================================

set -euo pipefail

# ========== 清除系统代理 (mihomo 可能未运行) ==========
unset http_proxy https_proxy HTTP_PROXY HTTPS_PROXY all_proxy ALL_PROXY no_proxy NO_PROXY
export http_proxy="" https_proxy="" HTTP_PROXY="" HTTPS_PROXY="" all_proxy="" ALL_PROXY="" no_proxy="" NO_PROXY=""

RED='\033[31m'
GREEN='\033[32m'
YELLOW='\033[33m'
CYAN='\033[36m'
BLUE='\033[34m'
GRAY='\033[90m'
BOLD='\033[1m'
NC='\033[0m'

CLASH_BASE_DIR="${HOME}/.clashctl"
CLASH_BIN_DIR="${CLASH_BASE_DIR}/bin"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

ask_confirm() {
    local prompt="$1"
    local default="${2:-N}"
    if [ "$default" = "Y" ]; then
        read -r -p "$(echo -e "${YELLOW}${prompt} [Y/n]:${NC} ")" answer
        answer="${answer:-Y}"
    else
        read -r -p "$(echo -e "${YELLOW}${prompt} [y/N]:${NC} ")" answer
        answer="${answer:-N}"
    fi
    [[ "$answer" =~ ^[Yy]([Ee][Ss])?$ ]]
}

stop_kernel() {
    echo -e "${BLUE}[1/5] 停止内核...${NC}"
    if command -v clashctl &>/dev/null; then
        clashctl stop 2>/dev/null || true
    elif [ -f "${CLASH_BIN_DIR}/clashctl" ]; then
        "${CLASH_BIN_DIR}/clashctl" stop 2>/dev/null || true
    fi
    pkill -9 mihomo 2>/dev/null || true
    echo -e "${GREEN}✓ 内核已停止${NC}"
}

update_source() {
    echo ""
    echo -e "${BLUE}[2/5] 更新源码...${NC}"
    
    if [ ! -d "${SCRIPT_DIR}/.git" ]; then
        echo -e "${YELLOW}⚠ 非 Git 仓库, 跳过源码更新${NC}"
        return
    fi
    
    cd "${SCRIPT_DIR}"
    git pull origin clash-terminal 2>/dev/null || {
        echo -e "${YELLOW}⚠ Git pull 失败, 继续使用当前代码${NC}"
    }
    echo -e "${GREEN}✓ 源码已更新${NC}"
}

rebuild_cli() {
    echo ""
    echo -e "${BLUE}[3/5] 重新编译 CLI...${NC}"
    
    local go_bin="${CLASH_BIN_DIR}/clashctl"
    
    if [ ! -f "${SCRIPT_DIR}/cmd/clashctl/main.go" ]; then
        echo -e "${YELLOW}⚠ CLI 源码不存在, 跳过${NC}"
        return
    fi
    
    if ! command -v go &>/dev/null; then
        echo -e "${YELLOW}⚠ Go 未安装, 跳过 CLI 编译${NC}"
        return
    fi
    
    cd "${SCRIPT_DIR}"
    echo -e "  ${GRAY}下载依赖...${NC}"
    go mod download 2>/dev/null || true
    echo -e "  ${GRAY}编译...${NC}"
    GONOSUMCHECK=* GOFLAGS=-mod=mod go build -v -ldflags="-s -w" -o "$go_bin" ./cmd/clashctl/ 2>&1 | tail -3 && {
        echo -e "${GREEN}✓ clashctl 编译成功${NC}"
        
        echo -e "${YELLOW}⚠ 更新 /usr/local/bin/clashctl 需要 sudo 权限${NC}"
        if ask_confirm "是否更新到 /usr/local/bin?"; then
            sudo cp "$go_bin" /usr/local/bin/clashctl
            sudo chmod +x /usr/local/bin/clashctl
            echo -e "${GREEN}✓ 已更新${NC}"
        fi
    } || echo -e "${RED}✗ 编译失败${NC}"
    cd - >/dev/null
}

rebuild_tui() {
    echo ""
    echo -e "${BLUE}[4/5] 重新编译 TUI (可选)...${NC}"
    
    if [ ! -f "${SCRIPT_DIR}/tui/Cargo.toml" ]; then
        echo -e "${GRAY}TUI 源码不存在, 跳过${NC}"
        return
    fi
    
    if ! command -v cargo &>/dev/null; then
        echo -e "${GRAY}Cargo 未安装, 跳过${NC}"
        return
    fi
    
    if ask_confirm "是否重新编译 TUI?" "N"; then
        cd "${SCRIPT_DIR}/tui"
        cargo build --release && {
            cp target/release/clash-tui "${CLASH_BIN_DIR}/clash-tui" 2>/dev/null || true
            
            echo -e "${YELLOW}⚠ 更新 /usr/local/bin/clash-tui 需要 sudo 权限${NC}"
            if ask_confirm "是否更新到 /usr/local/bin?"; then
                sudo cp "${CLASH_BIN_DIR}/clash-tui" /usr/local/bin/clash-tui
                sudo chmod +x /usr/local/bin/clash-tui
            fi
            echo -e "${GREEN}✓ TUI 更新完成${NC}"
        } || echo -e "${RED}✗ TUI 编译失败${NC}"
        cd - >/dev/null
    else
        echo -e "${GRAY}跳过 TUI 编译${NC}"
    fi
}

restart_kernel() {
    echo ""
    echo -e "${BLUE}[5/5] 重启内核...${NC}"
    
    if ask_confirm "是否重启代理?" "Y"; then
        if command -v clashctl &>/dev/null; then
            clashctl start
        else
            echo -e "${YELLOW}⚠ clashctl 未安装, 无法自动重启${NC}"
        fi
    else
        echo -e "${GRAY}跳过重启${NC}"
    fi
}

main() {
    echo ""
    echo -e "${CYAN}╔══════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║${NC}       ${BOLD}Clash-Terminal Updater${NC}                   ${CYAN}║${NC}"
    echo -e "${CYAN}╚══════════════════════════════════════════════╝${NC}"
    echo ""
    
    stop_kernel
    update_source
    rebuild_cli
    rebuild_tui
    restart_kernel
    
    echo ""
    echo -e "${GREEN}╔══════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║${NC}       ${BOLD}Clash-Terminal 更新完成${NC}                ${GREEN}║${NC}"
    echo -e "${GREEN}╚══════════════════════════════════════════════╝${NC}"
    echo ""
}

main "$@"
