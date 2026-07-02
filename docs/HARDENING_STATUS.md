# Clash for Linux Hardening Status

本文档记录当前硬化工作的可审查状态。`PROJECT_HARDENING_SPEC.md` 定义目标和验收标准；本文档只记录当前工作树已经实现、已经本地验证、仍需外部验证或后续处理的事实。

更新时间：2026-07-02

## 状态定义

- `DONE_LOCAL`：代码或脚本已实现，并已通过本地自动化验证。
- `EXTERNAL_REQUIRED`：代码已具备验证入口，但必须在真实 Linux、systemd、桌面会话、GitHub Actions 或发布环境中验证。
- `PARTIAL_LOCAL`：已有实现和部分本地测试，但还不能证明完整满足规格。
- `NOT_STARTED`：规格中要求存在，但当前不属于本轮已完成工作。

## 本地验证快照

本地已通过：

```bash
go test ./cmd/clashctl -run "TestCLIProcessExitCodes|TestSwitchSubscription|TestHandleSubUpdateError|TestShowHelpAndExit|TestConfigCommandHelpers|TestDesktop|TestProxy|TestSecret|TestNode" -count=1 -v
go test ./...
go vet ./...
GOOS=linux GOARCH=amd64 go build ./cmd/clashctl

cd tui
cargo fmt --check
cargo test
cargo clippy -- -D warnings

git diff --check
bash -n install.sh update.sh uninstall.sh install_tui.sh scripts/preflight.sh scripts/init/nohup.sh scripts/init/systemd.sh scripts/smoke/distro_install.sh scripts/smoke/release_install.sh scripts/smoke/systemd_service.sh scripts/smoke/static_safety.sh scripts/smoke/nohup_pid_safety.sh scripts/smoke/cli_exit_codes.sh
bash scripts/smoke/static_safety.sh
bash scripts/smoke/nohup_pid_safety.sh
```

本地不能证明：

- `GOOS=linux GOARCH=amd64 go test ./...` 的真实 Linux 执行结果。
- `systemd` unit 生命周期。
- 真实 Mihomo、yq、geodata 下载、安装、启动和回滚。
- 真实 GNOME/KDE/XFCE 桌面代理行为。
- GitHub Actions、release artifact、SHA256SUMS 和 tarball 安装链路。

## WSL 子集验证快照

执行日期：2026-07-02

环境事实：

- WSL 发行版：Ubuntu 26.04 LTS，WSL2，kernel `6.18.33.1-microsoft-standard-WSL2`。
- `systemctl is-system-running` 返回 `running`。
- 非交互 `sudo -n true` 不可用，因此未执行 root 安装、卸载、systemd unit 写入或真实 TUN 开关。
- WSL 内没有 `go`，因此未执行真实 Linux `go test ./...`、`go vet ./...` 或 `scripts/smoke/cli_exit_codes.sh`。
- WSL 内 `cargo clippy` 未安装，因此未执行 WSL 侧 clippy。

WSL 已通过：

```bash
bash -n install.sh update.sh uninstall.sh install_tui.sh scripts/preflight.sh scripts/init/nohup.sh scripts/init/systemd.sh scripts/smoke/distro_install.sh scripts/smoke/release_install.sh scripts/smoke/systemd_service.sh scripts/smoke/static_safety.sh scripts/smoke/nohup_pid_safety.sh scripts/smoke/cli_exit_codes.sh
bash scripts/smoke/static_safety.sh
bash scripts/smoke/nohup_pid_safety.sh

cd tui
cargo fmt --check
cargo test
```

WSL 只读探测结果：

- `systemctl list-unit-files clashctl.service` 未发现 unit，符合未安装状态。
- `/dev/net/tun` 存在。
- `ip -d -j link show` 可返回结构化 link JSON；当前只有 `lo` 和 `eth0`，没有真实 Mihomo TUN 设备。
- `git diff --check` 在 WSL 返回 0，但提示若干既有跟踪文件未来会从 CRLF 转为 LF；Windows 侧 `git diff --check` 返回 0。

结论：

- WSL 证明了 shell 语法、静态安全、nohup pid 归属 smoke、Rust TUI 格式和 Rust 单元测试在 Linux 用户态下可通过。
- WSL 没有证明 Go Linux 测试、真实 root 安装、真实 systemd unit 生命周期、真实 Mihomo、真实 TUN 路由、桌面代理、GitHub Actions 和 release artifact。

## 真实 Debian SSH/TUN 事故快照

执行日期：2026-07-02

环境事实：

- 真机主机名：`YeQuDesuDebian`。
- 真机局域网地址：`192.168.1.23/24`，SSH 客户端为 `192.168.1.20`。
- 事故发生时 runtime 中 `tun.enable=true`、`tun.auto-route=true`、`tun.strict-route=true`、设备名 `SakuraiTunnel`。
- TUN 开启后公网路由进入 table 2022：`default via 198.18.0.2 dev SakuraiTunnel`。
- 当前 SSH 回程路由在恢复后为：`192.168.1.20 dev wlp5s0 src 192.168.1.23`。
- `sudo clashctl tun off` 曾失败为 `clashctl not installed. Run install.sh first.`，实际原因不是二进制缺失，而是 sudo 下 `HOME=/root` 且缺少安装 marker，root 进程把 base dir 误判为 `/root/clashctl`。
- 机器上存在 `/etc/systemd/system/clashctl.service`，但当时 `systemctl is-active clashctl` 为 inactive，同时 raw/nohup `mihomo` 进程正在运行。

已执行恢复：

```bash
clashctl stop
sudo env CLASH_BASE_DIR=/home/yequdesu/.clashctl clashctl tun off
clashctl start
```

恢复后状态：

- `clashctl status` 显示 `Tun status: disabled`。
- `ip rule show` 只剩默认规则。
- `ip route get 39.156.66.10` 回到 `via 192.168.1.1 dev wlp5s0`。

本轮代码修复：

- `internal/config/env.go`：sudo/root 场景下优先解析 `SUDO_USER` 的真实安装目录；没有 marker 时回退到该用户已有的 `~/clashctl` 或 `~/.clashctl`，避免误判 `/root/clashctl`。
- `internal/kernel/service.go`：systemd unit 存在但 inactive 时，如果发现 raw/nohup 受管进程，`IsRunning` 和 `Stop` 不再误判。
- `cmd/clashctl/ssh_tun_guard.go`：`start`、`restart`、`tun on`、`sub use`、`upgrade-kernel` 在 SSH 会话中遇到 TUN 自动路由时优先保护当前 SSH 客户端路由；同网段直连路由放行，root/sudo 场景下自动写入 priority 8000、table 2021 的 bypass 规则，无法保护时才失败；`--allow-ssh-tun-risk` 或 `CLASH_ALLOW_SSH_TUN_RISK=1` 作为显式风险覆盖。
- `clashctl doctor` 新增 SSH+TUN 风险提示和 systemd inactive/raw running 状态提示。

已用临时新二进制验证：

- 将当前工作树交叉构建出的 Linux amd64 `clashctl` 临时上传到 `/tmp/clashctl-codex`，未替换正式 `/usr/local/bin/clashctl`。
- 普通用户 `/tmp/clashctl-codex status` 可识别当前 raw/nohup 内核 pid `2965170` 和 `Tun status: disabled`。
- `sudo /tmp/clashctl-codex status` 不再报 `clashctl not installed`，同样识别 `/home/yequdesu/.clashctl` 下的运行状态。
- `sudo /tmp/clashctl-codex doctor` 显示 base dir 为 `/home/yequdesu/.clashctl`，并提示 `service state: kernel pid 2965170 running outside active systemd unit`。

仍需真机复测：

- 安装新构建后的 `sudo clashctl tun off` 是否能直接命中 `/home/yequdesu/.clashctl`。
- SSH 环境下 `clashctl start` 遇到 `tun.auto-route=true` 是否在 stop/start 前保护当前 SSH 客户端路由，并保留当前连接。
- `--allow-ssh-tun-risk` 显式覆盖路径是否可用。
- root/systemd 与普通用户 raw 模式混用时，`stop/restart/tun off` 是否一致收敛到单一受管进程。

## P0 状态矩阵

| 项目 | 状态 | 本地证据 | 仍需验证 |
| --- | --- | --- | --- |
| P0-1 统一 systemd 服务名和服务管理 | PARTIAL_LOCAL | `internal/config/env.go`、`internal/kernel/service.go`、`cmd/clashctl/log.go` 使用 `ServiceName`；systemd inactive/raw running 误判已有本地修复；`go test ./...` 通过 | 真实 systemd 上验证 `install.sh`、`clashctl start/status/log/stop` 与 `systemctl status clashctl` 一致 |
| P0-2 安全默认 API 暴露 | DONE_LOCAL | `resources/mixin.yaml` 默认 loopback；secret 隐藏和 `secret show` 路径有测试；`doctor` 会报告空 secret | 安装脚本真实生成 secret 后，在 Linux 上验证 runtime 与 API auth |
| P0-3 内核 API 客户端 | DONE_LOCAL | API path escaping、JSON body、error body 截断均有 Go/Rust 测试；`go test ./...`、`cargo test` 通过 | 真实 Mihomo API 兼容性 |
| P0-4 订阅更新与 profile 一致性 | DONE_LOCAL | add/import/update/use/remove 核心流程可返回错误；文件锁、原子写、快照回滚测试通过；`sub update` 和 `sub use` 失败退出码已修；`TestCLIProcessExitCodes` 覆盖进程级失败退出码 | 真实订阅 URL、subconverter、active profile 切换后内核启动 |
| P0-5 原子配置合并与回滚 | DONE_LOCAL | runtime 临时文件、AST 更新、配置 set-* 回滚测试通过 | 使用真实 Mihomo `-t` 校验复杂订阅配置 |
| P0-6 内核升级可验证和可回滚 | DONE_LOCAL | checksum 解析、gzip、install-state 写入失败回滚等测试通过 | 真实 MetaCubeX release asset、SHA256SUMS、升级后启动 |
| P0-7 代理环境命令语义 | DONE_LOCAL | `proxy on/off` 输出 shell 脚本；desktop on/off 缺工具或设置失败 fatal；命令构造测试通过 | 真实桌面会话代理设置 |
| P0-8 CI 与最小测试集 | PARTIAL_LOCAL | `.github/workflows`、smoke 脚本、CLI exit-code smoke、Go/Rust/脚本本地检查存在并通过 | GitHub Actions 实际运行结果 |

## P1 状态矩阵

| 项目 | 状态 | 本地证据 | 仍需验证 |
| --- | --- | --- | --- |
| P1-1 结构化 RuntimeInfo | DONE_LOCAL | CLI/TUI 共享结构化解析语义；IPv4/IPv6/unspecified listener 测试通过 | 真实配置迁移场景 |
| P1-2 `clashctl doctor` | PARTIAL_LOCAL | 安装状态、权限、secret、API、DNS、TUN、geodata 等检查已有测试 | 真实 Linux、systemd、API auth、TUN 设备 |
| P1-3 TUN 模式可靠化 | PARTIAL_LOCAL | YAML 写入、快照回滚、启动失败回滚测试通过；sudo base-dir 解析、SSH/TUN 路由保护、doctor 风险提示已有本地测试 | root/capability、`/dev/net/tun`、`ip link`、真实路由行为、SSH 下 route bypass |
| P1-4 订阅转换器管理 | PARTIAL_LOCAL | subconverter 端口选择、pref 写入/恢复有测试 | 真实 subconverter 二进制和订阅转换结果 |
| P1-5 日志统一 | PARTIAL_LOCAL | CLI 统一读取 `cfg.LogFile()`、legacy log 和 `journalctl -u cfg.ServiceName`；tail 失败 fatal | 真实 systemd journal 和 nohup 日志路径 |
| P1-6 安装脚本幂等和可审计 | PARTIAL_LOCAL | shell 语法、static smoke、release smoke 脚本存在 | root 权限真实安装、重复安装、卸载后残留检查 |
| P1-7 本地残留收尾 | PARTIAL_LOCAL | `status` profiles 错误会 warning；`sub` 审计日志写入失败可见且 cron 成功路径严格；`sub list` 使用 YAML 结构化统计代理数；TUN 检测优先 `ip tuntap`/`ip -d -j link`；`doctor` 退出路径统一到 `exitProcess`；新增对应 Go 测试，`go test ./...`、`go vet ./...` 通过 | TUN 检测仍需真实 Linux 设备和 Mihomo 启动行为验证 |

## P2 状态矩阵

| 项目 | 状态 | 本地证据 | 仍需验证 |
| --- | --- | --- | --- |
| P2-1 Ratatui 完整终端控制面 | PARTIAL_LOCAL | 当前只覆盖 Overview/Proxies/Subscriptions/Connections/Logs 基础能力；TUI 结构化读取 runtime/profiles；调用 `clashctl` 子命令时传递安装环境；22 个 Rust 测试通过 | 仍需覆盖全部 `clashctl` 日常命令、清晰分页、action registry、鼠标 hitbox、所有元素点击/滚轮、真实 Mihomo API 和终端交互 |
| P2-2 Release 工程 | PARTIAL_LOCAL | release workflow 和 release install smoke 存在 | GitHub release artifact、SHA256SUMS、tarball 安装 |
| P2-3 配置模板与规则管理 | PARTIAL_LOCAL | geodata staging 替换、install-state 记录和回滚测试通过 | 真实 geodata 下载源和网络失败场景 |

## Ratatui 覆盖审计

当前代码事实：

- Go CLI 命令面包含生命周期、状态、日志、proxy、desktop proxy、TUN、secret、upgrade、upgrade-kernel、config、sub、node、env、test、doctor、geodata、version。
- Ratatui 当前页签只有 Overview、Proxies、Subscriptions、Connections、Logs。
- Ratatui 当前已覆盖：状态摘要、代理组展示/节点选择基础、模式切换、订阅 use/update、连接关闭、日志暂停/过滤/滚动、结构化 runtime/profiles 读取。
- Ratatui 当前未完整覆盖：start/stop/restart、doctor/config doctor、config view/raw/merge/set-*、sub add/import/remove/log、geodata update、upgrade/upgrade-kernel、secret show/set、proxy shell env、desktop proxy、TUN 真实诊断、env/test/version 的完整页面化体验。
- 鼠标当前仅启用 capture，并只对 Logs 页滚轮做了实际处理；页签、按钮、表格行、弹窗、输入框、列表和其他滚动区域尚未具备统一点击/滚轮语义。

结论：

- 当前 Ratatui 可以作为基础监控和少量操作面板，但还不是完整终端 GUI。
- `PROJECT_HARDENING_SPEC.md` 的 P2-1 已将目标提升为 `clashctl` 全功能覆盖、清晰分页、action registry、统一 mouse hitbox 和所有元素点击/滚轮支持。
- `docs/RATATUI_REDESIGN_PLAN.md` 已给出基于当前 Go CLI 命令面的详细整改设计，包括左侧导航、紧凑导航降级、各页面 ASCII 布局、命令映射、鼠标/键盘交互、异步任务、确认弹窗和分阶段实施计划。

## 退出码审计结论

已修正为非 0 的确定失败路径：

- 缺少必要子命令、只打印帮助：`clashctl`、`clashctl config`、`clashctl sub`、`clashctl node`、`clashctl geodata`。
- `sub update` 普通交互模式失败和 `--cron` 失败。
- `sub use` 在旧服务未运行且新配置启动失败时的配置回滚和非 0 退出。
- `sub list` 读取 profiles 元数据失败。
- `sub log` 日志文件存在但读取失败。
- `clashctl log` 找到日志文件但 `tail` 执行失败。
- `clashctl tui` 子进程异常退出。

新增回归入口：

- `cmd/clashctl/cli_exit_test.go` 构建真实 `clashctl` 进程并验证代表性成功/失败退出码，包括损坏或不可读 `profiles.yaml`。
- `scripts/smoke/cli_exit_codes.sh` 在 CI Ubuntu Go job 中执行同类进程级 smoke。

保留为 0 或中间告警的路径：

- `status`、`proxy`、`tun` 无参数等状态展示命令。
- `secret` 无参数时只展示是否设置；`secret show` 无 secret 仍为 fatal。
- `sub import` 单文件失败会先告警，最终汇总失败时 fatal。
- `upgrade-kernel` 回滚前的 `Warn` 是上下文提示，最终失败路径仍 fatal。
- `upgrade-kernel --allow-unsigned` 的 checksum 警告是显式风险接受后的成功路径。

## 后续交付清单

详细命令见 `docs/EXTERNAL_VALIDATION_CHECKLIST.md`。

1. 在真实 Linux/systemd 环境执行安装、启动、状态、日志、停止、卸载验证。
2. 在 GitHub Actions 跑通 CI、install smoke、release workflow。
3. 用真实 release artifact 和 SHA256SUMS 验证 tarball 安装。
4. 用真实 Mihomo/yq/geodata 下载验证安装和升级链路。
5. 在 GNOME/KDE/XFCE 或目标桌面环境验证 `proxy desktop on/off/status`。
6. 根据本状态矩阵拆分提交或 PR：P0 core、installer/smoke、TUI、docs/status。
7. 外部验证通过后，再定稿 README/install/release 文档，避免把未验证承诺写进用户入口文档。
