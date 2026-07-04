# Clash for Linux

Linux 命令行优先的 Mihomo/Clash 管理工具，包含 Go 编写的 `clashctl`、Rust/Ratatui 编写的终端 TUI `clash-tui`，以及面向 release 产物的安装、更新、卸载链路。

English: Linux-first Mihomo/Clash management toolkit with a Go CLI, a Ratatui terminal UI, and release-based install/update/uninstall flows.

## 项目定位

本项目管理 Mihomo，不实现代理内核，也不 fork Mihomo。目标是让 Linux/SSH/终端用户通过 CLI 和 TUI 完成订阅、代理、连接、流量、TUN、日志、设置、更新和卸载等常见操作。

English: This project manages Mihomo instead of implementing or forking a proxy kernel. It targets Linux terminal and SSH users.

主要组件：

- `clashctl`：核心 CLI，负责安装后生命周期、配置合并、订阅、节点、TUN、代理环境、日志、诊断、流量统计、更新和卸载。
- `clash-tui`：终端 TUI，覆盖常用 CLI 功能，支持鼠标、键盘、分页、弹窗、命令输出窗口和中英双语界面。
- `install.sh`：首次安装入口，支持 release artifact、checksum、镜像 fallback、TUI 安装和 systemd service。
- `clashctl self update`：从 GitHub release 产物更新 CLI/TUI。
- `clashctl self uninstall`：交互式卸载，支持分项保留订阅、配置、geodata、流量、日志、TUI 设置、内核工具和 completion。
- `scripts/smoke`：CI 和本地 smoke 检查。

当前没有 Tauri 桌面 GUI。`clash-tui` 是终端应用。

English: There is no Tauri desktop app. `clash-tui` is a terminal UI.

## 当前状态

当前验证基线是 `v0.2.0-rc.16`。该 RC 已完成以下高优先级验证：

- GitHub Actions：`CI`、`Install Smoke`、`Release` 均通过。
- Release 产物：`clash-for-linux-amd64.tar.gz`、`clash-for-linux-arm64.tar.gz`、`SHA256SUMS` 已生成。
- 裸机安装：干净 Linux 环境通过 release artifact 安装，包含 Mihomo、yq、geodata、CLI、TUI、systemd service。
- 镜像 fallback：GitHub 直连失败时可通过 `gh-proxy.org` 镜像完成下载。
- systemd 与内核：`clashctl start/status` 真实启动验证通过。
- TUI 兼容性：release TUI 已改为 musl 构建，避免旧 glibc 机器上 `GLIBC_2.39 not found`。
- TUN：SSH 场景下 route guard 会保护 SSH 客户端、隧道端点和隧道路由。
- 卸载：检测当前 shell proxy 变量，并提示 `unset` 清理命令。

当前分支相对 `v0.2.0-rc.16` 还有待发布改动：卸载时的数据保留问题已拆分为订阅、配置、geodata、流量、日志、TUI 设置等独立选择项；安装资产会在卸载时清理，不再因为保留用户数据而留下 root-owned `scripts/`。

English: `v0.2.0-rc.16` is the current validated release candidate. The branch also contains post-rc.16 uninstall UX improvements that need the next RC or stable tag before installed users receive them.

正式版前仍建议补充：

- 更多发行版真机覆盖。
- GNOME/KDE 桌面代理真实行为验证。
- 最后一版 RC 通过后，定稿 release notes。

## 安装

推荐使用明确 tag 安装，不建议在 RC 阶段使用 `latest`。

English: Use an explicit tag for RC installs. Do not rely on `latest` during RC validation.

直连 GitHub：

```bash
curl -fsSL https://raw.githubusercontent.com/yequdesu/clash-for-linux/v0.2.0-rc.16/install.sh | bash -s -- --with-tui --tag v0.2.0-rc.16
```

使用镜像：

```bash
curl -fsSL https://gh-proxy.org/https://raw.githubusercontent.com/yequdesu/clash-for-linux/v0.2.0-rc.16/install.sh | bash -s -- --with-tui --tag v0.2.0-rc.16
```

已安装但需要强制重装：

```bash
curl -fsSL https://gh-proxy.org/https://raw.githubusercontent.com/yequdesu/clash-for-linux/v0.2.0-rc.16/install.sh | bash -s -- --with-tui --force --tag v0.2.0-rc.16
```

源码开发安装：

```bash
bash install.sh --with-tui
```

## 常用流程

添加订阅并启动：

```bash
clashctl sub add https://your-subscription-url
clashctl start
clashctl status
clashctl doctor
```

在当前 shell 启用代理环境：

```bash
eval "$(clashctl env)"
```

关闭当前 shell 的代理环境：

```bash
eval "$(clashctl proxy off)"
```

打开 TUI：

```bash
clashctl tui
```

English: Add a subscription, start the kernel, optionally export proxy environment variables, and launch the TUI with `clashctl tui`.

## TUN 与 SSH

TUN 会修改路由，SSH 环境下存在断连风险。本项目会在风险操作前检测并保护管理路由，包括当前 SSH 客户端、隧道端点、连接路由和用户通过 `CLASH_TUN_PROTECT_ROUTES` 指定的自定义 CIDR。

English: TUN changes routes and can break SSH sessions. `clashctl` detects and protects management routes before enabling TUN.

开启 TUN：

```bash
sudo clashctl tun on
```

关闭 TUN：

```bash
sudo clashctl tun off
```

自定义保护路由：

```bash
CLASH_TUN_PROTECT_ROUTES=10.0.0.0/24,192.168.1.0/24 sudo clashctl tun on
```

## 更新

已安装新版 CLI 后可使用：

```bash
CLASHCTL_GITHUB_DIRECT=false CLASHCTL_GITHUB_MIRRORS="https://gh-proxy.org/" clashctl self update --tag v0.2.0-rc.16
```

如果机器上没有 `/usr/local/bin/clashctl`，不能使用 `self update`，应重新执行安装命令。

English: `self update` requires an existing `clashctl`. If the binary is missing, reinstall from `install.sh`.

## 卸载

交互式卸载：

```bash
clashctl self uninstall
```

卸载会停止内核、移除 systemd service、completion、shell rc 片段、cron 条目和系统二进制。当前分支的卸载逻辑会分别询问是否保留：

- 订阅和导入配置。
- 本地配置、mixin 和 `.env`。
- geodata：`Country.mmdb`、`geosite.dat`、`geoip.dat`。
- 持久化流量统计。
- 日志。
- TUI 设置。
- `mihomo` 和 `yq`。
- shell completion。

即使保留用户数据，卸载也会清理安装资产，例如 `bin/`、`runtime/`、`scripts/`、`install-state.json`。

如果当前 shell 仍有 `eval "$(clashctl env)"` 设置过的代理变量，卸载程序无法修改父 shell。卸载会提示你运行：

```bash
unset http_proxy HTTP_PROXY https_proxy HTTPS_PROXY all_proxy ALL_PROXY no_proxy NO_PROXY
```

English: Uninstall is interactive and asks which data to keep. Proxy environment variables exported in the parent shell must be unset manually or by opening a new shell.

## 自动补全

安装当前 shell 的 completion：

```bash
clashctl completion install
```

卸载 completion：

```bash
clashctl completion uninstall
```

也可以只打印脚本：

```bash
clashctl completion bash
clashctl completion zsh
clashctl completion fish
```

## 本地验证

Go：

```bash
go test ./...
go vet ./...
GOOS=linux GOARCH=amd64 go build ./cmd/clashctl
```

Rust TUI：

```bash
cd tui
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

脚本：

```bash
bash -n install.sh update.sh uninstall.sh install_tui.sh scripts/preflight.sh scripts/init/nohup.sh scripts/init/systemd.sh scripts/smoke/*.sh
bash scripts/smoke/static_safety.sh
bash scripts/smoke/nohup_pid_safety.sh
```

Linux 进程级 CLI 退出码 smoke：

```bash
bash scripts/smoke/cli_exit_codes.sh
```

## 文档

当前有效文档：

- [文档索引](docs/README.md)
- [外部验证清单](docs/EXTERNAL_VALIDATION_CHECKLIST.md)
- [安装 smoke 验证](docs/INSTALL_SMOKE.md)

历史规划、审计和重构设计已移动到 [docs/archive](docs/archive)。归档文档只作为历史背景，不代表当前项目状态。

English: Current docs are listed in `docs/README.md`; historical plans and audits are archived under `docs/archive`.

## License

GPL-3.0
