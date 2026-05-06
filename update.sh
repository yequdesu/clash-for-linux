#!/bin/bash
# ============================================================
# Clash-Terminal — update.sh
# 升级脚本: 更新代码, 重新编译, 检查内核更新
#
# sudo 说明: 重新编译后的二进制复制到 /usr/local/bin 需 sudo
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
CLASH_BIN_DIR="${CLASH_BASE_DIR}/bin"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# ========== 网络检测 & 镜像 ==========
GITHUB_MIRROR=""
MIRROR_LIST=(
    "https://gh-proxy.org"
    "https://gh-proxy.com"
    "https://ghproxy.net"
)

detect_network() {
    if curl -fsSL --connect-timeout 5 --max-time 10 "https://github.com" -o /dev/null 2>/dev/null; then
        GITHUB_MIRROR=""
        return
    fi
    for mirror in "${MIRROR_LIST[@]}"; do
        if curl -fsSL --connect-timeout 5 --max-time 10 "${mirror}/https://github.com" -o /dev/null 2>/dev/null; then
            GITHUB_MIRROR="$mirror"
            return
        fi
    done
}

github_api_url() {
    local url="$1"
    if [ -n "$GITHUB_MIRROR" ]; then
        echo "${GITHUB_MIRROR}/${url}"
    else
        echo "$url"
    fi
}

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
    echo -e "${BLUE}[1/6] 停止内核...${NC}"
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
    echo -e "${BLUE}[2/6] 更新源码...${NC}"
    
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
    echo -e "${BLUE}[3/6] 重新编译 CLI...${NC}"
    
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
    go build -v -ldflags="-s -w" -o "$go_bin" ./cmd/clashctl/ 2>&1 | tail -3 && {
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
    echo -e "${BLUE}[4/6] 重新编译 TUI (可选)...${NC}"
    
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

check_kernel_update() {
    echo ""
    echo -e "${BLUE}[5/6] 检查 Mihomo 内核更新...${NC}"
    
    local current_version=""
    if command -v clashctl &>/dev/null; then
        current_version=$(clashctl status 2>/dev/null | grep "Kernel:" | awk '{print $2}' || echo "")
    fi
    
    detect_network
    local api_url
    api_url=$(github_api_url "https://api.github.com/repos/MetaCubeX/mihomo/releases/latest")
    local latest_version=""
    
    latest_version=$(curl -fsSL --connect-timeout 10 "$api_url" 2>/dev/null | \
        grep -oP '"tag_name":\s*"\K[^"]+' | head -1) || true
    
    if [ -z "$latest_version" ]; then
        echo -e "${YELLOW}⚠ 无法获取最新版本信息${NC}"
        echo -e "${YELLOW}  GitHub API 可能无法访问${NC}"
        return
    fi
    
    echo -e "  当前版本: ${CYAN}${current_version:-未知}${NC}"
    echo -e "  最新版本: ${CYAN}${latest_version}${NC}"
    
    if [ "$current_version" = "$latest_version" ] && [ -n "$current_version" ]; then
        echo -e "${GREEN}✓ 已是最新版本${NC}"
        return
    fi
    
    if ask_confirm "是否下载新版内核?"; then
        if command -v clashctl &>/dev/null; then
            clashctl upgrade-kernel
        else
            echo -e "${YELLOW}  clashctl 未安装, 无法自动升级内核${NC}"
        fi
    fi
}

restart_kernel() {
    echo ""
    echo -e "${BLUE}[6/6] 重启内核...${NC}"
    
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
    check_kernel_update
    restart_kernel
    
    echo ""
    echo -e "${GREEN}╔══════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║${NC}       ${BOLD}Clash-Terminal 更新完成${NC}                ${GREEN}║${NC}"
    echo -e "${GREEN}╚══════════════════════════════════════════════╝${NC}"
    echo ""
}

main "$@"
