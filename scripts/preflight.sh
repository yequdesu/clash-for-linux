#!/bin/bash
# ============================================================
# Clash-Terminal — preflight.sh
# 安装前检查 + 资源下载 (被 install.sh 调用)
# ============================================================

set -euo pipefail

GREEN='\033[32m'
YELLOW='\033[33m'
RED='\033[31m'
CYAN='\033[36m'
NC='\033[0m'

# GitHub 镜像
GH_PROXY="${GH_PROXY:-}"
if [ -n "$GH_PROXY" ]; then
    GITHUB_DL="${GH_PROXY}/https://github.com"
else
    GITHUB_DL="https://github.com"
fi

check_commands() {
    local cmds=("$@")
    for cmd in "${cmds[@]}"; do
        if ! command -v "$cmd" &>/dev/null; then
            echo -e "${RED}✗ 缺少命令: $cmd${NC}"
            return 1
        fi
    done
    return 0
}

download_with_fallback() {
    local url="$1"
    local output="$2"
    local name="$3"
    
    # Try direct
    if curl -fsSL --connect-timeout 15 --max-time 60 "$url" -o "$output" 2>/dev/null; then
        echo -e "${GREEN}✓ $name${NC}"
        return 0
    fi
    
    # Try with GH_PROXY if set
    if [ -n "$GH_PROXY" ]; then
        local proxy_url="${GH_PROXY}/${url#https://}"
        if curl -fsSL --connect-timeout 15 --max-time 60 "$proxy_url" -o "$output" 2>/dev/null; then
            echo -e "${GREEN}✓ $name (via proxy)${NC}"
            return 0
        fi
    fi
    
    echo -e "${YELLOW}⚠ $name 下载失败${NC}"
    return 1
}

echo -e "${CYAN}=== Preflight 检查 ===${NC}"

# 检查网络
echo -e "  检查网络连接..."
if curl -fsSL --connect-timeout 5 "https://github.com" -o /dev/null 2>/dev/null; then
    echo -e "  ${GREEN}✓ GitHub 可访问${NC}"
elif [ -n "$GH_PROXY" ] && curl -fsSL --connect-timeout 5 "${GH_PROXY}/https://github.com" -o /dev/null 2>/dev/null; then
    echo -e "  ${GREEN}✓ GitHub 可访问 (via proxy)${NC}"
else
    echo -e "  ${YELLOW}⚠ GitHub 可能无法访问${NC}"
    echo -e "  ${YELLOW}  设置 GH_PROXY 环境变量以使用镜像:${NC}"
    echo -e "  ${YELLOW}  export GH_PROXY=https://ghproxy.net${NC}"
fi

echo -e "${CYAN}=== Preflight 完成 ===${NC}"
