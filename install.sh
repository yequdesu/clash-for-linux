#!/bin/bash
# ============================================================
# Clash-Terminal — install.sh
# 安装脚本: 下载依赖, 编译工具, 设置环境
#
# 网络: 自动检测 GitHub 连通性, 不可达时自动使用镜像
#   - 直连检测: curl --connect-timeout 5 https://github.com
#   - 自动镜像: https://gh-proxy.com/ (中国大陆网络)
#
# sudo: 需要时自动提示用户确认
#   1. 安装系统依赖 (apt install)
#   2. 复制二进制到 /usr/local/bin
#   3. 安装 systemd 服务
#   4. 设置内核网络能力
# ============================================================

set -euo pipefail

# ========== 清除系统代理 (mihomo 还未运行, 否则所有连接失败) ==========
unset http_proxy https_proxy HTTP_PROXY HTTPS_PROXY all_proxy ALL_PROXY no_proxy NO_PROXY
export http_proxy="" https_proxy="" HTTP_PROXY="" HTTPS_PROXY="" all_proxy="" ALL_PROXY="" no_proxy="" NO_PROXY=""

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

VERSION_MIHOMO="v1.19.17"
ARCH=""
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
STEP=0
TOTAL=10

# ========== 镜像列表 ==========
MIRROR_LIST=(
    "https://gh-proxy.org"
    "https://gh-proxy.com"
    "https://ghproxy.net"
)

# ========== 智能下载 (直连 → 镜像自动降级) ==========
download_file() {
    local url="$1"
    local output="$2"
    local desc="$3"
    
    # 1. 尝试直连
    echo -e "  ${GRAY}↓ ${desc} [direct]${NC}"
    if curl -fSL --connect-timeout 5 --max-time 10 "$url" -o "$output"; then
        echo -e "  ${GREEN}✓${NC} ${desc} ${GRAY}($(du -h "$output" 2>/dev/null | cut -f1 || echo 'ok'))${NC}"
        return 0
    fi
    echo -e "  ${YELLOW}⚠ direct failed${NC}"
    
    # 2. 逐镜像降级
    for mirror in "${MIRROR_LIST[@]}"; do
        local mirror_url="${mirror}/${url}"
        echo -e "  ${GRAY}↓ ${desc} [${mirror##*/}]${NC}"
        echo -e "    ${GRAY}${mirror_url}${NC}"
        if curl -fSL --connect-timeout 10 --max-time 300 "$mirror_url" -o "$output"; then
            echo -e "  ${GREEN}✓${NC} ${desc} ${GRAY}($(du -h "$output" 2>/dev/null | cut -f1 || echo 'ok'))${NC}"
            return 0
        fi
        echo -e "  ${YELLOW}⚠ ${mirror##*/} failed${NC}"
    done
    
    # 3. 全部失败
    echo -e "  ${RED}✗${NC} ${desc} ${RED}all attempts failed${NC}"
    return 1
}

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

step_title() {
    STEP=$((STEP + 1))
    echo ""
    echo -e "${BLUE}[${STEP}/${TOTAL}] $1${NC}"
}

print_deps() {
    echo ""
    echo -e "${BLUE}=== 将安装/检查以下系统依赖 ===${NC}"
    echo ""
    echo -e "  ${CYAN}curl${NC}       - 下载资源"
    echo -e "  ${CYAN}tar${NC}        - 解压归档"
    echo -e "  ${CYAN}gzip${NC}       - 解压 gzip"
    echo -e "  ${CYAN}unzip${NC}      - 解压 zip (规则数据库)"
    echo -e "  ${CYAN}git${NC}        - 克隆代码 (已安装则跳过)"
    echo -e "  ${CYAN}systemd${NC}    - 服务管理 (Ubuntu 默认已有)"
    echo ""
    echo -e "${BLUE}=== 可选依赖 ===${NC}"
    echo ""
    echo -e "  ${CYAN}go 1.21+${NC}   - 编译 CLI 工具 (如未安装将尝试下载)"
    echo -e "  ${CYAN}cargo${NC}      - 编译 TUI 仪表盘 (如未安装将跳过 TUI)"
    echo ""
}

install_system_deps() {
    step_title "检查系统依赖"
    
    local missing=""
    for cmd in curl tar gzip unzip git; do
        if ! command -v "$cmd" &>/dev/null; then
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
    local machine
    machine="$(uname -m)"
    
    case "$machine" in
        x86_64|amd64)
            # Mihomo releases use plain "amd64" for all x86_64 variants
            # (v1/v2/v3/v4 detection is for info only, not for filename)
            ARCH="amd64"
            ;;
        aarch64|arm64)
            ARCH="arm64"
            ;;
        armv7l|armv7)
            ARCH="armv7"
            ;;
        *)
            echo -e "${YELLOW}⚠ 未知架构: $machine, 使用 amd64${NC}"
            ARCH="amd64"
            ;;
    esac
    
    echo -e "  ${GRAY}检测到架构: ${machine} → ${ARCH}${NC}"
}

create_dirs() {
    step_title "创建目录结构"
    
    mkdir -p "$CLASH_BIN_DIR"
    mkdir -p "$CLASH_LOGS_DIR"
    mkdir -p "$CLASH_RUNTIME_DIR"
    mkdir -p "$CLASH_CONFIGS_DIR"
    mkdir -p "$CLASH_PROFILES_DIR"
    
    echo -e "${GREEN}✓ 目录结构已创建${NC}"
}

download_mihomo() {
    step_title "下载 Mihomo 内核"
    
    if [ -f "${CLASH_BIN_DIR}/mihomo" ]; then
        echo -e "  ${GREEN}✓${NC} Mihomo ${GRAY}(already cached)${NC}"
        return
    fi
    
    local filename="mihomo-linux-${ARCH}-${VERSION_MIHOMO}.gz"
    local url="https://github.com/MetaCubeX/mihomo/releases/download/${VERSION_MIHOMO}/${filename}"
    
    if ! download_file "$url" "/tmp/${filename}" "Mihomo ${VERSION_MIHOMO} (${ARCH})"; then
        if ask_confirm "是否继续安装? (Mihomo内核将需要手动下载)"; then
            echo -e "${YELLOW}⚠ 跳过 Mihomo 下载, 稍后可运行: clashctl upgrade-kernel${NC}"
            return
        else
            exit 1
        fi
    fi
    
    echo -e "  ${GRAY}解压...${NC}"
    gunzip -f "/tmp/${filename}"
    local extracted="/tmp/${filename%.gz}"
    if [ -f "$extracted" ]; then
        mv "$extracted" "${CLASH_BIN_DIR}/mihomo"
        chmod +x "${CLASH_BIN_DIR}/mihomo"
        echo -e "  ${GREEN}✓ Mihomo 内核就绪${NC}"
    else
        echo -e "  ${RED}✗ 解压失败, 未找到: ${extracted}${NC}"
    fi
    
    rm -f "/tmp/${filename}" "/tmp/${extracted##*/}"
    rm -f "/tmp/${filename}" "/tmp/${filename%.gz}"
}

download_yq() {
    step_title "下载 yq YAML 工具"
    
    if [ -f "${CLASH_BIN_DIR}/yq" ]; then
        echo -e "  ${GREEN}✓${NC} yq ${GRAY}(already cached)${NC}"
        return
    fi
    
    local yq_version="v4.44.3"
    local yq_arch="$ARCH"
    # yq uses linux_amd64 / linux_arm64 naming
    case "$ARCH" in
        amd64) yq_arch="amd64" ;;
        arm64) yq_arch="arm64" ;;
        armv7) yq_arch="arm"   ;;
        *)     yq_arch="amd64" ;;
    esac
    
    local filename="yq_linux_${yq_arch}.tar.gz"
    local url="https://github.com/mikefarah/yq/releases/download/${yq_version}/${filename}"
    
    if ! download_file "$url" "/tmp/yq.tar.gz" "yq ${yq_version} (${yq_arch})"; then
        echo -e "${YELLOW}⚠ yq 下载失败, 将使用内置简化合并${NC}"
        return
    fi
    
    cd /tmp
    tar xzf yq.tar.gz
    
    local found=false
    for candidate in "yq_linux_${yq_arch}" "yq" "./yq"; do
        if [ -f "/tmp/${candidate}" ]; then
            mv "/tmp/${candidate}" "${CLASH_BIN_DIR}/yq"
            chmod +x "${CLASH_BIN_DIR}/yq"
            found=true
            break
        fi
    done
    
    cd - >/dev/null
    rm -f /tmp/yq.tar.gz /tmp/yq_linux_* /tmp/man_mikefarah_yq /tmp/install-man-page.sh 2>/dev/null || true
    
    if $found; then
        echo -e "${GREEN}✓ yq 就绪${NC}"
    else
        echo -e "${YELLOW}⚠ yq 解压后未找到二进制, 将使用内置简化合并${NC}"
    fi
}

download_rules() {
    step_title "下载规则数据库"
    
    local rules_version="latest"
    local base_url
    base_url="https://github.com/MetaCubeX/meta-rules-dat/releases/download/${rules_version}"
    
    local rules=(
        "Country.mmdb:${CLASH_RESOURCES_DIR}/Country.mmdb"
        "geosite.dat:${CLASH_RESOURCES_DIR}/geosite.dat"
        "geoip.dat:${CLASH_RESOURCES_DIR}/geoip.dat"
    )
    
    for entry in "${rules[@]}"; do
        local name="${entry%%:*}"
        local dest="${entry##*:}"
        local url="${base_url}/${name}"
        
        if [ -f "$dest" ]; then
            echo -e "  ${GREEN}✓${NC} ${name} ${GRAY}(already exists)${NC}"
        else
            download_file "$url" "$dest" "${name}" || true
        fi
    done
}

install_resources() {
    step_title "安装资源文件"
    
    # Copy mixin.yaml template
    if [ -f "${SCRIPT_DIR}/resources/mixin.yaml" ]; then
        cp "${SCRIPT_DIR}/resources/mixin.yaml" "${CLASH_RESOURCES_DIR}/mixin.yaml"
    else
        cat > "${CLASH_RESOURCES_DIR}/mixin.yaml" << 'MIXIN_EOF'
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
    echo -e "  ${GREEN}✓${NC} mixin.yaml"
    
    # Auto-load configs from configs/ directory
    local config_index=1
    local first_config=""
    
    if [ -d "${SCRIPT_DIR}/configs" ]; then
        for yamlfile in "${SCRIPT_DIR}/configs"/*.yaml "${SCRIPT_DIR}/configs"/*.yml; do
            [ -f "$yamlfile" ] || continue
            local dest="${CLASH_CONFIGS_DIR}/$(basename "$yamlfile")"
            cp "$yamlfile" "$dest"
            echo -e "  ${GREEN}✓${NC} configs/$(basename "$yamlfile") → configs/"
            
            if [ -z "$first_config" ]; then
                first_config="$yamlfile"
            fi
        done
    fi
    
    # Generate profiles.yaml from loaded configs
    if [ -n "$first_config" ]; then
        local yaml_name
        yaml_name="$(basename "$first_config" .yaml)"
        yaml_name="${yaml_name%.yml}"
        
        cp "$first_config" "${CLASH_RESOURCES_DIR}/config.yaml"
        echo -e "  ${GREEN}✓${NC} $(basename "$first_config") → config.yaml (active)"
        
        cat > "${CLASH_RESOURCES_DIR}/profiles.yaml" << PROFILE_EOF
use: 1
profiles:
  - id: 1
    path: ${CLASH_RESOURCES_DIR}/config.yaml
    url: file://${CLASH_RESOURCES_DIR}/config.yaml
    name: ${yaml_name}
    updated: 0
    interval: 0
PROFILE_EOF
        echo -e "  ${GREEN}✓${NC} profiles.yaml (auto-generated)"
    else
        # Fallback: empty profiles
        if [ ! -f "${CLASH_RESOURCES_DIR}/profiles.yaml" ]; then
            cp "${SCRIPT_DIR}/resources/profiles.yaml" "${CLASH_RESOURCES_DIR}/profiles.yaml" 2>/dev/null || \
            cat > "${CLASH_RESOURCES_DIR}/profiles.yaml" << 'PROFILE_EOF'
use: 0
profiles: []
PROFILE_EOF
        fi
        echo -e "  ${GRAY}○${NC} profiles.yaml (empty, no configs found)"
    fi
    
    # Create .env
    cat > "${CLASH_BASE_DIR}/.env" << ENV_EOF
CLASH_BASE_DIR=${CLASH_BASE_DIR}
CLASH_BIN_DIR=${CLASH_BIN_DIR}
CLASH_RESOURCES_DIR=${CLASH_RESOURCES_DIR}
CLASH_LOGS_DIR=${CLASH_LOGS_DIR}
CLASH_RUNTIME_DIR=${CLASH_RUNTIME_DIR}
CLASH_MIXED_PORT=7897
CLASH_CONTROLLER=127.0.0.1:9090
VERSION_MIHOMO=${VERSION_MIHOMO}
ENV_EOF
    echo -e "  ${GREEN}✓${NC} .env"
}

set_kernel_caps() {
    step_title "设置内核网络能力"
    
    if [ -f "${CLASH_BIN_DIR}/mihomo" ]; then
        echo -e "${YELLOW}⚠ 此操作需要 sudo 权限来设置网络能力 (cap_net_admin, cap_net_raw)${NC}"
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
    step_title "编译 CLI 工具"
    
    if [ ! -f "${SCRIPT_DIR}/cmd/clashctl/main.go" ]; then
        echo -e "${YELLOW}⚠ CLI 源码不存在, 跳过${NC}"
        return
    fi
    
    if ! command -v go &>/dev/null; then
        echo -e "${YELLOW}⚠ Go 未安装, 跳过 CLI 编译${NC}"
        echo -e "${YELLOW}  安装 Go: https://go.dev/dl/${NC}"
        return
    fi
    
    local go_version
    go_version=$(go version 2>/dev/null | grep -oP 'go[0-9]+\.[0-9]+(\.[0-9]+)?' || echo "unknown")
    echo -e "  ${GRAY}Go: ${go_version}${NC}"
    
    cd "${SCRIPT_DIR}"
    
    echo -e "  ${GRAY}下载 Go 依赖...${NC}"
    if ! go mod download; then
        echo -e "  ${YELLOW}⚠ proxy.golang.org 不可达, 尝试国内源...${NC}"
        
        local goproxy_ok=false
        for goproxy in "https://goproxy.cn,direct" "https://goproxy.io,direct" "https://mirrors.aliyun.com/goproxy/,direct"; do
            echo -e "  ${GRAY}→ ${goproxy%%,*}... ${NC}"
            if GOPROXY="$goproxy" go mod download; then
                export GOPROXY="$goproxy"
                echo -e "  ${GREEN}✓${NC} 依赖就绪 ${GRAY}(${goproxy%%,*})${NC}"
                goproxy_ok=true
                break
            fi
        done
        
        if ! $goproxy_ok; then
            echo -e "${RED}✗ Go 模块下载失败 (所有源均不可达)${NC}"
            echo -e "  ${YELLOW}  手动: go env -w GOPROXY=https://goproxy.cn,direct${NC}"
            cd - >/dev/null
            return
        fi
    else
        echo -e "  ${GREEN}✓${NC} 依赖就绪"
    fi
    
    echo -e "  ${GRAY}编译 clashctl...${NC}"
    if GONOSUMCHECK=* GOFLAGS=-mod=mod go build -v -ldflags="-s -w" -o "${CLASH_BIN_DIR}/clashctl" ./cmd/clashctl/ 2>&1 | tail -3; then
        echo -e "${GREEN}✓ clashctl 编译成功${NC}"
        
        echo -e "${YELLOW}⚠ 复制到 /usr/local/bin 需要 sudo 权限${NC}"
        if ask_confirm "是否复制 clashctl 到 /usr/local/bin?"; then
            sudo cp "${CLASH_BIN_DIR}/clashctl" /usr/local/bin/clashctl
            sudo chmod +x /usr/local/bin/clashctl
            echo -e "${GREEN}✓ clashctl 已安装到 /usr/local/bin/clashctl${NC}"
        else
            echo -e "  ${CYAN}→ 手动使用: ${CLASH_BIN_DIR}/clashctl${NC}"
        fi
    else
        echo -e "${RED}✗ clashctl 编译失败${NC}"
        echo -e "${YELLOW}  请确保 Go 1.21+ 已安装并可用${NC}"
    fi
    
    cd - >/dev/null
}

install_systemd() {
    step_title "配置 systemd 服务 (可选)"
    
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
    step_title "编译 TUI 仪表盘 (可选)"
    
    if [ ! -f "${SCRIPT_DIR}/tui/Cargo.toml" ]; then
        echo -e "${GRAY}TUI 源码不存在, 跳过${NC}"
        return
    fi
    
    if ! command -v cargo &>/dev/null; then
        echo -e "${YELLOW}⚠ Cargo (Rust) 未安装, 跳过 TUI 编译${NC}"
        echo -e "${YELLOW}  安装 Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh${NC}"
        return
    fi
    
    echo -e "  ${GRAY}Rust: $(rustc --version)${NC}"
    echo -e "${YELLOW}⚠ 编译 TUI 可能需要 2-5 分钟${NC}"
    if ask_confirm "是否编译 TUI 仪表盘?" "N"; then
        cd "${SCRIPT_DIR}/tui"
        
        echo -e "  ${GRAY}编译 release...${NC}"
        if cargo build --release 2>&1 | tail -5; then
            cp target/release/clash-tui "${CLASH_BIN_DIR}/clash-tui" && \
                echo -e "${GREEN}✓ clash-tui 编译成功${NC}" || \
                echo -e "${YELLOW}⚠ 编译成功但二进制未找到${NC}"
            
            if [ -f "${CLASH_BIN_DIR}/clash-tui" ]; then
                echo -e "${YELLOW}⚠ 复制到 /usr/local/bin 需要 sudo 权限${NC}"
                if ask_confirm "是否复制 clash-tui 到 /usr/local/bin?"; then
                    sudo cp "${CLASH_BIN_DIR}/clash-tui" /usr/local/bin/clash-tui
                    sudo chmod +x /usr/local/bin/clash-tui
                    echo -e "${GREEN}✓ clash-tui 已安装到 /usr/local/bin/clash-tui${NC}"
                fi
            fi
        else
            echo -e "${RED}✗ TUI 编译失败${NC}"
        fi
        
        cd - >/dev/null
    else
        echo -e "${GRAY}跳过 TUI 编译${NC}"
    fi
}

setup_shell_integration() {
    echo ""
    echo -e "${BLUE}Shell 集成...${NC}"

    # DO NOT auto-inject proxy env vars into shell RC files.
    # The proxy may not be running when the user opens a new shell,
    # which would break all network connections.
    # Instead, users should run 'clashctl proxy on' after starting the kernel.

    for rc in "${HOME}/.bashrc" "${HOME}/.zshrc"; do
        if [ -f "$rc" ]; then
            if grep -q "# clashctl START" "$rc" 2>/dev/null; then
                echo -e "  ${GRAY}○${NC} $(basename "$rc") ${GRAY}(proxy already configured)${NC}"
            else
                echo -e "  ${GRAY}  $(basename "$rc") found${NC}"
            fi
        fi
    done

    echo ""
    echo -e "  ${BOLD}Important:${NC}"
    echo -e "  ${YELLOW}Proxy settings are NOT automatically added to your shell RC files.${NC}"
    echo -e "  ${YELLOW}To enable system proxy AFTER starting the kernel, run:${NC}"
    echo ""
    echo -e "    ${CYAN}clashctl start${NC}                # Start the proxy kernel"
    echo -e "    ${CYAN}clashctl proxy on${NC}            # Enable system proxy (persistent)"
    echo -e "    ${CYAN}eval \$(clashctl env)${NC}          # Enable proxy in current shell only"
    echo ""
    echo -e "  ${YELLOW}To disable the proxy:${NC}"
    echo ""
    echo -e "    ${CYAN}clashctl proxy off${NC}           # Disable system proxy"
    echo -e "    ${CYAN}unset http_proxy https_proxy all_proxy${NC}  # Current shell only"
    echo ""
}

print_success() {
    echo ""
    echo -e "${GREEN}╔══════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║${NC}       ${BOLD}Clash-Terminal 安装完成!${NC}               ${GREEN}║${NC}"
    echo -e "${GREEN}╚══════════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "  ${BOLD}快速开始:${NC}"
    echo ""
    echo -e "  ${CYAN}clashctl start${NC}   启动代理 (如已加载配置)"
    echo -e "  ${CYAN}clashctl status${NC}  查看状态"
    echo -e "  ${CYAN}clashctl help${NC}    查看帮助"
    echo -e "  ${CYAN}clashctl tui${NC}     启动仪表盘"
    echo ""
    echo -e "  ${GRAY}安装目录: ${CLASH_BASE_DIR}${NC}"
    echo -e "  ${GRAY}配置文件: ${CLASH_RESOURCES_DIR}/mixin.yaml${NC}"
    echo -e "  ${GRAY}日志文件: ${CLASH_LOGS_DIR}/mihomo.log${NC}"
    
    # Check if config already loaded
    if [ -f "${CLASH_RESOURCES_DIR}/config.yaml" ]; then
        echo ""
        echo -e "  ${GREEN}✓ 配置文件已自动加载, 直接运行:${NC}"
        echo -e "    ${CYAN}clashctl start${NC}"
    fi
    
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
