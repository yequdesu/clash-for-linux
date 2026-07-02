# Clash for Linux Project Audit Report

本文档是对当前工作树的工程审计结论，覆盖 Go 控制面、终端 TUI、功能完整度、稳定性、可靠性、可审计性、易用性和常见 Linux Clash/Mihomo 使用痛点。

审计日期：2026-07-02

## 总结结论

当前项目已经从“脚本 + CLI 原型”推进到“具备成熟项目骨架和大量本地回归测试的 Linux-first Mihomo 管理器”。但在真实 Linux/systemd、真实 release artifact、真实 Mihomo/yq/geodata 下载、真实桌面代理和 GitHub Actions 全绿之前，不能把它定义为稳定成熟版本。

当前可给出的确定性结论：

- Go 控制面质量：本地代码质量和失败路径已经显著增强，核心状态写入、订阅、升级、API client、服务管理都有测试覆盖；真实 Linux/systemd 仍是主要未验证面。
- TUI 质量：当前是 Rust 终端 TUI，不是 Tauri 应用。它已经具备结构化 runtime 读取、安装目录解析、API 错误体显示和 CLI 子命令环境传递；真实终端交互和真实 Mihomo API 仍需验证。
- 功能完整度：CLI/TUI/installer/update/uninstall/doctor/geodata/upgrade/subscription/proxy/TUN 的骨架完整，但 release 和真实桌面/TUN 行为还不能只靠本地测试证明。
- 稳定性与可靠性：原子写、锁、快照、回滚、非 0 退出码、nohup pid 归属校验已补齐主要缺口；稳定性等级取决于后续真实 Linux 验证。
- 可审计性：`install-state.json`、checksum、来源 URL、geodata 状态和 CI/smoke 入口已经形成审计基础；还缺真实 release 产物证据。
- 易用性：常见痛点已有命令入口和诊断入口；当前 README 已改为硬化期入口文档，最终用户级安装文档应等真实验证完成后再定稿。

## Go 控制面审计

### 已改善的质量点

- 服务管理明确区分 `ServiceName` 和 `KernelName`，避免 systemd unit 和内核进程名混用。
- nohup fallback 不再用 `pgrep/pkill` 按名字杀进程，而是通过 pid file、`/proc`、可执行文件归属和 syscall 控制受管进程。
- 配置、订阅 profile、runtime、install-state 等关键文件使用原子写、文件锁和快照回滚。
- 订阅 add/import/update/use/remove 由可返回错误的核心函数实现，不再通过 Cobra 嵌套调用伪装成功。
- `sub update --cron` 静默且保留失败 exit code；普通 `sub update` 失败也返回非 0。
- `TestCLIProcessExitCodes` 和 `scripts/smoke/cli_exit_codes.sh` 覆盖真实进程级退出码，避免 `ilog.Fatal/os.Exit` 路径只停留在代码审计；profiles 元数据缺失和真实读取错误已区分处理。
- `config set-*`、`secret`、`tun` 复用结构化 YAML 写入和回滚路径。
- 内核 API client 增加 timeout、path escaping、JSON body 和错误响应体。
- `upgrade-kernel` 支持 release asset 选择、gzip、SHA256 checksum、`--allow-unsigned`、旧内核备份和失败回滚。
- `geodata update` 使用 staging 目录，全部下载成功后再替换，install-state 写入失败会回滚。
- `doctor` 形成安装、配置、安全、API、TUN、geodata 的集中诊断入口。

### 仍需关注的风险

- systemd 生命周期、权限和日志路径必须在真实 Linux 中验证。
- `tun` 设备检测仍需要真实路由、capability、`ip link` 行为验证。
- `upgrade-kernel` 的 release asset 选择需要对真实 MetaCubeX release 资产做一次端到端验证。
- install/update/uninstall 脚本虽然有静态和 smoke 入口，但 root 权限真实运行仍是必须门槛。
- 当前 dirty worktree 很大，后续应按 P0 core、installer/smoke、TUI、docs 拆分，降低审查难度。

### Go 控制面评级

本地工程质量：`B+`

达到稳定发布前提后的预期：`A-`

限制条件：评级不包含真实 Linux/systemd/release 结果；这些结果可能暴露权限、路径、发行版差异或 Mihomo 兼容性问题。

## TUI 审计

### 范围澄清

当前项目没有 Tauri 应用。`clash-tui` 是 Rust 终端 TUI。若未来目标是 Tauri 桌面 GUI，需要单独定义产品边界、权限模型、IPC、打包和自动更新策略，不应把现有 TUI 误称为 Tauri。

### 已改善的质量点

- TUI 使用结构化 YAML 读取 runtime，而不是脆弱字符串扫描。
- API 地址规范化覆盖 IPv4、IPv6、`0.0.0.0`、`::` 等监听场景。
- 安装目录解析顺序与 CLI 对齐：`CLASH_BASE_DIR`、用户 install marker、系统 install marker、默认目录。
- TUI 调用 `clashctl sub use/update` 时会传递 `CLASH_BASE_DIR`、`SERVICE_NAME`、`KERNEL_NAME`，避免显示和操作落到不同实例。
- API 错误包含 response body，方便诊断 Mihomo 返回的真实原因。
- TUN 状态读取结构化 runtime。
- Rust 测试覆盖安装路径解析、API URL 推导、错误体、节点选择器、日志过滤和模式显示。

### 仍需关注的风险

- 真实终端尺寸、颜色、键盘交互、异常网络和慢 API 响应还未完成端到端验证。
- 节点切换、连接关闭、订阅更新等真实 Mihomo 行为需要在运行内核上验证。
- TUI 目前仍依赖外部 `clashctl` 子进程完成部分 mutating workflow，必须保证 release artifact 同时安装 CLI 和 TUI。
- 终端 UI 的帮助提示必须持续和真实快捷键保持一致。
- 按新的产品目标，Ratatui 必须覆盖 `clashctl` 的全部日常管理命令；当前 5 个页签尚未覆盖 start/stop/restart、doctor/config doctor、config set-*、sub add/import/remove/log、geodata、upgrade/upgrade-kernel、secret、desktop proxy、TUN 诊断等完整工作流。
- 当前鼠标只对 Logs 页滚轮有实际行为；页签、按钮、表格行、弹窗、输入框、列表和其他滚动区域还没有统一 hitbox 和事件分发模型，不能满足“所有元素可点击/可滚轮”的目标。

### TUI 评级

本地工程质量：`B-`

达到完整 Ratatui 控制面和真实交互验证后的预期：`A-`

限制条件：没有真实 Mihomo API 和终端交互验证前，不能评为成熟控制台。

## 功能完整度审计

### 已覆盖功能

- 安装、更新、卸载脚本。
- systemd 与 nohup fallback。
- `clashctl start/stop/restart/status/log/doctor/test/env/version`。
- 订阅 add/import/list/use/update/remove/log。
- 配置 view/raw/merge/set-port/set-api/set-dns-mode/set-lan/doctor。
- 节点 list/switch/delay。
- proxy shell env 和 desktop proxy on/off/status。
- secret show/set。
- TUN on/off/status。
- kernel API upgrade 和 release binary upgrade。
- geodata update。
- TUI 状态、节点、连接、日志、订阅相关控制面基础。

### 仍不应承诺成熟的功能

- 跨发行版安装完全可靠。
- 真实桌面代理覆盖所有主流 DE。
- TUN 在所有发行版、内核和路由策略下稳定。
- release artifact 可直接作为稳定版交付。
- 订阅转换器对所有输入格式可靠。
- TUI 在所有终端和真实 API 负载下交互完整。

## 稳定性审计

当前稳定性改进集中在四类：

- 状态一致性：profile 文件、profiles metadata、config/runtime 同步更新和回滚。
- 进程安全：受管 pid 归属校验，停止前确认目标进程属于当前安装目录。
- 失败可见：关键 mutating commands 失败返回非 0，避免 cron、脚本、TUI 误判成功。
- 配置可恢复：merge、TUN、secret、sub use/update、kernel/geodata upgrade 失败时恢复旧状态。

仍需真实验证的稳定性风险：

- systemd unit 超时、权限、重启策略和 journal 行为。
- nohup fallback 在不同 `/proc` 行为下的兼容性。
- 网络失败、GitHub 速率限制、代理下载失败。
- 文件系统权限、跨设备 rename、只读目录、磁盘满。

## 可靠性审计

可靠性已经具备本地工程基础：

- Go 单元测试覆盖核心配置、订阅、升级、服务、API 和日志行为。
- Go 集成测试会构建真实 `clashctl` 进程并验证代表性成功/失败退出码。
- Rust 单元测试覆盖 TUI 核心解析和 API 行为。
- shell static smoke、nohup pid safety smoke 和 CLI exit-code smoke 入口已存在；其中 static/nohup 已本地执行通过，CLI exit-code smoke 已通过 `bash -n` 和 Go 集成测试覆盖其核心语义。
- release/systemd/distro smoke 脚本已存在。

可靠性尚未被证明的部分：

- GitHub Actions 实际运行。
- 真实 Linux `GOOS=linux GOARCH=amd64 go test ./...`。
- 真实安装和卸载。
- 真实 TUN、桌面代理和 release artifact。

## 可审计性审计

已具备的审计能力：

- `install-state.json` 记录 base dir、service name、kernel name 和组件信息。
- kernel upgrade 记录版本、source URL、checksum URL、checksum 是否验证、是否允许 unsigned。
- geodata update 记录版本、source URL、资源路径和更新时间。
- `doctor` 输出安装状态、安全风险和常见环境缺陷。
- shell 脚本语法、static safety、nohup pid safety 可作为审计入口。

还需补强：

- release artifact 的 provenance 和 SHA256SUMS 真实验证结果。
- CI run URL 和真实系统验证日志。
- 每个稳定版本的硬化状态快照。

## 易用性审计

已改善的易用性：

- `proxy on/off` 输出可 `eval` 的 shell 脚本，不再误导用户认为子进程能修改父 shell 环境。
- `secret` 默认隐藏，`secret show` 明确警告。
- `doctor` 提供集中诊断入口。
- `config set-*` 覆盖常见端口、API、DNS、LAN 设置。
- `geodata update`、`upgrade-kernel`、`sub update --auto/--cron` 覆盖常见维护操作。
- 错误路径给出更具体原因，脚本可通过 exit code 判断失败。

仍需改善的易用性：

- README 已作为硬化期入口文档更新；最终用户级 install/release 文档需要在真实验证完成后定稿。
- 需要增加“首次安装后推荐命令”文档。
- 需要增加“订阅导入失败如何排查”、“TUN 失败如何排查”、“桌面代理失败如何排查”。
- TUI 帮助和快捷键需要真实交互验证后再最终定稿。

## 常见痛点覆盖情况

| 痛点 | 当前覆盖 | 状态 |
| --- | --- | --- |
| 服务名混乱 | `ServiceName` 与 `KernelName` 分离 | PARTIAL_LOCAL |
| API 暴露到公网 | 默认 loopback，doctor 检查 unsafe controller | DONE_LOCAL |
| secret 明文泄露 | 默认隐藏，显式 show 才打印 | DONE_LOCAL |
| 订阅更新破坏当前配置 | temp、校验、快照、回滚 | DONE_LOCAL |
| cron 静默失败 | `--cron` 静默且非 0 | DONE_LOCAL |
| 命令失败返回 0 | 关键 mutating paths 已审计修正，并有进程级退出码测试 | DONE_LOCAL |
| 错误杀进程 | nohup pid 归属校验 | PARTIAL_LOCAL |
| TUN 半更新 | YAML 写入和回滚 | PARTIAL_LOCAL |
| geodata 混合版本 | staging 全量替换 | DONE_LOCAL |
| 内核升级不可回滚 | backup、checksum、rollback | PARTIAL_LOCAL |
| 桌面代理设置无效 | GNOME/KDE 命令构造和失败 fatal | PARTIAL_LOCAL |
| TUI 操作错实例 | 安装环境传递给子进程 | DONE_LOCAL |

## 发布建议

下一步不建议直接发布稳定版。建议流程：

1. 先按 `HARDENING_STATUS.md` 和 `EXTERNAL_VALIDATION_CHECKLIST.md` 完成真实 Linux、CI、release、TUN、桌面代理验证。
2. 修复验证中发现的问题。
3. 将当前大工作树拆分为可审查提交。
4. 用真实验证结果更新 `HARDENING_STATUS.md`。
5. 再重写 README、安装文档和 release notes。

只有当 P0 全部从 `PARTIAL_LOCAL` 降为真实验证通过，才应标记 v0.3.0 稳定线。
