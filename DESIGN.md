# Linux CLI&TUI Clash — 设计文档

> **基于**: Clash Verge Rev v2.4.8 (REPORT.md)  
> **目标**: Ubuntu 22.04+ Linux 终端代理管理工具  
> **交互方式**: CLI 命令 + TUI 可视化仪表盘  
> **仓库**: 个人 GitHub `linux-cli-tui-clash`  
> **许可证**: GPL-3.0  

---

## 目录

1. [项目定位](#1-项目定位)
2. [功能裁剪决策](#2-功能裁剪决策)
3. [架构设计](#3-架构设计)
4. [目录结构](#4-目录结构)
5. [安装 / 卸载 / 升级](#5-安装--卸载--升级)
6. [CLI 命令设计](#6-cli-命令设计)
7. [TUI 界面设计](#7-tui-界面设计)
8. [配置管理设计](#8-配置管理设计)
9. [内核管理设计](#9-内核管理设计)
10. [订阅管理设计](#10-订阅管理设计)
11. [代理与网络设计](#11-代理与网络设计)
12. [实施路线图](#12-实施路线图)

---

## 1. 项目定位

### 1.1 一句话描述

一款面向 Ubuntu Linux 的终端代理客户端，提供 CLI 命令行完整控制和 TUI 可视化仪表盘两种交互方式。

### 1.2 核心原则

| 原则 | 说明 |
|------|------|
| **终端优先** | 所有操作都可通过 CLI 完成，TUI 是视觉辅助 |
| **轻量** | 不依赖 GUI 框架，最小化二进制体积 |
| **实用** | 保留 Clash Verge Rev 的核心高频功能，裁剪低频 GUI 专属功能 |
| **可移植** | 纯 Rust (TUI) + Go (CLI) + Bash (安装)，单一静态二进制分发 |
| **安全安装** | 安装前明确告知依赖，用户确认后继续 |

### 1.3 从 GUI 到 TUI 的对应关系

| GUI 页面 (原版) | TUI / CLI 实现 |
|----------------|---------------|
| 仪表盘 (Home) | TUI Tab 1: Overview |
| 代理 (Proxies) | TUI Tab 2: Proxies + CLI `clashctl proxy` |
| 订阅 (Profiles) | TUI Tab 3: Subscriptions + CLI `clashctl sub` |
| 连接 (Connections) | TUI Tab 4: Connections |
| 日志 (Logs) | TUI Tab 5: Logs + CLI `clashctl log` |
| 设置 (Settings) | CLI `clashctl config` (TUI 通过 hotkey 调出设置面板) |
| 延迟测试 (Test) | CLI `clashctl test` |
| 解锁检测 (Unlock) | 不实现 (低频功能) |

---

## 2. 功能裁剪决策

### 2.1 保留的核心功能

| 功能 | 原版实现位置 | 新实现 | 原因 |
|------|-------------|--------|------|
| Mihomo 内核管理 (start/stop/restart) | `core/manager/` | Go CLI + Bash | 核心能力 |
| 订阅管理 (增删改查/切换/更新) | `config/profiles/` + `feat/profile.rs` | Go CLI | 核心能力 |
| 配置合并 (Merge) | `enhance/merge.rs` | `yq eval-all` | 核心能力 |
| 代理组切换 | `feat/proxy.rs` | Mihomo API + Go CLI | 核心能力 |
| 代理节点延迟测试 | `feat/clash.rs` + `services/delay.ts` | Mihomo API + Go CLI | 核心能力 |
| 系统代理设置 (环境变量) | `core/sysopt.rs` | Bash RC 注入 | 核心能力 |
| TUN 模式 | `core/sysopt.rs` + `core/service.rs` | Go CLI + systemd | 核心能力 |
| 日志查看 | `cmd/clash.rs` (get_clash_logs) | Go CLI | 核心能力 |
| 连接查看 | Mihomo WebSocket | Mihomo API + Go CLI | 核心能力 |
| 实时流量监控 | `utils/connections_stream.rs` | Mihomo API 轮询 | 核心能力 |
| 订阅自动更新 | `core/timer.rs` | cron + Go CLI | 核心能力 |

### 2.2 裁剪的功能 (原因)

| 功能 | 原版实现 | 裁剪原因 |
|------|---------|---------|
| WebDAV 云备份 | `core/backup.rs` + `feat/backup.rs` | 低频；终端用户倾向 `cp -r` |
| 脚本增强 (Boa JS) | `enhance/script.rs` | TUI/CLI 不适合代码编辑 |
| 规则/代理组编辑 | Monaco editor | 终端不适合 YAML 编辑器 |
| 链式代理 (dialer-proxy) | `enhance/chain.rs` | 低频高级功能 |
| 托盘图标/菜单 | `core/tray/` | 终端无系统托盘 |
| 全局热键 | `core/hotkey.rs` | 终端内由应用自行处理 |
| 窗口状态管理 | `utils/window_manager.rs` | 终端无窗口概念 |
| 自定义主题/CSS注入 | `IVergeTheme` | 终端色彩方案有限 |
| 多语言 (13种) | i18n 系统 | 仅保留英文 |
| IP 地理位置检测 | `services/api.ts` | 低频；可外接 `curl ip.sb` |
| 媒体解锁检测 | `cmd/media_unlock_checker/` | 低频 |
| 二维码显示 | `QrViewer` | 终端不适合 |
| 自动更新 (自更新) | `core/updater.rs` | 通过 `bash update.sh` 替代 |
| 开机自启动 | `core/autostart.rs` | systemd service 已覆盖 |
| Windows 服务模式 | `core/service.rs` (Windows) | Linux 平台不适用 |
| 加密存储 (AES-GCM) | `config/encrypt.rs` | 终端用户自行负责文件权限 |
| 代理守护 (Proxy Guard) | `core/sysopt.rs` (guard) | systemd 已保证进程存活 |

### 2.3 适当简化的功能

| 功能 | 简化方案 |
|------|---------|
| 配置增强流水线 (10步) | 简化为订阅配置 + yq 合并 + 校验 (2步) |
| DNS 独立配置 | 合并到 mixin.yaml 中 |
| PAC 模式 | 不实现 (终端不使用 PAC) |
| 连接管理 (表格→列表) | 简化为列表视图 |
| 延迟测试 (批量并发) | 保留但降低默认并发 |

---

## 3. 架构设计

### 3.1 总体架构

```
┌─────────────────────────────────────────────────────┐
│                    Ubuntu Linux                       │
│                                                       │
│  ┌─────────────────┐  ┌───────────────────────────┐  │
│  │  Bash 安装层    │  │  用户交互层                 │  │
│  │  install.sh     │  │                            │  │
│  │  uninstall.sh   │  │  ┌─────────────────────┐  │  │
│  │  update.sh      │  │  │ clashctl (Go CLI)   │  │  │
│  └────────┬────────┘  │  │ - 内核管理           │  │  │
│           │            │  │ - 订阅管理           │  │  │
│  ┌────────▼────────┐  │  │ - 代理设置           │  │  │
│  │  资源层          │  │  │ - 配置管理           │  │  │
│  │ - mihomo 内核   │  │  │ - TUN 模式           │  │  │
│  │ - Country.mmdb  │  │  │ - 状态查看           │  │  │
│  │ - geosite.dat   │  │  └─────────────────────┘  │  │
│  │ - geoip.dat     │  │                            │  │
│  │ - yq (YAML工具) │  │  ┌─────────────────────┐  │  │
│  └─────────────────┘  │  │ clash-tui (Rust)    │  │  │
│                        │  │ - Dashboard 仪表盘  │  │  │
│  ┌─────────────────┐  │  │ - Proxies 代理管理  │  │  │
│  │  Mihomo 内核     │  │  │ - Subscriptions     │  │  │
│  │  (sidecar 进程)  │  │  │ - Connections       │  │  │
│  │  HTTP API :9090  │◄─┤  │ - Logs 日志         │  │  │
│  └─────────────────┘  │  └─────────────────────┘  │  │
│                        └───────────────────────────┘  │
│                                                       │
│  ┌──────────────────────────────────────────────┐    │
│  │  系统层                                        │    │
│  │  - systemd service (可选)                      │    │
│  │  - 环境变量 (http_proxy / ALL_PROXY)          │    │
│  │  - Shell RC 注入 (.bashrc / .zshrc)           │    │
│  │  - cron 定时任务                               │    │
│  └──────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────┘
```

### 3.2 技术选型

| 组件 | 技术 | 理由 |
|------|------|------|
| CLI 工具 | Go 1.21+ (cobra) | 单二进制，交叉编译方便，HTTP 客户端成熟 |
| TUI 仪表盘 | Rust (ratatui + crossterm) | 基于 TUI-template，性能好，终端兼容 |
| 安装脚本 | Bash 5+ | Ubuntu 默认，兼容性最佳 |
| 配置合并 | `yq` (mikefarah/yq) | YAML deep-merge 标准工具 |
| 内核通信 | Mihomo HTTP REST API | 原生接口，无需额外 IPC |
| 服务管理 | systemd (优先) / nohup (fallback) | Ubuntu 标准 |
| 日志 | 内核 stdout → 文件 | 简单可靠 |

### 3.3 进程间通信

```
clashctl (Go CLI)
    │
    ├── HTTP → Mihomo REST API (127.0.0.1:9090)
    │          Endpoints: /proxies, /proxies/{name}/delay,
    │                     /connections, /configs, /version, /logs
    │
    ├── fork+exec → mihomo 内核进程
    │
    └── file I/O → ~/.clashctl/ (配置/日志/运行时状态)

clash-tui (Rust TUI)
    │
    ├── HTTP → Mihomo REST API (同上)
    │      获取: 代理、连接、流量、版本、内存
    │
    └── subprocess → clashctl (复用 CLI 功能)
```

---

## 4. 目录结构

```
linux-cli-tui-clash/
│
├── install.sh                    # 安装脚本 (Bash)
├── uninstall.sh                  # 卸载脚本 (Bash)
├── update.sh                     # 升级脚本 (Bash)
├── Makefile                      # 构建辅助
├── README.md                     # 项目文档
├── DESIGN.md                     # 本设计文档
├── .env                          # 默认配置模板
│
├── cmd/
│   └── clashctl/
│       ├── main.go               # 入口, cobra 根命令
│       ├── start.go              # clashctl start
│       ├── stop.go               # clashctl stop
│       ├── restart.go            # clashctl restart
│       ├── status.go             # clashctl status
│       ├── log.go                # clashctl log
│       ├── proxy.go              # clashctl proxy [on/off/status]
│       ├── tun.go                # clashctl tun [on/off/status]
│       ├── config.go             # clashctl config [edit/view/merge]
│       ├── sub.go                # clashctl sub <子命令>
│       ├── sub_add.go            # clashctl sub add
│       ├── sub_remove.go         # clashctl sub remove
│       ├── sub_list.go           # clashctl sub list
│       ├── sub_use.go            # clashctl sub use
│       ├── sub_update.go         # clashctl sub update
│       ├── test.go               # clashctl test [url|name]
│       ├── secret.go             # clashctl secret [show|new]
│       ├── tui.go                # clashctl tui (启动 TUI)
│       ├── env.go                # clashctl env (输出代理环境变量)
│       ├── upgrade_kernel.go     # clashctl upgrade-kernel
│       └── utils.go              # 公共工具函数
│
├── internal/
│   ├── kernel/
│   │   ├── api.go                # Mihomo HTTP API 客户端
│   │   └── service.go            # 内核服务管理 (systemd/nohup)
│   ├── config/
│   │   ├── env.go                # 环境配置加载 (.env)
│   │   ├── merge.go              # yq 合并逻辑
│   │   └── profiles.go           # 订阅索引 CRUD
│   ├── sub/
│   │   └── download.go           # 订阅下载 (HTTP + file://)
│   └── log/
│       └── fmt.go                # 彩色日志输出
│
├── tui/
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── build-linux.sh            # 交叉编译脚本
│   └── src/
│       ├── main.rs               # 入口: 事件循环, 键盘映射
│       ├── app.rs                # 应用状态, 渲染调度, 5 个 Tab 渲染器
│       ├── api.rs                # Mihomo API 客户端 (拦截 clash API)
│       ├── config.rs             # TUI 配置文件加载
│       ├── theme.rs              # 颜色主题 (匹配 GUI 暗色方案)
│       ├── event.rs              # 事件系统 (Key, Tick, DataReady)
│       ├── window.rs             # 窗口管理 (位置/大小/持久化)
│       ├── background.rs         # 背景特效 (粒子/无)
│       └── widgets/
│           ├── mod.rs
│           ├── card.rs           # 边框卡片容器
│           ├── tab_bar.rs        # 5 标签页导航栏
│           ├── table.rs          # 通用表格 (代理/连接/日志)
│           ├── status_dot.rs     # 运行状态指示灯 (● breathing)
│           ├── gauge.rs          # 流量/内存 进度条
│           └── sparkline.rs      # 实时流量趋势图
│
├── scripts/
│   ├── preflight.sh              # 安装前检查 + 资源下载
│   ├── init/
│   │   ├── systemd.sh            # systemd service 模板
│   │   └── nohup.sh              # nohup 启动脚本模板
│   └── rc/
│       └── clashctl.sh           # Bash Shell 函数 (fallback, 无 Go 时)
│
└── resources/                    # 安装时复制到 $CLASH_BASE_DIR
    ├── mixin.yaml                # 用户自定义合并模板
    ├── profiles.yaml             # 订阅索引 (空模板)
    ├── configs/                  # 本地配置目录
    │   └── .gitkeep
    └── profiles/                 # 下载的订阅 YAML 目录
        └── .gitkeep
```

### 4.1 安装后目录 (`~/.clashctl/`)

```
~/.clashctl/
├── .env                          # 运行时环境变量
├── bin/
│   ├── mihomo                    # Mihomo 内核二进制
│   └── yq                        # yq YAML 工具
├── resources/
│   ├── Country.mmdb              # GeoIP 数据库
│   ├── geosite.dat               # 域名分类数据库
│   ├── geoip.dat                 # IP 地理位置数据库
│   ├── config.yaml               # 当前订阅源 (激活的)
│   ├── mixin.yaml                # 用户自定义合并配置
│   ├── runtime.yaml              # 最终合并后运行时配置
│   ├── profiles.yaml             # 订阅索引
│   ├── profiles.log              # 订阅操作日志
│   ├── configs/                  # 本地 YAML 配置
│   └── profiles/                 # 下载的订阅文件
│       ├── 1.yaml
│       └── 2.yaml
├── logs/
│   ├── mihomo.log                # 内核运行日志
│   └── clashctl.log              # CLI 操作日志
└── runtime/
    ├── mihomo.pid                # 内核进程 PID
    └── api_secret                # API 密钥
```

---

## 5. 安装 / 卸载 / 升级

### 5.1 install.sh 流程

```
1. 欢迎信息 & 依赖确认
   ├── 列出将安装的包: curl, tar, gzip, unzip
   ├── 询问用户是否继续 [Y/n]
   └── 检查必要命令是否存在, 缺失则 apt install (需 sudo)

2. 架构检测
   ├── x86_64 → v1/v2/v3 (根据 /proc/cpuinfo)
   ├── aarch64
   └── armv7l

3. 下载资源 (preflight.sh)
   ├── Mihomo 内核 (MetaCubeX/mihomo releases)
   │   └── 从 GitHub 下载, 支持 GH_PROXY 加速
   ├── yq YAML 工具 (mikefarah/yq releases)
   └── 规则数据库 (MetaCubeX/meta-rules-dat):
       ├── Country.mmdb
       ├── geosite.dat
       └── geoip.dat

4. 创建目录结构
   └── mkdir -p ~/.clashctl/{bin,resources/{configs,profiles},logs,runtime}

5. 安装文件
   ├── 复制资源到 ~/.clashctl/resources/
   ├── 生成 .env 配置文件
   ├── 复制默认 mixin.yaml
   └── 设置文件权限

6. 设置内核能力
   └── sudo setcap cap_net_admin,cap_net_raw+ep ~/.clashctl/bin/mihomo

7. 构建 Go CLI
   ├── 检测 go 是否安装
   ├── go build -ldflags="-s -w" -o /usr/local/bin/clashctl ./cmd/clashctl
   └── 若无 Go: 使用预编译二进制 / 安装 Go

8. Shell 集成
   ├── 向 .bashrc / .zshrc 注入 source 行
   │   └── # clashctl START / # clashctl END 标记
   └── 复制 Bash 函数文件到 ~/.clashctl/scripts/

9. systemd 服务 (可选)
   ├── 询问是否安装 systemd 服务 [Y/n]
   └── 生成并启用 clashctl.service

10. 构建 TUI (可选)
    ├── 询问是否编译 TUI 仪表盘 [Y/n]
    ├── 检测 cargo 是否安装
    └── cargo build --release (或使用预编译二进制)

11. 完成 & 使用提示
    ├── clashctl help  查看帮助
    ├── clashctl tui   启动 TUI
    └── clashctl start 启动代理
```

### 5.2 依赖声明 (安装时展示)

```
=== 将安装以下系统依赖 ===
  curl       - 下载资源
  tar        - 解压归档
  gzip       - 解压 gzip
  unzip      - 解压 zip (规则数据库)
  systemd    - 服务管理 (Ubuntu 默认已有)

=== 可选依赖 ===
  go 1.21+   - 编译 CLI 工具 (如未安装将尝试下载)
  cargo      - 编译 TUI 仪表盘 (如未安装将跳过 TUI)

是否继续? [Y/n]
```

### 5.3 uninstall.sh 流程

```
1. 停止内核进程
   ├── systemctl stop clashctl (如有 service)
   └── pkill -9 mihomo (确保停止)

2. 移除 systemd 服务
   ├── systemctl disable clashctl
   └── rm /etc/systemd/system/clashctl.service

3. 移除二进制
   └── rm -f /usr/local/bin/clashctl /usr/local/bin/clash-tui

4. 清理 Shell RC
   └── sed -i '/# clashctl START/,/# clashctl END/d' ~/.bashrc ~/.zshrc

5. 清理 cron
   └── crontab -l | grep -v clashsub | crontab -

6. 移除数据目录
   ├── rm -rf ~/.clashctl/
   └── rm -rf ~/.config/clashctl/

7. 验证 (残留检查)
   ├── 检查 mihomo 进程
   ├── 检查 clashctl 命令
   └── 检查 ~/.clashctl/ 目录
```

### 5.4 update.sh 流程

```
1. 停止内核
   └── clashctl stop

2. Git pull 最新代码 (如从 GitHub 安装)
   └── git -C /tmp/linux-cli-tui-clash pull

3. 重新编译 CLI
   └── go build -ldflags="-s -w" -o /usr/local/bin/clashctl

4. 重新编译 TUI (可选)
   └── cargo build --release --manifest-path tui/Cargo.toml

5. 检查内核更新
   └── 对比 .env 中的 VERSION_MIHOMO 与 GitHub latest release
       ├── 如有新版本: 下载替换
       └── 无更新: 跳过

6. 重启内核
   └── clashctl start
```

---

## 6. CLI 命令设计

### 6.1 命令总览

```
clashctl
├── start                    启动代理内核 + 设置环境变量
├── stop                     停止代理内核 + 清除环境变量
├── restart                  重启代理内核
├── status                   显示运行状态
├── log                      查看日志
│
├── proxy [on|off|status]    系统代理开关
│
├── tun [on|off|status]      TUN 模式开关
│
├── config                   配置管理
│   ├── view                 查看当前运行配置
│   ├── edit                 编辑 mixin.yaml
│   ├── merge                手动触发配置合并
│   └── validate             校验配置
│
├── sub                      订阅管理
│   ├── add <url> [--name]   添加订阅
│   ├── remove <id>          删除订阅
│   ├── list                 列出所有订阅
│   ├── use <id>             切换订阅
│   ├── update [id] [--auto] 更新订阅
│   └── log                  查看订阅操作日志
│
├── test                     延迟测试
│   ├── <url>                测试指定 URL
│   └── proxy <name>         测试指定代理节点
│
├── node                     代理节点管理
│   ├── list                 列出所有代理组和节点
│   ├── switch <group> <node>切换代理节点
│   └── delay [group]        测试代理组延迟
│
├── env                      输出代理环境变量 (用于 eval)
├── secret [show|new]        API 密钥管理
├── tui                      启动 TUI 仪表盘
├── upgrade-kernel           升级 Mihomo 内核
├── version                  显示版本
└── help                     帮助信息
```

### 6.2 详细命令规格

#### `clashctl start`
```
启动代理内核并设置系统代理环境变量。

用法:
  clashctl start [flags]

Flags:
  --no-proxy    启动内核但不设置系统代理
  --tun         同时启用 TUN 模式

示例:
  clashctl start                # 启动内核 + 设置系统代理
  clashctl start --no-proxy     # 仅启动内核
  clashctl start --tun          # 启动内核 + TUN 模式

输出:
  ✓ Mihomo v1.19.17 已启动
  ✓ 系统代理已设置 (HTTP: 127.0.0.1:7897, SOCKS5: 127.0.0.1:7897)
  ✓ 当前订阅: MySub (1)
```

#### `clashctl stop`
```
停止代理内核并清除系统代理环境变量。

用法:
  clashctl stop

输出:
  ✓ Mihomo 已停止
  ✓ 系统代理已清除
```

#### `clashctl status`
```
显示代理运行状态。

用法:
  clashctl status [--json]

Flags:
  --json      以 JSON 格式输出

输出:
  状态: ● 运行中  (PID: 12345, 运行时间: 2h 30m)
  内核: Mihomo v1.19.17
  模式: Rule (规则)
  端口: Mixed: 7897 | SOCKS5: 7897
  TUN:  ○ 未启用
  连接: 15 活跃 / 230 总连接
  流量: ↑ 1.2 MB/s  ↓ 3.5 MB/s  (总计: ↑ 250 MB ↓ 1.8 GB)
  内存: 45.2 MB / OS Limit: 8.0 GB (0.6%)
  当前订阅: MySub (1) — 更新于 2026-05-05 12:00
```

#### `clashctl node list`
```
列出所有代理组及其节点。

用法:
  clashctl node list [--group <name>] [--filter <text>]

Flags:
  --group     仅显示指定组
  --filter    按名称过滤

输出:
  ┌─────────────────────────────────────────────────────────────┐
  │ ♯ 代理节点                     类型     延迟        策略   │
  ├─────────────────────────────────────────────────────────────┤
  │ [♯ GLOBAL]                                                │
  │   ● 香港 01                   ss       45ms        URL    │
  │   ○ 日本 02                   vmess    89ms        URL    │
  │   ○ 新加坡 03                 trojan   120ms       URL    │
  │   ○ DIRECT                    direct   —           内置   │
  │ [♯ 流媒体]                                                │
  │   ● 香港 Netflix              ss       52ms        URL    │
  │   ○ 台湾 Disney+              trojan   95ms        URL    │
  └─────────────────────────────────────────────────────────────┘
  ● = 当前选中  ○ = 可用
```

#### `clashctl node switch`
```
切换代理组中的节点。

用法:
  clashctl node switch <group> <node>

示例:
  clashctl node switch GLOBAL "香港 01"
  clashctl node switch 流媒体 DIRECT

输出:
  ✓ GLOBAL → 香港 01  (延迟: 45ms)
```

#### `clashctl node delay`
```
测试代理组中所有节点的延迟。

用法:
  clashctl node delay [group] [--timeout 5000] [--url <url>]

Flags:
  --timeout   超时 (ms), 默认 5000
  --url       测试 URL, 默认 http://www.gstatic.com/generate_204

示例:
  clashctl node delay           # 测试所有组
  clashctl node delay GLOBAL    # 测试指定组
```

#### `clashctl sub add`
```
添加新订阅。

用法:
  clashctl sub add <url> [--name <name>]

Flags:
  --name      订阅名称 (默认从 HTTP header 或 URL 提取)

示例:
  clashctl sub add https://sub.example.com/api/v1?token=xxx --name "我的订阅"

输出:
  ↓ 正在下载订阅...
  ✓ 下载成功 (128 proxies, 15 groups, 45 rules)
  ✓ 已添加订阅: 我的订阅 (ID: 3)
  ✓ 已激活为当前订阅
  ✓ 代理已重新加载
```

#### `clashctl sub list`
```
列出所有订阅。

输出:
  ┌──────────────────────────────────────────────────────┐
  │ ID  名称           状态   更新时间          代理数   │
  ├──────────────────────────────────────────────────────┤
  │ ● 1  MySub         活跃   05-05 12:00       128     │
  │   2  Backup Server  就绪  05-04 08:30       96      │
  │   3  本地配置       就绪  05-03 15:00       45      │
  └──────────────────────────────────────────────────────┘
  ● = 当前激活
```

#### `clashctl proxy on`
```
设置系统代理环境变量。

用法:
  clashctl proxy on

输出:
  ✓ 系统代理已启用
    export http_proxy=http://127.0.0.1:7897
    export https_proxy=http://127.0.0.1:7897
    export all_proxy=socks5h://127.0.0.1:7897
```

#### `clashctl test`
```
测试 URL 或代理节点连通性。

用法:
  clashctl test <url>              # 直接测试 URL
  clashctl test proxy <name>       # 通过代理节点测试

示例:
  clashctl test https://www.google.com
  clashctl test proxy "香港 01"

输出:
  ✓ https://www.google.com — 245ms
```

### 6.3 颜色约定

| 颜色 | 用途 |
|------|------|
| 绿色 `●` | 运行中 / 成功 / 在线 |
| 黄色 `●` | 警告 / 等待中 |
| 红色 `●` | 已停止 / 错误 / 超时 |
| 灰色 `○` | 未激活 / 未启用 |
| 青色 | 高亮值 / URL |
| 蓝色 | 标题 / 表头 |

---

## 7. TUI 界面设计

### 7.1 颜色主题 (匹配 GUI 暗色方案)

基于 TUI-template 的 `theme.rs`，调整为匹配原 GUI 的配色：

```rust
pub const CLASH_THEME: Theme = Theme {
    // 背景层级 — 对应 GUI 暗色模式
    bg_outer:     Color::Rgb(15,  23,  42),  // #0F172A  (slate-900)
    bg:           Color::Rgb(17,  24,  39),  // #111827  (slate-800)
    surface:      Color::Rgb(30,  41,  59),  // #1E293B  (slate-700)
    border:       Color::Rgb(51,  65,  85),  // #334155  (slate-600)

    // 主色调 — 对应 GUI primary #3B82F6
    primary:      Color::Rgb(59,  130, 246), // #3B82F6  (blue-500)
    secondary:    Color::Rgb(139, 92,  246), // #8B5CF6  (violet-500)

    // 状态色 — 对应 GUI
    accent:       Color::Rgb(16,  185, 129), // #10B981  (emerald-500) 成功/在线
    warning:      Color::Rgb(245, 158, 11),  // #F59E0B  (amber-500)
    danger:       Color::Rgb(239, 68,  68),  // #EF4444  (red-500)

    // 文本 — 对应 GUI
    text:         Color::Rgb(249, 250, 251), // #F9FAFB  (slate-50)
    muted:        Color::Rgb(156, 163, 175), // #9CA3AF  (slate-400)

    // 图表/特殊
    sparkline_bg: Color::Rgb(30,  41,  59),  // #1E293B
};
```

### 7.2 键盘映射

```
全局键:
  q / Esc          退出 TUI
  Tab / →          下一个标签页
  Shift+Tab / ←    上一个标签页
  1 / 2 / 3 / 4 / 5  直接跳转到标签页
  j / k / ↓ / ↑    向下/向上导航
  g                跳转到顶部
  G                跳转到底部
  /                搜索/过滤
  r                刷新数据
  Enter             确认/展开
  Space             切换/选择
  h / ?            显示帮助面板

Tab 特定:
  Proxies Tab:
    s               切换排序方式 (延迟/名称)
    d               测试当前组延迟
    D               测试全部延迟
    p               切换代理模式 (Rule/Global/Direct)
    Enter           切换选中节点
  
  Subscriptions Tab:
    a               添加订阅
    u               更新当前/选中订阅
    U               更新全部订阅
    Enter           切换订阅
  
  Connections Tab:
    c               关闭选中连接
    C               关闭全部连接
  
  Logs Tab:
    f               切换日志级别过滤
    p               暂停/恢复日志
```

### 7.3 标签页布局设计

#### Tab 1: Overview (仪表盘)

```
┌──── Overview ──── Proxies ──── Subscriptions ──── Connections ──── Logs ────┐
│                                                                              │
│  ┌─ Status ───────────────────────────────────────────────────────────────┐ │
│  │  ● 运行中    Mihomo v1.19.17    Rule 模式    运行: 2h 30m              │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                                                              │
│  ┌──────────────────────┐  ┌──────────────────────┐  ┌────────────────────┐ │
│  │ Traffic              │  │ Current Proxy        │  │ Proxy Mode         │ │
│  │                      │  │                      │  │                    │ │
│  │  ↑ 1.2 MB/s          │  │  ♯ GLOBAL            │  │ [Rule] Global Dir  │ │
│  │  ↓ 3.5 MB/s          │  │  ● 香港 01 (45ms)    │  │                    │ │
│  │                      │  │  ♯ 流媒体            │  │ 切换模式: r/g/d    │ │
│  │  ▁▂▃▅▆▇█▇▆▅▃▂▁      │  │  ● 香港 Netflix(52ms)│  │                    │ │
│  │                      │  └──────────────────────┘  └────────────────────┘ │
│  │  Total: ↑250M ↓1.8G  │                                                   │
│  └──────────────────────┘  ┌──────────────────────────────────────────────┐ │
│                            │ System Info                                  │ │
│  ┌─ Connections ─────────┐ │ OS: Ubuntu 24.04  |  Arch: x86_64-v3        │ │
│  │ Active: 15            │ │ Memory: 45.2 MB / 8.0 GB (0.6%)             │ │
│  │ Total:  230           │ │ TUN: ○ Disabled  |  Auto-start: ✓ Enabled   │ │
│  └────────────────────────┘ └──────────────────────────────────────────────┘ │
│                                                                              │
│  ┌─ Subscription ─────────────────────────────────────────────────────────┐ │
│  │ ● MySub (128 proxies)  —  updated 2026-05-05 12:00  |  流量: 25GB/100GB│ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                                                              │
│  ?:Help  r:Refresh  q:Quit                                                   │
└──────────────────────────────────────────────────────────────────────────────┘
```

#### Tab 2: Proxies (代理管理)

```
┌──── Overview ──── Proxies ──── Subscriptions ──── Connections ──── Logs ────┐
│ 模式: [Rule] Global Direct  |  ● ● ●  |  搜索: _________  |  排序: 延迟 ▼   │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ♯ GLOBAL                                                             [测]  │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │ ● 香港 01                      ss         45ms  ████░░░░░░░          │   │
│  │ ○ 日本 02                      vmess      89ms  ████████░░░░         │   │
│  │ ○ 新加坡 03                    trojan    120ms  ████████████         │   │
│  │ ○ 美国 04                      vmess     190ms  ████████████████     │   │
│  │ ○ DIRECT                       direct       —                       │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│  ♯ 流媒体                                                             [测]  │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │ ● 香港 Netflix                  ss         52ms  ████░░░░░░░          │   │
│  │ ○ 台湾 Disney+                  trojan     95ms  ████████░░░░         │   │
│  │ ○ 日本 Abema                    vmess     130ms  ████████████         │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│  ♯ 香港节点 (来自 provider: HK-Pool)                                  [测]  │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │ ○ HK-01                         ss         38ms  ███░░░░░░░░░         │   │
│  │ ○ HK-02                         ss         42ms  ███░░░░░░░░░         │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│  Enter:选择  d:测延迟  D:测全部  p:切换模式  s:排序  /:搜索  q:退出        │
└──────────────────────────────────────────────────────────────────────────────┘
```

#### Tab 3: Subscriptions (订阅管理)

```
┌──── Overview ──── Proxies ──── Subscriptions ──── Connections ──── Logs ────┐
│ a:添加  u:更新  U:更新全部  Enter:切换  d:删除                               │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ● ID 1 │ MySub                                                             │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │ URL:    https://sub.example.com/api/v1?token=***                      │   │
│  │ 代理数: 128  │  更新时间: 2026-05-05 12:00  │  状态: ✓ 活跃          │   │
│  │ 流量:   25.3 GB / 100 GB  │  到期: 2026-06-01  │  自动更新: 12h   │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│     ID 2 │ Backup Server                                                    │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │ URL:    https://backup.example.com/sub                                │   │
│  │ 代理数: 96   │  更新时间: 2026-05-04 08:30  │  状态: 就绪            │   │
│  │ 流量:   未知  │  到期: 未知  │  自动更新: 24h                         │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│     ID 3 │ 本地配置                                                         │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │ 文件:   ~/.clashctl/resources/configs/local.yaml                      │   │
│  │ 代理数: 45   │  更新时间: 2026-05-03 15:00  │  状态: 就绪            │   │
│  │ 类型:   本地文件  │  自动更新: 关闭                                    │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│  a:添加  u:更新  U:更新全部  Enter:切换  d:删除  q:退出                     │
└──────────────────────────────────────────────────────────────────────────────┘
```

#### Tab 4: Connections (连接管理)

```
┌──── Overview ──── Proxies ──── Subscriptions ──── Connections ──── Logs ────┐
│ 活跃连接: 15  |  总连接: 230  |  ↑ 1.2M/s ↓ 3.5M/s  |  c:关闭  C:关闭全部 │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  [活跃]  [已关闭]                                                            │
│                                                                              │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │ Host                    Type      Chain               DL Speed   Time │   │
│  │──────────────────────────────────────────────────────────────────────│   │
│  │ api.github.com          HTTPS     GLOBAL → 香港 01    2.5 MB/s  12s  │   │
│  │ cdn.example.com         HTTPS     GLOBAL → 日本 02    1.8 MB/s  30s  │   │
│  │ 10.0.0.5:443            TCP       流媒体 → 香港 NF     450 KB/s  5m  │   │
│  │ youtube.com             QUIC      GLOBAL → DIRECT     3.2 MB/s  2m   │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│  /:搜索  c:关闭选中  C:关闭全部  Tab:切换活跃/已关闭  q:退出                │
└──────────────────────────────────────────────────────────────────────────────┘
```

#### Tab 5: Logs (日志)

```
┌──── Overview ──── Proxies ──── Subscriptions ──── Connections ──── Logs ────┐
│ 级别: [ALL] INFO WARN ERROR  |  搜索: _________  |  ↑↓顺序  |  ⏸ 暂停      │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  12:30:45  INFO  默认路径 192.168.1.1:443 匹配规则: Proxy                   │
│  12:30:45  INFO  TCP 连接 192.168.1.1:443 已建立                            │
│  12:30:44  INFO  代理组 [GLOBAL] 使用 [香港 01]                              │
│  12:30:44  WARN  [香港 02] 连接超时, 尝试下一个节点                           │
│  12:30:43  INFO  订阅更新: MySub 检查更新中...                               │
│  12:30:40  INFO  DNS 解析 example.com → 93.184.216.34                       │
│  12:30:39  INFO  [Rule] example.com → Proxy                                 │
│  12:30:38  ERROR [新加坡 03] TLS handshake failed                           │
│  12:30:37  INFO  TUN 设备启动: utun420, 192.168.255.1/30                     │
│                                                                              │
│  f:切换级别  /:搜索  p:暂停  ↑↓:排序  q:退出                                │
└──────────────────────────────────────────────────────────────────────────────┘
```

### 7.4 基于 TUI-template 的修改清单

| 文件 | 修改内容 |
|------|---------|
| `theme.rs` | 替换为 CLASH_THEME (匹配 GUI 配色) |
| `app.rs` | 新增 5 个 Tab 渲染函数，替换原有示例数据 |
| `api.rs` | 替换为 Mihomo REST API 客户端 (GET /proxies, /connections, /version, /logs, PUT /proxies/{group}) |
| `widgets/tab_bar.rs` | 修改为 5 个标签页: Overview, Proxies, Subscriptions, Connections, Logs |
| `widgets/card.rs` | 保留 (边框卡片) |
| `widgets/table.rs` | 保留并增强 (添加高亮行, 排序指示器) |
| `widgets/status_dot.rs` | 保留 (运行状态 breathing 灯) |
| `widgets/gauge.rs` | **新增**: 水平进度条 (流量使用 / 延迟柱状图) |
| `widgets/sparkline.rs` | **新增**: 实时流量趋势图 (小型 ASCII 图) |
| `main.rs` | 添加 Proxies/Subscriptions/Connections/Logs 专属键位映射 |
| `config.rs` | 修改为加载 `~/.clashctl/.env` |
| `window.rs` | 保留 (窗口管理) |
| `background.rs` | 保留粒子系统 (或替换为静态) |

### 7.5 数据刷新策略

| 数据 | 刷新频率 | 方式 |
|------|---------|------|
| 流量速度 | 1s | HTTP GET /traffic (或解析 /proxies) |
| 代理列表 | 5s | HTTP GET /proxies |
| 连接列表 | 2s | HTTP GET /connections |
| 日志 | 实时 | 读取日志文件 tail |
| 订阅列表 | 按需 | HTT P/api on tab switch |
| 系统信息 | 10s | 系统调用 |

---

## 8. 配置管理设计

### 8.1 配置文件层级

```
1. 订阅源配置 (config.yaml)          ← 远程下载, 只读
   │
2. 用户补丁 (mixin.yaml)             ← 用户编辑, 最高优先级
   │
   ▼ yq eval-all '. as $item ireduce({}; . * $item)'
3. 运行时配置 (runtime.yaml)         ← 内核加载此文件
```

### 8.2 mixin.yaml 模板

```yaml
# ================================
# Linux CLI&TUI Clash — mixin.yaml
# 此文件的配置会覆盖/合并到订阅配置中
# ================================

# —— 端口 ——
mixed-port: 7897
socks-port: 7898
port: 7899

# —— 模式 ——
mode: rule
log-level: info
ipv6: false
allow-lan: false

# —— API ——
external-controller: 127.0.0.1:9090
secret: ""

# —— TUN ——
tun:
  enable: false
  stack: gvisor
  auto-route: true
  auto-detect-interface: true
  dns-hijack:
    - any:53

# —— DNS ——
dns:
  enable: true
  enhanced-mode: fake-ip
  nameserver:
    - 223.5.5.5
    - 119.29.29.29
  fallback:
    - 8.8.8.8
    - 1.1.1.1

# —— 规则前缀 (插入到订阅规则之前) ——
rules:
  prefix:
    - DOMAIN-SUFFIX,local,DIRECT
    - IP-CIDR,10.0.0.0/8,DIRECT

# —— 规则后缀 (插入到订阅规则之后) ——
rules:
  suffix:
    - MATCH,Proxy

# —— 代理组注入 ——
proxy-groups:
  inject:
    - group: "GLOBAL"
      proxies:
        - DIRECT

# —— 代理节点覆盖 ——
proxies:
  override:
    - name: "自定义节点"
      type: ss
      server: example.com
      port: 8388
      cipher: aes-256-gcm
      password: "password"
```

### 8.3 配置命令

```
clashctl config view              # 查看合并后的运行时配置
clashctl config edit              # 用 $EDITOR 打开 mixin.yaml
clashctl config merge             # 手动触发合并
clashctl config validate          # 使用 mihomo -t 校验配置
```

---

## 9. 内核管理设计

### 9.1 内核启动流程

```
1. 检查端口占用
   └── ss -tunl | grep 9090 → 如果占用, 报错退出

2. 合并配置
   └── yq eval-all '. as $item ireduce({}; . * $item)' \
         config.yaml mixin.yaml > runtime.yaml

3. 端口冲突检测
   └── 从 runtime.yaml 读取端口, 如有冲突生成随机端口写入 mixin.yaml

4. 配置校验
   └── mihomo -d resources/ -f runtime.yaml -t

5. 启动内核
   ├── systemd (如已安装 service):
   │   └── systemctl start clashctl
   └── nohup (fallback):
       └── nohup ~/.clashctl/bin/mihomo \
             -d ~/.clashctl/resources/ \
             -f ~/.clashctl/resources/runtime.yaml \
             > ~/.clashctl/logs/mihomo.log 2>&1 &

6. 等待就绪
   └── 轮询 127.0.0.1:9090 (最多 15s, 每 500ms)
```

### 9.2 systemd Service 模板

```ini
[Unit]
Description=Clashctl Proxy Service (Mihomo)
After=network.target NetworkManager.service systemd-networkd.service iwd.service

[Service]
Type=simple
User=%u
LimitNPROC=500
LimitNOFILE=1000000
CapabilityBoundingSet=CAP_NET_ADMIN CAP_NET_RAW CAP_NET_BIND_SERVICE
AmbientCapabilities=CAP_NET_ADMIN CAP_NET_RAW CAP_NET_BIND_SERVICE
Restart=always
RestartSec=3
ExecStartPre=/usr/bin/sleep 1s
ExecStart=__KERNEL_PATH__ -d __RESOURCES_DIR__ -f __RESOURCES_DIR__/runtime.yaml
ExecStop=/bin/kill -SIGTERM $MAINPID
StandardOutput=append:__LOG_DIR__/mihomo.log
StandardError=append:__LOG_DIR__/mihomo.log

[Install]
WantedBy=multi-user.target
```

### 9.3 内核升级

```
clashctl upgrade-kernel

1. 检查当前版本 (GET /version)
2. 获取 GitHub latest release (MetaCubeX/mihomo)
3. 对比版本
4. 下载新内核 → ~/.clashctl/bin/mihomo.new
5. 停止内核
6. mv mihomo.new mihomo
7. 重启内核
```

---

## 10. 订阅管理设计

### 10.1 订阅索引 (`profiles.yaml`)

```yaml
use: 1
profiles:
  - id: 1
    path: ~/.clashctl/resources/profiles/1.yaml
    url: https://sub.example.com/api/v1?token=xxx
    name: MySub
    updated: 1714881600
    interval: 43200
    user_agent: clash-verge/v2.4.8
    extra:
      upload: 0
      download: 27130351616
      total: 107374182400
      expire: 1717200000
  - id: 2
    path: ~/.clashctl/resources/profiles/2.yaml
    url: https://backup.example.com/sub
    name: Backup Server
    updated: 1714800000
    interval: 86400
```

### 10.2 订阅下载流程

```
1. curl 下载 → temp.yaml
   ├── 设置 User-Agent
   ├── 超时 30s
   └── 支持 HTTP 307 redirect

2. 提取 HTTP 响应头:
   ├── subscription-userinfo: upload/download/total/expire
   ├── content-disposition: 文件名
   ├── profile-update-interval: 自动更新间隔
   └── profile-web-page-url: 订阅主页

3. 验证配置:
   └── mihomo -d resources/ -f temp.yaml -t

4. 失败回退 — Subconverter:
   ├── 启动本地 subconverter (如已安装)
   ├── POST /sub?target=clash&url=<encoded>
   └── 重新验证

5. 保存文件
   └── mv temp.yaml profiles/{id}.yaml

6. 更新索引
   └── 更新 profiles.yaml 中的 updated/extra/interval 字段

7. 如果是当前激活订阅 → 触发配置合并 + 内核重载
```

### 10.3 自动更新 (cron)

```
clashctl sub update --auto

→ 添加 crontab:
  0 */12 * * * /usr/local/bin/clashctl sub update --cron
```

---

## 11. 代理与网络设计

### 11.1 系统代理设置

通过 Shell RC 注入实现，写入 `.bashrc` / `.zshrc`:

```bash
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
```

### 11.2 TUN 模式

```
clashctl tun on

1. 检查权限 (需要 sudo / root)
2. 设置 tun.enable=true 到 mixin.yaml
3. 合并配置
4. 重启内核 (systemctl restart / nohup 重启)
5. 验证 TUN 设备 (ip link show | grep utun)
```

### 11.3 代理模式切换

```
clashctl node switch GLOBAL "香港 01"
→ PUT /proxies/GLOBAL  {"name":"香港 01"}
→ clashctl test proxy "香港 01"  (验证延迟)
```

### 11.4 代理环境变量输出

```
$ eval $(clashctl env)
→ 在当前 shell 中设置代理变量 (不修改 RC 文件)
```

---

## 12. 实施路线图

### Phase 1: 基础设施 (Week 1)

- [x] `install.sh` — 完整安装脚本
- [x] `uninstall.sh` — 完整卸载脚本
- [x] `scripts/preflight.sh` — 资源下载
- [x] Go CLI 框架 (cobra 根命令 + 基础命令)
- [x] 内核管理 (start/stop/restart/status)
- [x] 配置合并 (yq 集成)
- [x] systemd service 模板

### Phase 2: CLI 核心功能 (Week 2)

- [ ] `clashctl sub add/list/remove/use/update` — 订阅管理
- [ ] `clashctl node list/switch/delay` — 代理节点管理
- [ ] `clashctl proxy on/off` — 系统代理
- [ ] `clashctl tun on/off` — TUN 模式
- [ ] `clashctl config view/edit/merge` — 配置管理
- [ ] `clashctl log` — 日志查看
- [ ] `clashctl test` — 延迟测试
- [ ] Shell RC 集成 + Fish 补全

### Phase 3: TUI 仪表盘 (Week 3-4)

- [ ] Tab 1: Overview (状态/流量/代理/系统信息)
- [ ] Tab 2: Proxies (代理组列表 + 切换 + 延迟)
- [ ] Tab 3: Subscriptions (订阅管理)
- [ ] Tab 4: Connections (连接列表)
- [ ] Tab 5: Logs (日志查看器)
- [ ] 自定义 widget: gauge (流量条), sparkline (趋势图)
- [ ] API 客户端与 Mihomo 通信
- [ ] 键盘映射完善

### Phase 4: 收尾 (Week 4)

- [ ] `update.sh` — 升级脚本
- [ ] `clashctl upgrade-kernel` — 内核升级
- [ ] 子订阅自动更新 (cron)
- [ ] 预编译二进制 + GitHub Release CI
- [ ] README.md 文档
- [ ] 端到端测试 (Ubuntu 22.04 / 24.04)
- [ ] 性能优化 (TUI 渲染帧率, API 请求合并)

---

## 附录 A: 与原版 Clash Verge Rev 的对应关系

| 原版模块 | 新实现 | 说明 |
|---------|--------|------|
| `src-tauri/src/cmd/clash.rs` | `cmd/clashctl/start.go, stop.go` | 内核管理 |
| `src-tauri/src/cmd/profile.rs` | `cmd/clashctl/sub*.go` | 订阅管理 |
| `src-tauri/src/cmd/proxy.rs` | `cmd/clashctl/node.go, proxy.go` | 代理/节点 |
| `src-tauri/src/cmd/system.rs` | `cmd/clashctl/proxy.go` | 系统代理 |
| `src-tauri/src/core/manager/` | `internal/kernel/service.go` | 内核生命周期 |
| `src-tauri/src/config/verge.rs` | `.env` + `mixin.yaml` | 配置 |
| `src-tauri/src/enhance/mod.rs` | `internal/config/merge.go` | 配置合并 |
| `src-tauri/src/core/sysopt.rs` | RC 注入 + systemd | 系统代理设置 |
| `src-tauri/src/core/timer.rs` | cron | 定时更新 |
| `src-tauri/src/core/tray/` | N/A (终端无托盘) | — |
| `src/services/delay.ts` | `cmd/clashctl/test.go` | 延迟测试 |
| `src/services/cmds.ts` | Go CLI 各命令 | 前端命令桥接 |
| `src/pages/home.tsx` | `tui/src/app.rs` Tab 1 | 仪表盘 |
| `src/pages/proxies.tsx` | `tui/src/app.rs` Tab 2 | 代理管理 |
| `src/pages/profiles.tsx` | `tui/src/app.rs` Tab 3 | 订阅管理 |
| `src/pages/connections.tsx` | `tui/src/app.rs` Tab 4 | 连接管理 |
| `src/pages/logs.tsx` | `tui/src/app.rs` Tab 5 | 日志查看 |

## 附录 B: 与原版配色对比

| 用途 | GUI (REPORT.md) | TUI (本设计) |
|------|----------------|-------------|
| 主背景 | #111827 | #111827 (一致) |
| 外背景 | #0F172A | #0F172A (一致) |
| 卡片面 | #1E293B | #1E293B (一致) |
| 主色 | #3B82F6 (blue-500) | #3B82F6 (一致) |
| 次色 | #8B5CF6 (violet-500) | #8B5CF6 (一致) |
| 成功/在线 | #10B981 (emerald-500) | #10B981 (一致) |
| 警告 | #F59E0B (amber-500) | #F59E0B (一致) |
| 错误/超时 | #EF4444 (red-500) | #EF4444 (一致) |
| 主文本 | #F9FAFB | #F9FAFB (一致) |
| 次要文本 | #9CA3AF | #9CA3AF (一致) |

---

> **文档结束** — 本设计文档基于 Clash Verge Rev 的 REPORT.md，通过合理裁剪，重新设计为面向 Ubuntu Linux 的 CLI+TUI 代理管理工具。实现时可参考 `TUI-template/` (Rust TUI 模板) 和 `clash-for-linux-install-master/` (Bash+Go 参考实现)。
