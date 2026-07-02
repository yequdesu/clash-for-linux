# Clash for Linux Hardening Specification

本文档定义本项目从“可用原型”提升为“稳定、可靠、可审计的 Linux Clash/Mihomo 管理项目”的确定性改造要求。

本文档只以当前源码、脚本和构建行为为依据，不依赖 README 或既有说明文本。后续所有改造都应优先满足本文档中的验收标准。

## 1. 产品边界

项目的真实定位必须明确为：

- `clashctl`：Linux 上的 Mihomo/Clash Meta 控制面 CLI。
- `clash-tui`：基于 Ratatui 的 Linux 终端完整控制面，目标是在命令行环境中覆盖 `clashctl` 提供的全部日常管理能力，并以清晰分区、分页、鼠标和键盘双入口提供接近桌面 GUI 的易用性。
- `install.sh/update.sh/uninstall.sh`：安装、升级、卸载工具链。
- 本项目不实现代理内核，不 fork Mihomo，只负责下载、配置、启动、监控和管理 Mihomo。

非目标：

- 不实现自研代理协议栈。
- 不把订阅转换器、内核、地理数据库作为不可审计的黑盒静默更新。
- 不承诺跨平台桌面 GUI；当前 TUI 是 Linux-first 终端 UI，不是 Tauri 应用。TUI 可以追求桌面端图形界面的功能覆盖和易用性，但必须保持终端运行、可脚本化诊断和可靠失败反馈。

## 2. 质量原则

所有后续改造必须遵守以下原则：

1. 单一事实源：服务名、安装路径、日志路径、运行配置、API 地址只能有一个权威来源。
2. 安全默认值：默认只监听本机，默认生成强 secret，默认不明文展示 secret。
3. 原子写入：配置、订阅、profile 元数据、runtime 文件必须用临时文件加 rename 写入。
4. 结构化解析：YAML、JSON、URL、systemd 状态不得靠脆弱字符串扫描完成核心判断。
5. 可回滚：内核升级、订阅更新、配置合并、TUN 开关必须失败可恢复。
6. 可审计：所有下载必须记录来源、版本、校验方式、安装结果。
7. 可测试：核心逻辑必须能在没有真实 Mihomo 进程的情况下用单元测试覆盖。
8. 可诊断：失败时必须给出具体原因、相关路径、下一步检查命令。

## 3. 发布门槛

任何被标记为稳定版本的提交必须满足：

```bash
gofmt -w ./cmd ./internal
go test ./...
GOOS=linux GOARCH=amd64 go test ./...
go vet ./...

cd tui
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

Shell 脚本必须满足：

```bash
bash -n install.sh update.sh uninstall.sh install_tui.sh scripts/preflight.sh scripts/init/nohup.sh scripts/init/systemd.sh scripts/smoke/*.sh
bash scripts/smoke/static_safety.sh
bash scripts/smoke/nohup_pid_safety.sh
```

安装链路必须满足：

- `.github/workflows/install-smoke.yml` 的 release artifact job 必须验证 installer 可从 tarball + SHA256SUMS 安装 `clashctl` 和 `clash-tui`，并写入正确 `install-state.json` 来源。
- `.github/workflows/install-smoke.yml` 的 systemd service job 必须验证 `install.sh` 写入 unit、`clashctl start/status/doctor/stop` 与 `systemctl status clashctl` 的一致性。
- `.github/workflows/install-smoke.yml` 在 Ubuntu、Debian、Fedora、Arch 容器中通过。
- `scripts/smoke/distro_install.sh` 不依赖真实 Mihomo/yq/geodata 下载，必须能验证安装、配置命令、doctor、卸载的确定性行为。
- `scripts/smoke/nohup_pid_safety.sh` 必须验证 nohup fallback 只停止当前安装目录的受管内核 pid，不会杀死 pid file 指向的无关进程。
- 稳定发布前还必须额外完成至少一次真实 VM/systemd 安装验证。

关键可变更命令必须满足：

- `clashctl geodata update` 任一下载、替换、状态写入或回滚失败时必须以非 0 退出码结束。
- `clashctl upgrade-kernel` 任一下载、校验、停止、备份、替换、启动、状态写入或回滚失败时必须以非 0 退出码结束；“已经是最新版本”是成功路径。
- `clashctl config merge/edit/view/raw/set-*` 任一读取、校验、写入、合并或回滚失败时必须以非 0 退出码结束。
- `clashctl sub add/import/update/use/remove` 任一下载、转换、校验、写入、激活、元数据保存或回滚失败时必须以非 0 退出码结束。
- `clashctl start/stop/restart/tun on/tun off` 任一停止、启动、配置写入、合并、验证或回滚失败时必须以非 0 退出码结束。
- `clashctl node list/switch/delay` 任一 API 连接、认证、代理列表读取或节点切换失败时必须以非 0 退出码结束；单个节点测速失败可以作为结果展示。
- `clashctl proxy desktop on/off` 任一桌面代理工具缺失或设置失败时必须以非 0 退出码结束；`proxy on/off` 只负责输出可 `eval` 的 shell 脚本。
- `clashctl secret <new-secret>` 写入、合并或回滚失败时必须以非 0 退出码结束；`clashctl secret show` 在 secret 为空时必须以非 0 退出码结束。
- `clashctl upgrade` 通过内核 API 升级失败时必须以非 0 退出码结束。
- `clashctl`、`clashctl config`、`clashctl sub`、`clashctl node`、`clashctl geodata` 这类缺少必要子命令、仅打印帮助的调用必须以非 0 退出码结束。

仓库必须加入 `.gitattributes`，固定 shell 脚本为 LF：

```gitattributes
*.sh text eol=lf
scripts/** text eol=lf
```

## 4. P0 必修项

P0 是阻止项目成为可靠 Linux 工具的直接缺陷，必须优先完成。

### P0-1 统一 systemd 服务名和服务管理

现状问题：

- 安装脚本创建 `/etc/systemd/system/clashctl.service`。
- Go 代码控制的是 `${KernelName}.service`，默认 `mihomo.service`。
- 日志命令查找 `${resources}/mihomo.log`、`/var/log/mihomo.log`、`journalctl -u mihomo`，与安装脚本的 `logs/mihomo.log` 和 `clashctl.service` 不一致。

目标：

- 项目服务名统一为 `clashctl.service`。
- Mihomo 内核进程名仍可为 `mihomo`，但 systemd unit 名不得与内核名混用。
- `ServiceManager` 必须持有 `ServiceName` 和 `KernelName` 两个字段。

必须改动：

- 在 `internal/config.EnvConfig` 增加 `ServiceName`，默认 `clashctl`。
- `internal/kernel/service.go` 所有 systemd 操作使用 `cfg.ServiceName`。
- `cmd/clashctl/log.go` 使用 `journalctl -u cfg.ServiceName`。
- 安装脚本、卸载脚本、Go 逻辑全部使用同一个服务名。
- 如果 systemd unit 存在但 inactive，而同一安装目录下 raw/nohup 内核进程仍在运行，`ServiceManager.IsRunning/Stop` 必须识别并处理该 raw 进程，避免 root/systemd 视角和普通用户 raw 视角分裂。

验收：

- 安装后执行 `systemctl status clashctl` 可看到服务。
- `clashctl start` 启动同一个 unit。
- `clashctl stop` 停止同一个 unit。
- `clashctl status` 与 `systemctl is-active clashctl` 一致。
- `clashctl log` 能读取安装脚本写入的日志。
- `clashctl doctor` 能提示 systemd inactive 但 raw/nohup 内核仍在运行的异常状态。

### P0-2 安全默认 API 暴露

现状问题：

- `resources/mixin.yaml` 默认 `external-controller: 0.0.0.0:9090`。
- 默认 `secret: ""`。
- `clashctl status` 和 `clashctl secret` 会明文打印 secret。

目标：

- 默认 API 只监听 `127.0.0.1:9090`。
- 安装时生成至少 32 字节随机 secret，编码为 hex 或 base64url。
- 普通状态输出不得展示 secret 明文。

必须改动：

- `resources/mixin.yaml` 改为 `external-controller: 127.0.0.1:9090`。
- 安装脚本生成 32 字节以上 secret。
- `clashctl status` 显示 `api key: set` 或 `api key: empty`。
- `clashctl secret` 默认只显示是否已设置。
- 只有显式 `clashctl secret show` 才允许显示明文，并输出警告到 stderr。
- `clashctl secret <new-secret>` 必须复用结构化 YAML 写入流程，写入前快照 `mixin.yaml` 和 `runtime.yaml`，merge 失败时恢复旧文件。

验收：

- 新装后 API 不监听 `0.0.0.0`。
- `clashctl status` 不泄漏 secret。
- `clashctl secret <new-secret>` 在 merge 失败时不留下半更新 secret。
- 未设置 secret 时 `clashctl doctor` 必须报 P0 安全风险。

### P0-3 修复内核 API 客户端

现状问题：

- Go API client 无 timeout。
- group/proxy 名称没有 `url.PathEscape`。
- JSON body 手工拼接。
- 非 2xx 响应体没有纳入错误信息。
- TUI 也存在相同问题。

目标：

- 所有 API 请求都有 timeout。
- URL path 参数必须 escape。
- JSON body 必须由 JSON encoder 生成。
- 错误包含 HTTP status 和最多 4KB response body。

必须改动：

- `internal/kernel/api.go` 增加 `http.Client{Timeout: 8 * time.Second}`。
- `TestDelay`、`SwitchProxy` 使用 `url.PathEscape`。
- `SwitchProxy` 使用 `json.Marshal(struct{Name string})`。
- TUI `tui/src/api.rs` 使用 `urlencoding` 或等价方式 escape path segment，并用 `serde_json::json!` 生成 body。
- TUI 所有 API 方法，包括切换节点、切换模式、关闭连接和关闭全部连接，非成功响应都必须返回 HTTP status 和最多 4KB response body。
- TUI 和 Go 都应将 `/version` 作为版本接口，除非明确兼容 `/`。

验收：

- proxy group 名含空格、斜杠、中文、引号时，测速和切换不因 URL/JSON 拼接失败。
- TUI 在切换节点、切换模式、关闭连接失败时，错误信息包含 Mihomo response body 且最多截断到 4KB。
- 内核未启动时 CLI/TUI 在 8 秒内返回明确错误。

### P0-4 修复订阅更新与 profile 数据一致性

现状问题：

- `sub update --auto` 写入 cron：`clashctl sub update --cron`，但不存在 `--cron` flag。
- `hasProtocol` 对短字符串可能越界。
- 更新时转换后没有再次校验。
- profile `Name/Updated/Interval` 基本不维护。
- 写入 profile 和 `profiles.yaml` 非原子。

目标：

- 订阅 add/update/use/remove/import 都必须保持 profile 元数据和文件一致。
- 自动更新命令必须存在且无交互。
- 更新失败不得破坏当前可用订阅。

必须改动：

- 增加 `--cron` flag，语义为非交互、静默、保留 exit code。
- `hasProtocol` 改为 `strings.HasPrefix`。
- `sub update` 流程：下载到 temp -> 校验 -> 必要时转换 -> 再校验 -> 原子替换 profile。
- 成功更新时写入 `Updated`。
- 如果能从订阅头或配置中解析名称，写入 `Name`；不能解析则保留旧值。
- `sub add` 和 `sub import` 必须通过同一个可返回错误的核心添加流程写入 profile 文件和元数据，不得通过嵌套调用 Cobra command 伪装成功。
- `sub import` 必须逐文件报告成功/失败，失败项不得计入已导入数量。
- 本地文件订阅必须规范化为绝对 `file://` source 后再写入 profile 元数据和进行重复检测，避免相对路径、绝对路径或 `file://` 写法差异造成重复 profile。
- 新 profile 文件写入后如果 `profiles.yaml` 保存失败，必须删除新增 profile 文件，避免孤儿订阅文件。
- `sub update` 写入新 profile 内容前必须快照旧 profile 文件；如果 `profiles.yaml` 保存失败，必须恢复旧 profile 文件。
- active profile 更新后如果激活新 runtime 失败，必须恢复旧 profile 文件和旧 `profiles.yaml`，避免 profile 已更新但运行配置仍是旧版本。
- `sub remove` 删除 profile 文件前必须快照旧 profile 文件；如果 `profiles.yaml` 保存失败，必须恢复被删除的 profile 文件。
- `sub use` 写入新 `config.yaml` 前必须快照旧 `config.yaml` 和 `runtime.yaml`；合并或保存 active 元数据失败时必须恢复旧配置和旧 active profile。
- `sub use` 在旧服务正在运行时，如果新配置启动失败，必须恢复旧 `config.yaml`/`runtime.yaml` 并尝试重新启动旧服务。
- `config set-port`、`set-api`、`set-dns-mode`、`set-lan` 写入 `mixin.yaml` 前必须快照旧 `mixin.yaml` 和 `runtime.yaml`；merge 失败时必须恢复旧文件。
- `clashctl secret <new-secret>` 与 `config set-*` 使用同一套快照、结构化写入和 merge 回滚路径。
- `profiles.yaml` 写入使用文件锁和原子写。
- `sub update --auto` 合并 crontab 时必须忽略注释行，必须替换缺少 `--cron` 的旧活动行，不得因为旧坏行存在而误判已启用。
- `sub update` 和 `sub update --cron` 任一更新、保存、激活失败都必须以非 0 退出码结束；普通交互模式可以保留警告输出，但不得把失败伪装成成功。
- `sub update --cron` 不得向 stdout/stderr 输出普通进度、成功或失败提示；订阅操作审计仍写入 `profiles.log`。

验收：

- `clashctl sub update --cron` 是合法命令。
- `clashctl sub update` 和 `clashctl sub update --cron` 在无订阅、下载失败、校验失败、元数据保存失败或 active profile 激活失败时返回非 0。
- `clashctl sub update --auto` 对已有 `clashctl sub update` 但缺少 `--cron` 的 cron 行会自动修复；只有注释行存在时仍会新增活动 cron 行。
- `clashctl sub import` 导入本地 YAML 时写入稳定 `Name`、`Updated`、`URL`、`Path`，重复导入应明确失败且不追加重复 profile。
- 同一个本地 YAML 通过相对路径、绝对路径或 `file://` 导入时只能生成一个 profile。
- `clashctl sub use <id>` 在合并失败时不改变旧 `config.yaml`、旧 `runtime.yaml` 和 active profile。
- `clashctl sub use <id>` 在旧服务运行且新服务启动失败时，旧配置被恢复，旧 active profile 不变，旧服务被重新启动。
- 订阅更新失败后，当前 profile 文件和 runtime 配置保持不变。
- `clashctl sub update <id>` 在元数据保存失败时不留下半更新 profile 文件。
- `clashctl sub update <active-id>` 在激活失败时不改变旧 profile 文件、旧 `profiles.yaml`、旧 `config.yaml` 和旧 `runtime.yaml`。
- `clashctl sub remove <id>` 在元数据保存失败时不删除 profile 文件，也不改变 `profiles.yaml`。
- `clashctl sub list` 的 Updated 字段在成功更新后变化。

### P0-5 原子配置合并与回滚

现状问题：

- `MergeConfig` 直接写 `runtime.yaml`，校验失败后再尝试恢复。
- `config.yaml` 缺失时直接写入 `{}`，忽略错误。
- 自动修复通过字符串替换 `MATCH,<group>`，容易误伤。

目标：

- 配置合并必须先生成临时 runtime，校验通过后再替换正式 runtime。
- 自动修复必须基于 YAML AST 或明确规则对象，不得全局字符串替换。

必须改动：

- `MergeConfig` 输出到 `runtime.yaml` 同目录下的唯一临时候选文件，禁止使用固定 `runtime.yaml.tmp`。
- `validateConfig` 校验候选临时文件。
- 校验通过后原子替换 `runtime.yaml`。
- 校验失败保留原 `runtime.yaml`。
- `applyProxyGroupFix` 改为解析 `rules` 列表，只修复 `MATCH,<missingGroup>` 的规则项。

验收：

- mixin 写错时，旧 runtime 不变。
- 内核正在运行时，错误配置不会导致下一次 restart 直接不可用。

### P0-6 内核升级必须可验证和可回滚

现状问题：

- `upgrade-kernel` 只选 linux amd64。
- 可能下载 `.gz` 后直接当二进制安装。
- 没有 checksum、签名、备份、回滚。

目标：

- 支持 amd64、arm64、armv7。
- 正确处理 `.gz`。
- 支持 SHA256 校验。
- 替换前备份旧内核。
- 新内核启动失败自动回滚。
- 成功升级必须记录版本、下载 URL、checksum URL、校验结果和安装路径到 `install-state.json`。

必须改动：

- 用 `runtime.GOARCH` 和 `uname -m` 映射 release asset。
- 下载到 `bin/mihomo.new.gz` 或 `bin/mihomo.new`。
- 如为 gzip，解压后再 chmod。
- 若 release 提供 checksums，必须校验；无法校验时输出明确风险并要求 `--allow-unsigned`。
- 替换前复制旧内核为 `mihomo.bak.<version-or-time>`。
- 新内核启动失败时恢复旧内核并重启旧版本。
- `install-state.json` 写入失败时必须停止新内核、恢复旧内核并重启旧版本，不得留下不可审计的成功升级。

验收：

- amd64 和 arm64 asset 选择正确。
- 下载压缩资产不会生成不可执行内核。
- 人为替换为坏二进制时，命令回滚到旧内核。
- 成功升级后 `components.mihomo` 包含 `version`、`source_url`、`checksum_url`、`checksum_verified`、`allow_unsigned`、`path`、`updated_at`；状态写入失败时旧内核和旧状态保持可用。

### P0-7 修复代理环境命令语义

现状问题：

- `clashctl proxy on/off` 只修改当前进程环境，命令退出后对用户 shell 无效。

目标：

- 命令名称和效果一致，不误导用户。

必须改动：

- `clashctl env` 保留，作为 shell eval 的权威方式。
- `clashctl proxy on/off` 改为输出可 eval 脚本，或重命名为 `proxy print-on/print-off`。
- 桌面代理通过 `clashctl proxy desktop on/off/status` 显式支持 GNOME/KDE，并检测 `gsettings`/`kwriteconfig6`/`kwriteconfig5`。

验收：

- `eval "$(clashctl proxy on)"` 生效。
- 单独运行 `clashctl proxy on` 不再声称已修改父 shell。

### P0-8 建立 CI 与最小测试集

目标：

- 每个 PR 自动跑格式、lint、测试和 Linux 构建。
- 核心 bug 必须有回归测试。

必须新增测试：

- `hasProtocol` 短字符串不 panic。
- profile 原子写和读取。
- runtime info YAML 解析。
- API path escape 和 JSON body。
- MergeConfig 失败保留旧 runtime。
- service name 使用 `clashctl` 而非 `mihomo`。
- CLI 进程级退出码：父命令帮助、订阅更新失败、profiles 读取失败、secret show 空值、proxy 非法参数必须返回非 0；空状态展示命令必须保持 0。

CI 必须包含：

- Ubuntu latest。
- Go stable。
- Rust stable。
- Shell syntax check。
- `GOOS=linux GOARCH=amd64 go test ./...`。
- `GOOS=linux GOARCH=amd64 go build ./cmd/clashctl`。
- `scripts/smoke/cli_exit_codes.sh`。

## 5. P1 完善项

P1 是让项目从“不会轻易坏”提升到“真实 Linux 用户长期可用”的改造。

### P1-1 结构化 RuntimeInfo

目标：

- 替换 `readRuntimeInfo`、`readProxyPort`、`readAPIPort` 中的文本扫描。
- `RuntimeInfo` 必须提供唯一的 API 地址推导入口，CLI、doctor、node、upgrade、service wait 不得各自拼接 `127.0.0.1:<port>`。
- `external-controller` 为 `0.0.0.0:<port>`、`[::]:<port>` 或空 host 时，只能转换为本机回环拨号地址，不得把通配监听地址作为客户端目标。
- TUI 必须使用结构化 YAML 读取 `runtime.yaml` 中的 `external-controller`、`secret` 和 `tun.enable`，并采用与 CLI 等价的本机拨号地址规范化规则。
- TUI 读取 `runtime.yaml`、`profiles.yaml` 等资源文件时必须复用同一套安装目录解析顺序：`CLASH_BASE_DIR`、`~/.config/clashctl/install.env`、`/etc/clashctl/install.env`、`~/clashctl`、`~/.clashctl`。
- TUI 触发 `clashctl sub use/update` 等 CLI 子命令时，必须把当前解析出的安装根目录、服务名和内核名分别作为 `CLASH_BASE_DIR`、`SERVICE_NAME`、`KERNEL_NAME` 传给子进程，避免显示和操作落到不同实例或 systemd unit。

设计：

```go
type RuntimeInfo struct {
    MixedPort string
    Port string
    SocksPort string
    ExternalController string
    APIScheme string
    APIHost string
    APIPort string
    Secret string
    TunEnabled bool
}
```

验收：

- YAML 字段有引号、空格、注释、IPv6 地址时解析正确。
- `external-controller: "[::1]:9090"` 不被错误截断。
- `external-controller: "[::1]:9090"` 时 CLI API client 使用 `http://[::1]:9090`。
- `external-controller: "0.0.0.0:9090"` 时 CLI API client 使用 `127.0.0.1:9090` 作为本机拨号目标。
- TUI 在 `external-controller: "[::]:9090"` 时使用 `http://[::1]:9090`，在 `external-controller: "0.0.0.0:9090"` 时使用 `http://127.0.0.1:9090`。
- TUI 订阅列表和 TUN 状态读取自同一个已解析安装目录，不因用户使用自定义 `CLASH_BASE_DIR` 或仅存在系统级 install marker 而显示旧目录数据。
- TUI 从订阅页执行 use/update 时，子进程环境中的 `CLASH_BASE_DIR`、`SERVICE_NAME`、`KERNEL_NAME` 与当前订阅列表来源的安装 marker 一致。

### P1-2 `clashctl doctor`

目标：

新增诊断命令，一次性检查常见痛点。

必须检查：

- 安装路径和 ownership。
- kernel/yq/subconverter 是否存在且可执行。
- systemd unit 是否存在，unit 名是否一致。
- runtime.yaml 是否存在且可通过 Mihomo `-t` 校验。
- API 是否可访问，secret 是否匹配。
- 代理端口是否监听。
- DNS/TUN 状态。
- geosite/geoip/mmdb 是否存在。
- API 是否错误监听公网。
- cron 自动更新是否有效。

验收：

- `clashctl doctor` exit code：0 表示健康，1 表示 warning，2 表示 fatal。
- 输出每项检查结果和修复建议。

### P1-3 TUN 模式可靠化

目标：

- TUN 开关失败不破坏当前运行状态。

必须改动：

- 开启 TUN 前先检查 root/capability、`/dev/net/tun`、`ip` 命令、内核支持。
- sudo/root 执行 `clashctl tun on/off` 时必须解析 `SUDO_USER` 的真实安装目录；缺少安装 marker 时也必须优先查找该用户已有的 `~/clashctl` 或 `~/.clashctl`，不得误判为 `/root/clashctl`。
- SSH 会话中启动或重启 TUN 自动路由必须默认阻断；覆盖必须显式传 `--allow-ssh-tun-risk` 或等价环境变量，并在错误中显示 SSH 客户端和当前路由。
- `start`、`restart`、`tun on`、`sub use`、`upgrade-kernel` 等所有可能启动内核的入口都必须复用 SSH/TUN 风险判断。
- 修改 mixin 前备份。
- TUN 开关必须使用结构化 YAML 写入，不得通过外部 `yq -i` 或字符串替换直接改 `mixin.yaml`。
- 新 runtime 校验通过后才重启。
- 重启失败自动恢复 mixin 和 runtime，并拉起旧配置。
- `verifyTunDevice` 不应只靠名称包含 `tun`，应结合 Mihomo 配置和 `ip tuntap`/`ip link`。

验收：

- 无 `/dev/net/tun` 时命令失败且旧代理继续运行。
- capability 缺失时给出精确修复命令。
- 在 SSH 环境、runtime 启用 `tun.auto-route` 或 `tun.strict-route` 时，未传显式覆盖参数的 `clashctl start/restart/tun on/sub use/upgrade-kernel` 必须在 stop/start 前失败，避免切断当前 SSH。
- `sudo clashctl tun off` 能在没有 `/etc/clashctl/install.env` 的情况下命中当前 `SUDO_USER` 的已有安装目录。

### P1-4 订阅转换器管理

目标：

- `subconverter` 不应靠固定 25500 端口和 `pkill -9 -f` 管理。

必须改动：

- 检测端口占用；如被其他进程占用，选择空闲端口或报错。
- 选择空闲端口时，必须写入受管 `pref.yml` 的 `server.listen`、`server.port` 和 `managed_config.managed_config_prefix`，并在转换结束后恢复用户原配置。
- 启动后记录 PID。
- 停止时只停止自己启动的 PID。
- 启动失败时展示 `latest.log` 尾部关键错误。

验收：

- 系统已有 25500 服务时不会误用或误杀。

### P1-5 日志统一

目标：

- 所有启动方式日志路径一致，`clashctl log` 总能定位。

设计：

- systemd 标准日志：`journalctl -u clashctl`。
- 文件日志：`${CLASH_BASE_DIR}/logs/mihomo.log`。
- nohup fallback 也写入同一路径。

验收：

- systemd 和 nohup 模式下 `clashctl log` 都能看到最近 50 行。

### P1-6 安装脚本幂等和可审计

目标：

- 安装、更新、卸载重复运行结果可预期。

必须改动：

- 所有下载记录版本和 URL 到 `${CLASH_BASE_DIR}/install-state.json`。
- `--force` 不应删除用户订阅，除非显式 `--reset-config`。
- 安装脚本不得静默忽略关键失败。
- 下载失败时明确说明哪个组件失败。

验收：

- 连续执行两次 install 不破坏已配置订阅。
- update 失败不会留下半安装二进制。

### P1-7 本地残留收尾

目标：

- 消除当前仍存在的低到中优先级诊断、审计和展示准确性缺口。
- 这些问题不是当前本地 P0 阻塞，但会影响成熟项目的可审计性、可诊断性和用户信任。

必须改动：

- `clashctl status` 读取 `profiles.yaml` 时不得静默忽略真实读取或解析错误；状态命令仍可保持信息展示语义，但必须至少输出 warning，或提供 `status --check` 作为脚本健康门禁并在 profiles 异常时返回非 0。
- `sub` 操作审计日志写入失败不得完全静默；如果审计日志是可靠性证据，应在交互模式 warning，在 `--cron` 或严格模式中保留失败 exit code，或明确把日志定义为 best-effort 并在 `doctor` 中检查日志目录可写性。
- `sub list` 的代理数量不得继续依赖脆弱行扫描；应使用 YAML 结构化解析统计 `proxies` 列表，解析失败时显示 `unknown` 并给出原因。
- `verifyTunDevice` 不得只依赖 `ip link show` 文本中包含 `tun`/`utun`；应结合配置中的 TUN stack、设备名、`ip tuntap`、`ip link -j` 或等价结构化输出，并在不支持结构化输出时清晰降级。
- `doctor`、`config doctor` 等命令应统一退出路径，减少直接 `os.Exit`，方便单元测试和进程级 smoke 对退出码进行一致验证。

验收：

- 损坏或不可读 `profiles.yaml` 时，`status` 不再静默丢失订阅信息。
- 订阅操作日志目录不可写时，用户能看到明确诊断信息。
- 多行标准 Clash YAML 的 `proxies` 数量显示准确。
- TUN 验证在真实 Linux 上能区分“配置已开启但设备未出现”、“设备存在但非本项目创建”、“系统缺少 TUN 支持”。
- 所有直接退出路径都有单元测试或进程级退出码测试覆盖。

## 6. P2 体验和功能完整性

P2 是产品成熟度提升项，不应早于 P0/P1。

### P2-1 TUI 变成真实控制面

详细整改设计见 `docs/RATATUI_REDESIGN_PLAN.md`。该文档基于当前 `cmd/clashctl` 命令面定义页面、导航、鼠标交互、异步任务、确认弹窗和命令覆盖矩阵。

目标：

- `clash-tui` 必须成为 `clashctl` 的完整终端控制面，而不是只读仪表盘。
- TUI 必须覆盖 Go CLI 提供的日常管理命令，并能完成接近 Windows 桌面客户端的核心体验：状态观察、节点选择、订阅管理、配置管理、TUN、日志、诊断、升级和维护。
- TUI 中显示的交互必须真实作用于 Mihomo 内核、配置文件或 `clashctl` 工作流，不允许只有本地状态。
- 所有可见交互元素必须同时支持键盘和鼠标；页签、按钮、表格行、列表项、输入框、弹窗操作、滚动区域都必须有点击或滚轮语义。

必须完成：

- 建立 TUI 命令覆盖矩阵，将每个 `clashctl` 命令映射到一个页面、表单或弹窗操作；未覆盖项必须在矩阵中标为 deliberate CLI-only 并说明原因。
- 建立统一 action registry：每个 UI 动作有稳定 action id、展示名称、所属页面、危险等级、调用方式、成功判定、失败展示和审计记录。
- 建立统一异步任务模型：所有会阻塞的 API 请求、CLI 子进程、下载、更新、测速和日志读取都必须后台执行，UI 不得卡死；任务进行中必须显示状态、可取消性或不可取消原因。
- 建立统一确认模型：`stop`、`restart`、`tun on/off`、`secret show/set`、`sub remove`、`upgrade`、`upgrade-kernel`、`geodata update`、`desktop proxy on/off` 等会改变系统状态或泄露敏感信息的操作必须有确认弹窗和结果摘要。
- 建立统一错误模型：错误必须包含失败命令/API、退出码或 HTTP status、关键 stderr/body、影响范围、是否已回滚和下一步建议。
- 建立统一鼠标命中模型：渲染阶段记录 hitbox registry，事件阶段根据坐标分派到页签、按钮、表格行、滚动区域、弹窗按钮和输入框，禁止靠散落的坐标 if 判断长期维护。
- 建立统一滚动模型：表格、日志、帮助、诊断结果、配置预览、节点列表和订阅列表均支持滚轮、PageUp/PageDown、Home/End，并保留可见滚动位置。
- 建立统一分页模型：页面清晰分为 Overview、Service、Proxies、Subscriptions、Config、TUN/DNS、Maintenance、Logs、Doctor、Help；窄终端下必须降级为同样可操作的单列布局。
- Overview 页面必须显示内核运行状态、API 连接状态、当前模式、当前订阅、TUN 状态、端口、安全状态、上下行速率、连接数和最近错误；所有摘要项可点击跳转到对应页面。
- Service 页面必须覆盖 `start`、`stop`、`restart`、`status`、`doctor`、`version`、`test`，并显示 systemd/nohup 模式、服务名、内核名、pid、日志路径和最近启动错误。
- Proxies 页面必须覆盖 `node list`、`node switch`、`node delay` 和模式切换；支持搜索、排序、分组展开、节点选择、当前节点标记、延迟测速、批量测速和连接关闭。
- Subscriptions 页面必须覆盖 `sub add`、`sub import`、`sub list`、`sub use`、`sub update`、`sub remove`、`sub log`；支持 URL/path 输入表单、文件路径输入、active 标记、更新时间、失败历史、cron/auto-update 状态和危险删除确认。
- Config 页面必须覆盖 `config view`、`config raw`、`config merge --autofix`、`config set-port`、`config set-api`、`config set-dns-mode`、`config set-lan`、`config doctor`；支持只读预览、表单编辑、diff/结果摘要和失败回滚提示。
- TUN/DNS 页面必须覆盖 `tun status/on/off`、DNS 模式、安全 LAN 暴露检查、`/dev/net/tun`、capability、路由和设备诊断；TUN 开关必须复用 Go CLI 的回滚语义。
- Maintenance 页面必须覆盖 `upgrade`、`upgrade-kernel`、`geodata update`、`proxy on/off` shell env 输出、`proxy desktop status/on/off`、`secret` 状态/设置/显式 reveal；所有外部下载和敏感操作必须记录来源和结果。
- Logs 页面必须覆盖 `log`、`sub log`、本地 TUI 任务日志和最近错误；支持暂停、级别过滤、搜索、清空本地缓冲、滚轮滚动和跳转到最新。
- Doctor 页面必须展示 `doctor` 和 `config doctor` 结果，按 P0/P1/P2 或 fatal/warn/info 分组，并允许点击某项跳转到对应修复页面。
- Help 页面必须从 action registry 生成，确保帮助文案和真实快捷键/鼠标行为一致。
- 搜索过滤代理组/节点/订阅/日志/诊断项必须真实影响可见列表，并保持选中项有效。
- 排序按名称、延迟、更新时间、状态真实生效，不得只改变本地展示后丢失当前选择。
- 代理模式 Rule/Global/Direct 必须调用 Mihomo API，并在 API 失败时不更新本地显示为成功。
- 代理组展开节点列表，选择具体节点后切换。
- 订阅页支持 add/import/update/use/remove，并显示每次操作结果。
- 日志页支持暂停、过滤级别、搜索、清空本地缓冲。
- 错误提示有明确恢复路径。

验收：

- 每一个帮助文字中的快捷键都有真实行为。
- 每一个可见按钮、页签、表格行、列表项、输入框和弹窗按钮都有鼠标点击行为；每一个可滚动区域都有鼠标滚轮行为。
- 用合成 mouse event 的 Rust 测试覆盖页签切换、表格行选择、按钮点击、弹窗确认/取消、日志滚动、列表滚动、输入框聚焦。
- 用 Rust 测试校验 action registry 与 CLI 覆盖矩阵一致；新增 Go CLI 命令时，如果 TUI 未覆盖或未标记 CLI-only，测试必须失败。
- TUI 页面数、命令矩阵、帮助页和快捷键定义来自同一份数据结构，避免帮助文案漂移。
- 任一 mutating action 执行失败时，TUI 不得显示成功状态，必须展示失败原因和是否回滚。
- 在代理组上按 `o` 可展开节点列表，`j/k` 选择节点，`Enter` 调用内核 API 切换到选中的具体节点。
- 鼠标点击代理组可选中，双击或点击“节点”动作可打开节点列表；鼠标点击节点后必须调用同一切换路径。
- 鼠标滚轮在代理组、节点、订阅、连接、日志、诊断和帮助区域中均能滚动当前区域，不误触全局页签。
- 终端宽度不足时，关键按钮和状态不重叠，所有动作仍可通过键盘和鼠标触达。
- TUI 不再显示“已切换模式”但内核未变化的状态。

### P2-2 Release 工程

目标：

- 用户不需要本地 Go/Rust toolchain 也能安装。

必须完成：

- GitHub Actions 构建 release artifacts。
- 提供 `clashctl` 和 `clash-tui` 的 linux amd64/arm64 二进制。
- release 附带 SHA256SUMS。
- installer 优先下载 release artifact，源码构建作为 fallback。

验收：

- 干净 Ubuntu VM 无 Go/Rust 时可完成安装。

### P2-3 配置模板与规则管理

目标：

- 解决 Clash 用户常见的规则、DNS、fake-ip、TUN 配置痛点。

必须完成：

- 提供 `clashctl config doctor`。
- 提供 `clashctl config set-port`、`set-api`、`set-dns-mode`、`set-lan`。
- 提供 `clashctl geodata update`。
- `clashctl geodata update` 必须先把 `Country.mmdb`、`geosite.dat`、`geoip.dat` 全部下载到 staging 目录；只有全部下载成功后才能替换正式资源，任一失败不得留下混合版本。
- `clashctl geodata update` 必须把版本、来源 URL 和资源路径写入 `install-state.json`；状态写入失败时必须回滚已替换的 geodata 文件并返回失败。
- 支持 fake-ip filter 常见局域网域名。
- 支持保留用户自定义 mixin，不被更新覆盖。

验收：

- 用户无需手写 YAML 即可完成常见配置。
- `config set-*` 类命令在 merge 失败时不留下半更新 `mixin.yaml`。

## 7. 目标目录结构

建议逐步演进为：

```text
cmd/clashctl/
internal/api/          # typed Mihomo API client
internal/config/       # env/runtime/mixin/profile parsing and atomic writes
internal/service/      # systemd/nohup service manager
internal/subscription/ # download/convert/update/profile metadata
internal/doctor/       # diagnostics
internal/release/      # GitHub release asset selection/checksum
internal/fsutil/       # atomic write, lock, permissions
resources/
scripts/
tui/
docs/
```

迁移原则：

- 不做大爆炸重写。
- 每个 P0 项独立 PR，带测试。
- 公共工具必须先有测试再迁移调用点。

## 8. 明确禁止事项

- 禁止默认监听 `0.0.0.0` 且 secret 为空。
- 禁止明文默认打印 secret。
- 禁止使用字符串拼接构造 JSON。
- 禁止未 escape 的 URL path 参数。
- 禁止直接覆盖可用内核且无备份。
- 禁止配置校验失败后留下损坏 runtime。
- 禁止 `pkill -9 -f` 杀非本项目启动的长期服务，除非用户显式 `--force-kill`。
- 禁止安装、卸载、更新脚本按 `mihomo`、`clash` 或 `${KERNEL_NAME}` 进程名全局 `pkill`；只能停止本项目 systemd unit、安装目录 pid file 或本项目二进制路径对应的进程。
- 脚本和 Go nohup fallback 从 pid file 读取 PID 后，发送 SIGTERM/SIGKILL 前必须通过 `/proc/<pid>/exe` 或 `/proc/<pid>/cmdline` 确认目标来自当前安装目录的内核二进制。
- Go nohup fallback 不得依赖 `pgrep -f` 或进程名字符串匹配发现内核；缺失 pid file 时只能扫描 `/proc` 并复用可执行文件归属校验。
- Go nohup fallback 的进程探活和停止必须使用平台原生 syscall，不得依赖外部 `kill` 命令；正常停止优先 SIGTERM，超时后才 SIGKILL。
- Go nohup fallback 只有在确认进程已退出后才能删除 pid file 并返回成功；SIGTERM/SIGKILL 发送失败或 SIGKILL 后仍存活必须返回明确错误。
- Linux 进程探活不得把 `/proc/<pid>/stat` 中状态为 `Z` 的 zombie 进程当作仍在运行的内核。
- 禁止帮助文案声称支持但实际无效的 TUI 操作。

## 9. 版本完成定义

### v0.3.0

必须完成全部 P0。

发布条件：

- CI 全绿。
- 新安装默认安全。
- systemd 管理一致。
- 订阅更新不破坏旧配置。
- 内核升级可回滚。

### v0.4.0

必须完成全部 P1。

发布条件：

- `clashctl doctor` 可定位主要环境问题。
- 安装、更新、卸载幂等。
- TUN 失败可恢复。

### v1.0.0

必须完成 P2 中 TUI 控制面和 Release 工程。

发布条件：

- 无 Go/Rust 环境可安装。
- TUI 帮助中列出的操作全部真实可用。
- 常见 Linux 发行版至少覆盖 Ubuntu/Debian/Fedora/Arch 的安装 smoke 验证。
- 至少一个真实 systemd VM 完成真实 Mihomo/yq/geodata 下载、安装、启动、`systemctl status clashctl` 和卸载验证。

## 10. 推荐执行顺序

1. `.gitattributes`、格式化、CI。
2. 服务名统一和日志路径统一。
3. API 安全默认值和 secret 输出修复。
4. API client timeout、escaping、JSON body。
5. 订阅更新 bug 和原子写。
6. runtime 合并原子化。
7. 内核升级校验和回滚。
8. `clashctl doctor`。
9. TUN 可靠化。
10. TUI 真实控制面。
11. Release artifacts。

每完成一步都必须补测试，避免后续重构重新引入同类问题。
