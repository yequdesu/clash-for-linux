#!/bin/bash
# ============================================================
# Clash-Terminal — install.sh
# 安装脚本: 下载依赖, 编译工具, 设置环境
# 
# 中国大陆网络支持: 可通过 GH_PROXY 环境变量设置 GitHub 镜像
#     export GH_PROXY=https://ghproxy.net
#     bash install.sh
#
# sudo 说明: 本脚本在需要系统级操作时需要 sudo 权限
#   1. 安装系统依赖 (apt install) — 需 sudo
#   2. 复制二进制到 /usr/local/bin — 需 sudo
#   3. 安装 systemd 服务 — 需 sudo
#   4. 设置内核网络能力 — 需 sudo
# ============================================================

set -euo pipefail

# ========== 颜色 ==========
RED='\033[31m'
GREEN='\033[32m'
YELLOW='\033[33m'
CYAN='\033[36m'
BLUE='\033[34m'
GRAY='\033[90m'
BOLD='\033[1m'
NC='\033[0m'

# ========== 配置变量 ==========
CLASH_BASE_DIR="${HOME}/.clashctl"
CLASH_BIN_DIR="${CLASH_BASE_DIR}/bin"
CLASH_RESOURCES_DIR="${CLASH_BASE_DIR}/resources"
CLASH_LOGS_DIR="${CLASH_BASE_DIR}/logs"
CLASH_RUNTIME_DIR="${CLASH_BASE_DIR}/runtime"
CLASH_CONFIGS_DIR="${CLASH_RESOURCES_DIR}/configs"
CLASH_PROFILES_DIR="${CLASH_RESOURCES_DIR}/profiles"

VERSION_MIHOMO="${VERSION_MIHOMO:-v1.19.17}"
ARCH="$(uname -m)"
MODE="install"

# ========== GitHub 镜像 ==========
# 中国大陆网络加速: 使用 GitHub 镜像代理
# 常用镜像: https://ghproxy.net  https://gh-proxy.com  https://gh.api.99988866.xyz
GH_PROXY="${GH_PROXY:-}"
if [ -n "$GH_PROXY" ]; then
    GITHUB_DOWNLOAD="${GH_PROXY}/https://github.com"
else
    GITHUB_DOWNLOAD="https://github.com"
fi

META_CUBEX="${GITHUB_DOWNLOAD}/MetaCubeX/mihomo/releases/download"
MIKEFARAH="${GITHUB_DOWNLOAD}/mikefarah/yq/releases/download"
META_RULES="${GITHUB_DOWNLOAD}/MetaCubeX/meta-rules-dat/releases/download"

# ========== 函数 ==========

print_banner() {
    echo ""
    echo -e "${CYAN}╔══════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║${NC}       ${BOLD}Clash-Terminal Installer${NC}                    ${CYAN}║${NC}"
    echo -e "${CYAN}║${NC}       Terminal Proxy Management Tool           ${CYAN}║${NC}"
    echo -e "${CYAN}╚══════════════════════════════════════════════╝${NC}"
    echo ""
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

print_deps() {
    echo ""
    echo -e "${BLUE}=== 将安装/检查以下系统依赖 ===${NC}"
    echo ""
    echo -e "  ${CYAN}curl${NC}       - 下载资源"
    echo -e "  ${CYAN}tar${NC}        - 解压归档"
    echo -e "  ${CYAN}gzip${NC}       - 解压 gzip"
    echo -e "  ${CYAN}unzip${NC}      - 解压 zip (规则数据库)"
    echo -e "  ${CYAN}git${NC}        - 克隆代码"
    echo -e "  ${CYAN}systemd${NC}    - 服务管理 (Ubuntu 默认已有)"
    echo ""
    echo -e "${BLUE}=== 可选依赖 ===${NC}"
    echo ""
    echo -e "  ${CYAN}go 1.21+${NC}   - 编译 CLI 工具 (如未安装将尝试下载)"
    echo -e "  ${CYAN}cargo${NC}      - 编译 TUI 仪表盘 (如未安装将跳过 TUI)"
    echo ""
}

install_system_deps() {
    echo ""
    echo -e "${BLUE}[1/10] 检查系统依赖...${NC}"
    
    local missing=""
    for cmd in curl tar gzip unzip git; do
        if ! command -v $cmd &>/dev/null; then
            missing="$missing $cmd"
        fi
    done
    
    if [ -n "$missing" ]; then
        echo -e "${YELLOW}⚠ 缺少依赖:${missing}${NC}"
        echo -e "${YELLOW}⚠ 安装依赖需要 sudo 权限${NC}"
        if ask_confirm "是否安装系统依赖?"; then
            sudo apt-get update -qq
            sudo apt-get install -y $missing
            echo -e "${GREEN}✓ 系统依赖安装完成${NC}"
        else
            echo -e "${RED}✗ 缺少必要依赖, 安装无法继续${NC}"
            exit 1
        fi
    else
        echo -e "${GREEN}✓ 所有系统依赖已就绪${NC}"
    fi
}

detect_arch() {
    case "$ARCH" in
        x86_64|amd64)
            # Detect v1/v2/v3
            if [ -f /proc/cpuinfo ]; then
                if grep -q avx512 /proc/cpuinfo 2>/dev/null; then
                    ARCH="amd64-v4"
                elif grep -q avx2 /proc/cpuinfo 2>/dev/null; then
                    ARCH="amd64-v3"
                elif grep -q sse4_2 /proc/cpuinfo 2>/dev/null; then
                    ARCH="amd64-v2"
                else
                    ARCH="amd64"
                fi
            else
                ARCH="amd64"
            fi
            ;;
        aarch64|arm64)
            ARCH="arm64"
            ;;
        armv7l)
            ARCH="armv7"
            ;;
        *)
            echo -e "${YELLOW}⚠ 未知架构: $ARCH, 使用 amd64${NC}"
            ARCH="amd64"
            ;;
    esac
}

create_dirs() {
    echo ""
    echo -e "${BLUE}[2/10] 创建目录结构...${NC}"
    
    mkdir -p "$CLASH_BIN_DIR"
    mkdir -p "$CLASH_LOGS_DIR"
    mkdir -p "$CLASH_RUNTIME_DIR"
    mkdir -p "$CLASH_CONFIGS_DIR"
    mkdir -p "$CLASH_PROFILES_DIR"
    
    echo -e "${GREEN}✓ 目录结构已创建${NC}"
}

download_mihomo() {
    echo ""
    echo -e "${BLUE}[3/10] 下载 Mihomo 内核...${NC}"
    
        local mihomo_arch="$ARCH"
        case "$ARCH" in
            amd64-v3|amd64-v4)
                mihomo_arch="amd64-compatible"
                ;;
        esac
    
    local filename="mihomo-linux-${mihomo_arch}-${VERSION_MIHOMO}.gz"
    local url="${META_CUBEX}/${VERSION_MIHOMO}/${filename}"
    
    echo -e "  ${GRAY}下载: ${url}${NC}"
    
    if curl -fsSL --connect-timeout 30 --max-time 120 "$url" -o "/tmp/${filename}" 2>/dev/null; then
        echo -e "  ${GRAY}解压...${NC}"
        gunzip -f "/tmp/${filename}"
        local extracted="/tmp/mihomo-linux-${mihomo_arch}-${VERSION_MIHOMO}"
        if [ -f "$extracted" ]; then
            mv "$extracted" "${CLASH_BIN_DIR}/mihomo"
        else
            mv "/tmp/${filename%.gz}" "${CLASH_BIN_DIR}/mihomo"
        fi
        chmod +x "${CLASH_BIN_DIR}/mihomo"
        echo -e "${GREEN}✓ Mihomo 内核下载完成 ($VERSION_MIHOMO)${NC}"
    else
        echo -e "${RED}✗ 下载失败${NC}"
        echo -e "${YELLOW}  请检查网络连接或尝试设置 GH_PROXY 环境变量${NC}"
        echo -e "${YELLOW}  示例: export GH_PROXY=https://ghproxy.net${NC}"
        
        if ask_confirm "是否继续安装? (Mihomo内核将需要手动下载)"; then
            echo -e "${YELLOW}⚠ 跳过 Mihomo 下载, 稍后可运行: clashctl upgrade-kernel${NC}"
        else
            exit 1
        fi
    fi
}

download_yq() {
    echo ""
    echo -e "${BLUE}[4/10] 下载 yq YAML 工具...${NC}"
    
    local yq_version="v4.44.3"
    local filename="yq_linux_amd64"
    local url="${MIKEFARAH}/${yq_version}/${filename}.tar.gz"
    
    echo -e "  ${GRAY}下载: ${url}${NC}"
    
    if curl -fsSL --connect-timeout 15 --max-time 60 "$url" -o "/tmp/yq.tar.gz" 2>/dev/null; then
        cd /tmp
        tar xzf yq.tar.gz
        # The yq tarball contains: yq_linux_amd64 (binary), yq (alternate name), man page
        if [ -f "/tmp/yq_linux_amd64" ]; then
            mv /tmp/yq_linux_amd64 "${CLASH_BIN_DIR}/yq"
        elif [ -f "/tmp/yq" ]; then
            mv /tmp/yq "${CLASH_BIN_DIR}/yq"
        fi
        chmod +x "${CLASH_BIN_DIR}/yq" 2>/dev/null || true
        cd - >/dev/null
        rm -f /tmp/yq.tar.gz /tmp/yq_linux_amd64 /tmp/yq /tmp/man_mikefarah_yq 2>/dev/null || true
        
        if [ -f "${CLASH_BIN_DIR}/yq" ]; then
            echo -e "${GREEN}✓ yq 下载完成${NC}"
        else
            echo -e "${YELLOW}⚠ yq 下载但未能正确安装, 配置合并功能可能受限${NC}"
        fi
    else
        echo -e "${YELLOW}⚠ yq 下载失败, 将使用内置简化合并${NC}"
    fi
}

download_rules() {
    echo ""
    echo -e "${BLUE}[5/10] 下载规则数据库...${NC}"
    
    local rules_version="latest"
    
    # Country.mmdb
    local mmdb_url="${META_RULES}/${rules_version}/Country.mmdb"
    echo -e "  ${GRAY}下载 Country.mmdb...${NC}"
    if curl -fsSL --connect-timeout 15 --max-time 60 "$mmdb_url" -o "${CLASH_RESOURCES_DIR}/Country.mmdb" 2>/dev/null; then
        echo -e "  ${GREEN}✓ Country.mmdb${NC}"
    else
        echo -e "  ${YELLOW}⚠ Country.mmdb 下载失败${NC}"
    fi
    
    # geosite.dat
    local geosite_url="${META_RULES}/${rules_version}/geosite.dat"
    echo -e "  ${GRAY}下载 geosite.dat...${NC}"
    if curl -fsSL --connect-timeout 15 --max-time 60 "$geosite_url" -o "${CLASH_RESOURCES_DIR}/geosite.dat" 2>/dev/null; then
        echo -e "  ${GREEN}✓ geosite.dat${NC}"
    else
        echo -e "  ${YELLOW}⚠ geosite.dat 下载失败${NC}"
    fi
    
    # geoip.dat
    local geoip_url="${META_RULES}/${rules_version}/geoip.dat"
    echo -e "  ${GRAY}下载 geoip.dat...${NC}"
    if curl -fsSL --connect-timeout 15 --max-time 60 "$geoip_url" -o "${CLASH_RESOURCES_DIR}/geoip.dat" 2>/dev/null; then
        echo -e "  ${GREEN}✓ geoip.dat${NC}"
    else
        echo -e "  ${YELLOW}⚠ geoip.dat 下载失败${NC}"
    fi
}

install_resources() {
    echo ""
    echo -e "${BLUE}[6/10] 安装资源文件...${NC}"
    
    # Copy resource files from the source directory
    local SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
    
    # Copy mixin.yaml if available
    if [ -f "${SCRIPT_DIR}/resources/mixin.yaml" ]; then
        cp "${SCRIPT_DIR}/resources/mixin.yaml" "${CLASH_RESOURCES_DIR}/mixin.yaml"
    else
        # Create default mixin.yaml
        cat > "${CLASH_RESOURCES_DIR}/mixin.yaml" << 'MIXIN_EOF'
# ================================
# Linux CLI&TUI Clash — mixin.yaml
# ================================
mixed-port: 7897
socks-port: 7898
port: 7899
mode: rule
log-level: info
ipv6: false
allow-lan: false
external-controller: 127.0.0.1:9090
secret: ""
tun:
  enable: false
  stack: gvisor
  auto-route: true
  auto-detect-interface: true
  dns-hijack:
    - any:53
dns:
  enable: true
  enhanced-mode: fake-ip
  nameserver:
    - 223.5.5.5
    - 119.29.29.29
  fallback:
    - 8.8.8.8
    - 1.1.1.1
MIXIN_EOF
    fi
    
    # Copy profiles.yaml if available
    if [ -f "${SCRIPT_DIR}/resources/profiles.yaml" ]; then
        cp "${SCRIPT_DIR}/resources/profiles.yaml" "${CLASH_RESOURCES_DIR}/profiles.yaml"
    else
        cat > "${CLASH_RESOURCES_DIR}/profiles.yaml" << 'PROFILE_EOF'
use: 0
profiles: []
PROFILE_EOF
    fi
    
    # Create .env
    cat > "${CLASH_BASE_DIR}/.env" << ENV_EOF
# Clash-Terminal 环境配置
CLASH_BASE_DIR=${CLASH_BASE_DIR}
CLASH_BIN_DIR=${CLASH_BIN_DIR}
CLASH_RESOURCES_DIR=${CLASH_RESOURCES_DIR}
CLASH_LOGS_DIR=${CLASH_LOGS_DIR}
CLASH_RUNTIME_DIR=${CLASH_RUNTIME_DIR}
CLASH_MIXED_PORT=7897
CLASH_CONTROLLER=127.0.0.1:9090
VERSION_MIHOMO=${VERSION_MIHOMO}
ENV_EOF
    
    echo -e "${GREEN}✓ 资源文件安装完成${NC}"
}

set_kernel_caps() {
    echo ""
    echo -e "${BLUE}[7/10] 设置内核网络能力...${NC}"
    
    if [ -f "${CLASH_BIN_DIR}/mihomo" ]; then
        echo -e "${YELLOW}⚠ 此操作需要 sudo 权限来设置网络能力${NC}"
        echo -e "${YELLOW}  需要的能力: cap_net_admin, cap_net_raw${NC}"
        if ask_confirm "是否设置内核网络能力?"; then
            sudo setcap cap_net_admin,cap_net_raw+ep "${CLASH_BIN_DIR}/mihomo"
            echo -e "${GREEN}✓ 内核网络能力已设置${NC}"
        else
            echo -e "${YELLOW}⚠ 跳过网络能力设置, TUN 模式可能无法使用${NC}"
        fi
    else
        echo -e "${YELLOW}⚠ Mihomo 内核未安装, 跳过${NC}"
    fi
}

build_cli() {
    echo ""
    echo -e "${BLUE}[8/10] 编译 CLI 工具...${NC}"
    
    local SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
    
    # Check if go source exists
    local go_build=0
    if [ -f "${SCRIPT_DIR}/cmd/clashctl/main.go" ]; then
        if command -v go &>/dev/null; then
            local go_version=$(go version 2>/dev/null | grep -oP 'go\K[0-9]+\.[0-9]+' | head -1)
            if [ -n "$go_version" ]; then
                local major=$(echo "$go_version" | cut -d. -f1)
                local minor=$(echo "$go_version" | cut -d. -f2)
                if [ "$major" -gt 1 ] || ([ "$major" -eq 1 ] && [ "$minor" -ge 21 ]); then
                    go_build=1
                fi
            fi
        fi
    fi
    
    if [ "$go_build" -eq 1 ]; then
        echo -e "  ${GRAY}编译 clashctl...${NC}"
        cd "${SCRIPT_DIR}"
        # Try to download dependencies
        go mod download 2>/dev/null || true
        go build -ldflags="-s -w" -o "${CLASH_BIN_DIR}/clashctl" ./cmd/clashctl/ 2>/dev/null && {
            echo -e "${GREEN}✓ clashctl 编译成功${NC}"
            echo -e "${YELLOW}⚠ 复制到 /usr/local/bin 需要 sudo 权限${NC}"
            if ask_confirm "是否复制 clashctl 到 /usr/local/bin?"; then
                sudo cp "${CLASH_BIN_DIR}/clashctl" /usr/local/bin/clashctl
                sudo chmod +x /usr/local/bin/clashctl
                echo -e "${GREEN}✓ clashctl 已安装到 /usr/local/bin/clashctl${NC}"
            else
                echo -e "  ${CYAN}→ 手动使用: ${CLASH_BIN_DIR}/clashctl${NC}"
            fi
        } || {
            echo -e "${RED}✗ clashctl 编译失败${NC}"
            echo -e "${YELLOW}  请确保 Go 1.21+ 已安装: https://go.dev/dl/${NC}"
        }
        cd - >/dev/null
    else
        echo -e "${YELLOW}⚠ Go 未安装或版本过低 (需要 1.21+)${NC}"
        echo -e "${YELLOW}  CLI 工具将跳过编译${NC}"
        echo -e "${YELLOW}  安装 Go: https://go.dev/dl/${NC}"
    fi
}

install_systemd() {
    echo ""
    echo -e "${BLUE}[9/10] 配置 systemd 服务 (可选)...${NC}"
    
    if ! command -v systemctl &>/dev/null; then
        echo -e "${GRAY}systemd 不可用, 跳过${NC}"
        return
    fi
    
    echo -e "${YELLOW}⚠ 安装 systemd 服务需要 sudo 权限${NC}"
    if ask_confirm "是否安装 systemd 服务? (推荐)"; then
        local SERVICE_FILE="/etc/systemd/system/clashctl.service"
        
        sudo tee "$SERVICE_FILE" > /dev/null << SERVICE_EOF
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
        
        sudo systemctl daemon-reload
        echo -e "${GREEN}✓ systemd 服务已安装${NC}"
        echo -e "  ${CYAN}→ 启动服务: sudo systemctl start clashctl${NC}"
        echo -e "  ${CYAN}→ 开机自启: sudo systemctl enable clashctl${NC}"
    else
        echo -e "${GRAY}跳过 systemd 服务安装${NC}"
    fi
}

build_tui() {
    echo ""
    echo -e "${BLUE}[10/10] 编译 TUI 仪表盘 (可选)...${NC}"
    
    local SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
    
    if [ ! -f "${SCRIPT_DIR}/tui/Cargo.toml" ]; then
        echo -e "${GRAY}TUI 源码不存在, 跳过${NC}"
        return
    fi
    
    if ! command -v cargo &>/dev/null; then
        echo -e "${YELLOW}⚠ Cargo (Rust) 未安装, 跳过 TUI 编译${NC}"
        echo -e "${YELLOW}  安装 Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh${NC}"
        return
    fi
    
    echo -e "${YELLOW}⚠ 编译 TUI 可能需要几分钟${NC}"
    if ask_confirm "是否编译 TUI 仪表盘?" "N"; then
        cd "${SCRIPT_DIR}/tui"
        cargo build --release && {
            cp target/release/clash-tui "${CLASH_BIN_DIR}/clash-tui" 2>/dev/null || \
            cp target/release/tui-template "${CLASH_BIN_DIR}/clash-tui" 2>/dev/null || \
            echo -e "${YELLOW}⚠ TUI 编译成功但复制失败${NC}"
            
            echo -e "${YELLOW}⚠ 复制到 /usr/local/bin 需要 sudo 权限${NC}"
            if ask_confirm "是否复制 clash-tui 到 /usr/local/bin?"; then
                sudo cp "${CLASH_BIN_DIR}/clash-tui" /usr/local/bin/clash-tui
                sudo chmod +x /usr/local/bin/clash-tui
                echo -e "${GREEN}✓ clash-tui 已安装到 /usr/local/bin/clash-tui${NC}"
            fi
        } || echo -e "${RED}✗ TUI 编译失败${NC}"
        cd - >/dev/null
    else
        echo -e "${GRAY}跳过 TUI 编译${NC}"
    fi
}

setup_shell_integration() {
    echo ""
    echo -e "${BLUE}Shell 集成...${NC}"
    
    # Inject into .bashrc
    if [ -f "${HOME}/.bashrc" ]; then
        if ! grep -q "# clashctl START" "${HOME}/.bashrc" 2>/dev/null; then
            cat >> "${HOME}/.bashrc" << 'BASHRC_EOF'

# clashctl START
export http_proxy=http://127.0.0.1:7897
export HTTP_PROXY=http://127.0.0.1:7897
export https_proxy=http://127.0.0.1:7897
export HTTPS_PROXY=http://127.0.0.1:7897
export all_proxy=socks5h://127.0.0.1:7897
export ALL_PROXY=socks5h://127.0.0.1:7897
export no_proxy=localhost,127.0.0.0/8,::1
export NO_PROXY=localhost,127.0.0.0/8,::1
# clashctl END
BASHRC_EOF
            echo -e "  ${GREEN}✓ .bashrc 已更新${NC}"
        fi
    fi
    
    # Inject into .zshrc
    if [ -f "${HOME}/.zshrc" ]; then
        if ! grep -q "# clashctl START" "${HOME}/.zshrc" 2>/dev/null; then
            cat >> "${HOME}/.zshrc" << 'ZSHRC_EOF'

# clashctl START
export http_proxy=http://127.0.0.1:7897
export HTTP_PROXY=http://127.0.0.1:7897
export https_proxy=http://127.0.0.1:7897
export HTTPS_PROXY=http://127.0.0.1:7897
export all_proxy=socks5h://127.0.0.1:7897
export ALL_PROXY=socks5h://127.0.0.1:7897
export no_proxy=localhost,127.0.0.0/8,::1
export NO_PROXY=localhost,127.0.0.0/8,::1
# clashctl END
ZSHRC_EOF
            echo -e "  ${GREEN}✓ .zshrc 已更新${NC}"
        fi
    fi
}

print_success() {
    echo ""
    echo -e "${GREEN}╔══════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║${NC}       ${BOLD}Clash-Terminal 安装完成!${NC}               ${GREEN}║${NC}"
    echo -e "${GREEN}╚══════════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "  ${BOLD}快速开始:${NC}"
    echo ""
    echo -e "  ${CYAN}clashctl help${NC}    查看帮助"
    echo -e "  ${CYAN}clashctl start${NC}   启动代理"
    echo -e "  ${CYAN}clashctl status${NC}  查看状态"
    echo -e "  ${CYAN}clashctl tui${NC}     启动仪表盘"
    echo ""
    echo -e "  ${GRAY}安装目录: ${CLASH_BASE_DIR}${NC}"
    echo -e "  ${GRAY}配置文件: ${CLASH_RESOURCES_DIR}/mixin.yaml${NC}"
    echo -e "  ${GRAY}日志文件: ${CLASH_LOGS_DIR}/mihomo.log${NC}"
    echo ""
}

# ========== 主流程 ==========

main() {
    print_banner
    print_deps
    
    if ! ask_confirm "是否继续安装?"; then
        echo "安装已取消"
        exit 0
    fi
    
    # 检查是否为 root
    if [ "$(id -u)" -eq 0 ]; then
        echo -e "${YELLOW}⚠ 检测到以 root 用户运行${NC}"
        echo -e "${YELLOW}  建议以普通用户身份运行, sudo 将在需要时自动请求${NC}"
        if ! ask_confirm "是否继续?"; then
            exit 0
        fi
    fi
    
    install_system_deps
    detect_arch
    create_dirs
    download_mihomo
    download_yq
    download_rules
    install_resources
    set_kernel_caps
    build_cli
    install_systemd
    build_tui
    setup_shell_integration
    print_success
}

main "$@"
