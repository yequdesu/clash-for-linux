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
- `internal/kernel/service.go`：`systemctl` 调用失败时会带出 stdout/stderr，不再只显示 `exit status 1`。
- `cmd/clashctl/ssh_tun_guard.go`：`start`、`restart`、`tun on`、`sub use`、`upgrade-kernel` 在 route-capturing TUN 启动前建立通用 route guard 计划；保护来源包括当前 SSH 客户端、`CLASH_TUN_PROTECT_ROUTES`、已探测到的隧道连接网段和隧道底层端点。root/sudo 场景下自动写入 priority 8000、table 2021 的保护规则，非 root 且需要保护时失败并提示 `sudo`、`--allow-route-risk` 和手工保护路由；旧 `--allow-ssh-tun-risk`/`CLASH_ALLOW_SSH_TUN_RISK=1` 保留为兼容入口。
- `cmd/clashctl/ssh_tun_guard.go`：sudo 清理 `SSH_CONNECTION` 时，会沿 `/proc/$PPID` 父进程链读取祖先进程环境，识别 SSH 客户端。
- `cmd/clashctl/ssh_tun_guard.go`：route guard 安装多条规则时事务化处理；状态写入或规则安装失败会回滚本轮已安装规则；默认路由不会被自动保护，避免绕开整个 TUN。
- `clashctl doctor` 新增 route guard 风险提示和 systemd inactive/raw running 状态提示。
- `install.sh`、`scripts/init/systemd.sh`：移除 `LimitNPROC=500` 和 `ExecStartPre=/usr/bin/sleep 1s`，避免桌面/SSH 用户进程较多时 systemd 无法 fork `sleep` 而失败；新增 `StartLimitIntervalSec=60`、`StartLimitBurst=3` 限制失败重启风暴。

已用临时新二进制验证：

- 将当前工作树交叉构建出的 Linux amd64 `clashctl` 临时上传到 `/tmp/clashctl-codex`，未替换正式 `/usr/local/bin/clashctl`。
- 普通用户 `/tmp/clashctl-codex status` 可识别当前 raw/nohup 内核 pid `2965170` 和 `Tun status: disabled`。
- `sudo /tmp/clashctl-codex status` 不再报 `clashctl not installed`，同样识别 `/home/yequdesu/.clashctl` 下的运行状态。
- `sudo /tmp/clashctl-codex doctor` 显示 base dir 为 `/home/yequdesu/.clashctl`，并提示 `service state: kernel pid 2965170 running outside active systemd unit`。
- 修正远端 `/etc/systemd/system/clashctl.service` 后，用临时新版执行 `sudo /tmp/clashctl-codex tun on` 成功；输出 `protected SSH client route before TUN: 192.168.1.20/32 via table 2021 priority 8000`。
- 验证后 `clashctl status` 显示 `Tun status: enabled`，`systemctl is-active clashctl` 为 active，`ip rule` 含 `8000: from all to 192.168.1.20 lookup 2021`，`ip route get 192.168.1.20` 使用 `table 2021 dev wlp5s0`，公网地址使用 `SakuraiTunnel table 2022`。

仍需真机复测：

- 安装新构建后的 `sudo clashctl tun off` 是否能直接命中 `/home/yequdesu/.clashctl`。
- SSH 环境下正式安装后的 `sudo clashctl tun on/start` 遇到 `tun.auto-route=true` 是否在 stop/start 前保护当前 SSH 客户端路由，并保留当前连接。
- 存在 WireGuard/Tailscale/OpenVPN/ZeroTier 等隧道路由或 `CLASH_TUN_PROTECT_ROUTES` 时，`sudo clashctl tun on/start` 是否安装全部 route guard，并且 `tun off/stop` 是否清理。
- 非 root 遇到需要保护的隧道路由时，是否拒绝继续并显示 `sudo`、`--allow-route-risk`、`CLASH_TUN_PROTECT_ROUTES` 指导。
- `--allow-route-risk` 和兼容别名 `--allow-ssh-tun-risk` 显式覆盖路径是否可用。
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
| P1-3 TUN 模式可靠化 | PARTIAL_LOCAL | YAML 写入、快照回滚、启动失败回滚测试通过；sudo base-dir 解析、route guard、手工保护路由、隧道路由探测、doctor 风险提示已有本地测试 | root/capability、`/dev/net/tun`、`ip link`、真实路由行为、SSH 和隧道场景下 route guard |
| P1-4 订阅转换器管理 | PARTIAL_LOCAL | subconverter 端口选择、pref 写入/恢复有测试 | 真实 subconverter 二进制和订阅转换结果 |
| P1-5 日志统一 | PARTIAL_LOCAL | CLI 统一读取 `cfg.LogFile()`、legacy log 和 `journalctl -u cfg.ServiceName`；tail 失败 fatal | 真实 systemd journal 和 nohup 日志路径 |
| P1-6 安装脚本幂等和可审计 | PARTIAL_LOCAL | shell 语法、static smoke、release smoke 脚本存在 | root 权限真实安装、重复安装、卸载后残留检查 |
| P1-7 本地残留收尾 | PARTIAL_LOCAL | `status` profiles 错误会 warning；`sub` 审计日志写入失败可见且 cron 成功路径严格；`sub list` 使用 YAML 结构化统计代理数；TUN 检测优先 `ip tuntap`/`ip -d -j link`；`doctor` 退出路径统一到 `exitProcess`；新增对应 Go 测试，`go test ./...`、`go vet ./...` 通过 | TUN 检测仍需真实 Linux 设备和 Mihomo 启动行为验证 |

## P2 状态矩阵

| 项目 | 状态 | 本地证据 | 仍需验证 |
| --- | --- | --- | --- |
| P2-1 Ratatui 完整终端控制面 | PARTIAL_LOCAL | 一级页已收敛为 Subscriptions/Proxies/Connections/Traffic/Network/Logs/Settings/Help；TUI 结构化读取 runtime/profiles；调用 `clashctl` 子命令时传递安装环境；Network 页已接入 status/doctor/config doctor/start/stop/restart、TUN、shell env、shell proxy、desktop proxy 的 CLI 动作按钮和键盘入口；Settings/i18n 已新增 `settings.yaml` 持久化语言设置、`Ctrl-L` 切换和导航/标题/Settings/Help/Network/Connections/Proxies/Traffic/Settings/Logs 运行态标签基础翻译；Settings / General 已接入 Language、Theme、Default page、Refresh interval、Mouse、Confirm 的 registry 按钮、键盘入口和持久化，主题设置会立即切换终端配色，默认页设置会影响下次启动页，刷新间隔设置会影响自动数据刷新频率，鼠标捕获偏好会影响下次 TUI 启动，关闭危险确认需要二次确认；Settings / Traffic 已接入 `traffic prune --retention` 表单动作，支持单值和 raw/rollup retention 输入并走确认弹窗；Settings / Traffic 默认 Range、Chart、By 已写入 `settings.yaml` 并影响下次 TUI 启动；Settings / Updates 已接入 `geodata update --version` 表单动作，支持 latest 或单个 release tag 输入并走确认弹窗；Settings prompt 已升级为字段化结构表单，支持 Tab/上下键切换字段、字段级鼠标点击聚焦、字段级中英双语标签/提示，并在弹窗打开时阻断底层页面动作；Settings 已接入 doctor/config doctor/config view/config raw/test/version/config merge/config autofix/config set-port/config set-api/config set-dns-mode/config set-lan/secret status/secret reveal/secret set/geodata update/API upgrade/kernel upgrade 的 CLI-backed 动作按钮、键盘入口和结果面板；`config set-api` 表单已支持 controller、`--secret` 和 `--allow-unsafe` 两种输入风格并在确认弹窗遮蔽 secret；Settings 普通输出默认脱敏，只有显式确认后的 secret reveal 允许明文显示，secret set 输入和确认均掩码展示；config set-* 表单已具备输入校验、确认弹窗和鼠标入口；Subscriptions 已接入多字段 add/edit 表单、import/remove/log，remove 走确认弹窗，log 有输出面板，订阅表单/quick-edit 弹窗主要可见文案已接入 i18n，底部按钮和表单 Save/Cancel hitbox 会按翻译后的终端显示列宽注册，避免中文宽字符在 tmux 分屏/窄 pane 中导致点击错位；状态栏、确认弹窗、Command Palette、Help 和风险等级文案已接入 i18n；Traffic 已接入 sample/collector start/collector stop/collector restart/prune/reset/export，stop/restart/prune/reset 走确认弹窗并显示命令结果，Traffic 图表支持滚轮平移历史窗口、点击桶锁定详情和点击空白清除锁定；Proxies 已接入 mode/sort/open nodes/switch/test selected/test all 的 registry 动作条，节点弹窗支持鼠标行选择、滚轮选择、Switch/Test/Close registry 按钮，并在弹窗打开时阻断底层表格误点；Connections 已接入 close selected/close all 的 registry 动作条；Logs 已接入 pause/filter/clear 的 registry 动作条；Network/Settings/Subscriptions/Traffic 危险动作已有基础确认弹窗，支持 Enter/y 确认、Esc/n 取消和鼠标 Confirm/Cancel；已新增通用 mouse hitbox registry 并覆盖页签、主要表格行、订阅操作与编辑弹窗、Settings 动作、Traffic 动作和 Logs/Traffic 滚轮；已新增 action registry，登记 action id、page、label、shortcut、mouse、danger、executor，Help 页面和 Proxies/Connections/Network/Traffic/Logs/Settings 可见动作按钮均由 registry 生成；Command Palette 已支持 `Ctrl-P` 打开、输入过滤、Up/Down 选择、Enter 执行、Esc 关闭，并复用 registry/HitboxAction 分发；action registry 的 label/button/description 已接入中英双语翻译，Help、Command Palette 和 registry 驱动按钮会按语言设置展示；Rust 测试校验 action id 唯一、Network/Settings/Traffic CLI-backed action enum 全部登记、registry 驱动的 Proxies/Connections/Traffic/Logs/Settings 可见按钮都有 hitbox 分发、command palette 可过滤/切页/打开 Settings 表单、Settings prompt 字段化输入、字段级鼠标聚焦/底层阻断、Settings / Traffic 默认视图 key 映射、Traffic 图表滚轮平移/点击桶锁定、订阅字段翻译、CJK 宽字符按钮/页签 hitbox 显示列宽计算、所有 registry action 都有中文 label/button/description 且 command palette 能按中文搜索，并从 Go Cobra 源码反推命令树校验 `cli_coverage` 矩阵无遗漏/无陈旧项，同时用 `cli_variant_coverage` 校验 proxy/tun/env/secret/config/sub/node/traffic 等参数变体要么映射到现有 action，要么显式标记 CLI-only；113 个 Rust 测试通过 | 仍需低频状态/错误消息继续迁移到 i18n key、Network 诊断输出的真实 Linux 行为验证、真实 Mihomo API 和终端交互 |
| P2-2 Release 工程 | PARTIAL_LOCAL | release workflow 和 release install smoke 存在 | GitHub release artifact、SHA256SUMS、tarball 安装 |
| P2-3 配置模板与规则管理 | PARTIAL_LOCAL | geodata staging 替换、install-state 记录和回滚测试通过；TUI Settings / Updates 已接入 `geodata update --version` 参数化表单 | 真实 geodata 下载源和网络失败场景 |
| P2-4 流量统计、持久化与可视化 | PARTIAL_LOCAL | Go `internal/traffic` 已实现 connection delta sampler、JSONL raw sample store、10s/1m rollup 文件、state cursor、history/top/prune 查询；`clashctl traffic status --json` 和 sample/collect/history/top/export/prune/reset 已接入，`traffic export --by` 已按维度过滤；新增 `traffic collector start/stop/restart/status`，后台采集器写入 `collector.json` 和 `collector.log`，Linux 下以 detached session 启动并继承 install env；`traffic status --json` 暴露 collector running/stale/pid/interval/log；TUI Traffic tab 读取 `clashctl traffic history/top/status --json` 并显示大图表、Line 多行主图、Bar 桶状图、route ranking、range/chart/dimension 键盘与鼠标控件、选中 breakdown 过滤、图表滚轮平移历史窗口、点击时间桶锁定 Bucket detail、点击图表空白解除锁定、CSV 导出入口、sample/collector start/stop/restart/prune/reset 动作和 store/collector 状态；Settings / Traffic 已接入 `traffic prune --retention` 参数化表单，支持 day 简写转 Go duration；Settings / Traffic 默认 Range、Chart、By 已接入 `settings.yaml`；Traffic ranking 支持鼠标行选择和滚轮，sample/collector/prune/reset/export 支持鼠标点击，stop/restart/prune/reset 走确认弹窗；fake Mihomo API 采样测试、collector stale/status 测试、rollup/store/query/export 测试、Traffic Line 图表纵向视窗测试、Traffic 图表滚轮平移/点击桶锁定测试、Settings / Traffic 默认视图 key 映射测试和 113 个 Rust 测试通过 | 真实 Mihomo 流量采样、真实长期 collector 运行、systemd/timer 化策略、真实性能验证 |
| P2-5 订阅参数化管理 | PARTIAL_LOCAL | Go profile metadata 已扩展；`sub add` 已支持 `--name/--interval/--update-proxy/--user-agent/--convert/--tag` 并在首次下载使用指定 UA/update proxy；`sub rename/set-url/set-interval/set-update-proxy/set-user-agent/set-convert/tag` 已实现；`sub update --scheduled --cron` 按 profile policy 更新到期项并记录 `last_error`；兼容旧 `interval`；`update_proxy` 已接入 direct/system/core/auto 下载路径；TUI subscription list 已显示 interval/next/status；`sub add` 和 `sub edit` 已升级为多字段表单，支持 source/name/interval/update proxy/user-agent/convert/tags，支持键盘字段切换、鼠标字段聚焦、输入校验和后台 CLI 执行；编辑保存会顺序调用 `sub rename/set-url/set-interval/set-update-proxy/set-user-agent/set-convert/tag add|remove`，不直接写 `profiles.yaml`；`sub import/remove/log` 已接入 TUI，空订阅列表也可 add/import/log，remove 走确认弹窗，log 输出在订阅页结果区；订阅行、底部操作和编辑弹窗支持鼠标选择/触发/保存/取消，订阅字段/占位符/表单按钮已接入中英双语，底部操作和表单按钮 hitbox 按终端显示列宽注册；`go test ./...` 和 113 个 Rust 测试通过 | 真实订阅 URL 调度、端到端 cron 验证 |

## Ratatui 覆盖审计

当前代码事实：

- Go CLI 命令面包含生命周期、状态、日志、proxy、desktop proxy、TUN、secret、upgrade、upgrade-kernel、config、sub、node、env、test、doctor、geodata、version。
- Ratatui 当前一级页为 Subscriptions、Proxies、Connections、Traffic、Network、Logs、Settings、Help；Network 已覆盖主要代理开关，Settings 已覆盖一批诊断/维护动作，但仍未覆盖全部命令。
- Ratatui 当前已覆盖：状态摘要、Network CLI 动作按钮和键盘入口、Network shell env 独立动作、Network/Settings/Subscriptions/Traffic 危险动作基础确认弹窗、Settings 基础页面、Settings General 语言/主题/默认页/刷新间隔/鼠标/危险确认可点击与可持久化、Settings / Traffic retention prune 参数化表单、Settings / Updates geodata version 参数化表单、Settings doctor/config doctor/config view/config raw/test/version/config merge/config autofix/config set-port/config set-api/config set-dns-mode/config set-lan/secret status/secret reveal/secret set/geodata update/API upgrade/kernel upgrade 动作与脱敏/显式 reveal 结果面板、Settings config set-* 字段化结构表单，其中 `config set-api` 已支持 controller、`--secret` 和 `--allow-unsafe` 参数化输入并遮蔽 secret、`settings.yaml` 语言持久化、导航/标题/Settings/Help/Network/Connections/Proxies/Traffic/Settings/Logs 运行态标签基础 i18n、状态栏/确认弹窗/Command Palette/Help/Subscriptions 主视图和订阅表单主要文案 i18n、代理组展示、节点弹窗选择、模式切换、订阅多字段 add/edit、订阅 import/use/update/remove/log、订阅 interval/next/status 展示、订阅快速单字段编辑、订阅底部操作鼠标入口、连接关闭、Traffic 持久化历史/route ranking 展示、Traffic range/chart/dimension 键盘与鼠标控件、Settings / Traffic 默认视图偏好、Traffic 图表滚轮平移/点击时间桶锁定、breakdown 过滤、Traffic sample/collector start/collector stop/collector restart/prune/reset/CSV 导出、Traffic store/collector 状态展示、日志暂停/过滤/滚动、结构化 runtime/profiles 读取。
- Ratatui 当前未完整覆盖：Traffic collector 真实长期运行和 systemd/timer 化策略、所有 TUI 文案统一 i18n key、Settings 剩余低频结构化表单、Network 诊断输出的真实 Linux 行为验证。
- 鼠标已新增通用 hitbox registry，覆盖页签、Proxies/Subscriptions/Connections/Traffic 表格行、Proxies 节点弹窗行选择和滚轮、Subscriptions 底部操作、Proxies/Connections/Network/Traffic/Logs/Settings 动作按钮、Settings General 偏好按钮、Settings prompt 输入区 focus、确认弹窗 Confirm/Cancel、Traffic range/chart/dimension 控件、Traffic 图表滚轮平移/点击时间桶锁定、订阅编辑弹窗保存/取消、Traffic CSV 导出、Logs 滚轮和 Traffic ranking 滚轮；Subscriptions 底部按钮、订阅表单 Save/Cancel、Settings prompt Continue/Cancel、确认弹窗 Confirm/Cancel 和页签会按终端显示列宽注册 hitbox；action registry 已驱动 Help、Command Palette、Proxies 动作区、Proxies 节点弹窗动作区、Connections 动作区、Network 动作区、Traffic 动作区、Logs 动作区和 Settings 动作区，registry action 的 label/button/description 已有中英双语，并用测试覆盖 CLI-backed action enum 登记完整性、Proxies/Connections/Traffic/Logs/Settings 可见按钮 hitbox 分发、Settings prompt 字段化输入、字段级鼠标聚焦/底层阻断、CJK 宽字符按钮/页签 hitbox、Traffic 图表滚轮平移/点击桶锁定、节点弹窗鼠标选择/滚轮/底层阻断、command palette 基础执行、中文搜索、Go Cobra 源码反推命令树校验 `cli_coverage` 无遗漏/无陈旧项，以及参数变体级 `cli_variant_coverage` 映射/CLI-only 审计；仍缺少真实终端鼠标验证。

结论：

- 当前 Ratatui 可以作为基础监控和少量操作面板，但还不是完整终端 GUI。
- `PROJECT_HARDENING_SPEC.md` 的 P2-1 已将目标提升为 `clashctl` 全功能覆盖、清晰分页、action registry、统一 mouse hitbox 和所有元素点击/滚轮支持。
- `docs/RATATUI_REDESIGN_PLAN.md` 已给出基于当前 Go CLI 命令面的详细整改设计，包括左侧导航、紧凑导航降级、Traffic 大视窗、订阅参数化编辑、各页面 ASCII 布局、命令映射、鼠标/键盘交互、异步任务、确认弹窗和分阶段实施计划。

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
