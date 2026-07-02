# Ratatui Redesign Plan

本文档定义 `clash-tui` 的详细整改设计。目标是在 Linux 命令行环境中，让 Ratatui 界面覆盖 `cmd/clashctl` 已提供的主要控制能力，并以清晰分区、低心智负担、鼠标和键盘双入口的方式接近桌面 GUI 的易用性。

本文档基于当前源码制定，不依赖旧 README：

- Go 控制面：`cmd/clashctl/*.go`
- 配置和运行状态：`internal/config/*`
- Mihomo API client：`internal/kernel/api.go`、`tui/src/api.rs`
- 当前 TUI：`tui/src/app.rs`、`tui/src/main.rs`、`tui/src/window.rs`、`tui/src/widgets/*`

## 1. 设计结论

### 1.1 保留现有窗口视觉，但收敛一级页面

当前 TUI 的优势是已经有一个可移动、可缩放的居中窗口：

- `WindowState` 支持 `x/y/scale`，窗口可移动和缩放。
- 主窗口有边框、顶部标题、底部状态栏。
- 页面内部已经使用 Card/Table/Popup 风格。

整改方案：

- 保留可移动/缩放窗口外壳。
- 取消“顶栏 tab 作为长期主导航”的结构。
- 改为左侧导航栏 + 顶部状态栏 + 主工作区 + 底部状态栏。
- 一级页面从旧方案的 12 个收敛为 8 个：Subscriptions、Proxies、Connections、Traffic、Network、Logs、Settings、Help。
- Service、Config、Security、Maintenance、Doctor 不再作为大众用户的一排一级入口，而是进入 Settings 内的分段页面。
- 顶部只展示当前位置、搜索、任务状态和关键状态，不承载十几个页面入口。

原因：

- Go CLI 命令面已经远超过当前 5 个 tab。
- 但大众用户高频任务集中在订阅、节点、连接和网络开关，不能把低频维护项排在前面。
- 顶栏继续横向扩展会导致标签拥挤、窄终端不可用。
- 左侧导航可以按使用频次排序，Settings 可以收纳低频但重要的配置项。

### 1.2 主布局

默认窗口布局：

```text
+------------------------------------------------------------------------------+
| clash-tui  Subscriptions                              API ok  TUN off  ? q   |
+-------------------+----------------------------------------------------------+
| 1 Subscriptions   |  Subscriptions / Active profile                         |
| 2 Proxies         |  +----------------------------------------------------+  |
| 3 Connections     |  | primary list or form                                |  |
| 4 Traffic         |  |                                                    |  |
| 5 Network         |  +----------------------------------------------------+  |
| 6 Logs            |  +---------------------------+  +---------------------+  |
| 7 Settings        |  | detail / preview          |  | actions             |  |
| ? Help            |  |                           |  |                    |  |
|                   |  |                           |  |                    |  |
+-------------------+----------------------------------------------------------+
| ready | click rows/buttons, scroll panels, Ctrl-P command, / search          |
+------------------------------------------------------------------------------+
```

窄终端降级布局，小于 100 列时侧栏收起为数字导航：

```text
+------------------------------------------------------------------------------+
| 1 Sub 2 Proxy 3 Conn 4 Traffic 5 Net 6 Log 7 Settings ? Help                 |
+------------------------------------------------------------------------------+
| Subscriptions > Active profile                                               |
+------------------------------------------------------------------------------+
| primary panel                                                                |
|                                                                              |
| secondary panel                                                              |
|                                                                              |
+------------------------------------------------------------------------------+
| ready | active panel: primary | wheel scrolls hovered panel                  |
+------------------------------------------------------------------------------+
```

### 1.3 页面拆分原则

一级页面按大众使用频次排序：

1. Subscriptions：订阅导入、更新、切换和删除。
2. Proxies：节点选择、模式、测速。
3. Connections：连接观察、关闭单个连接、关闭全部连接。
4. Traffic：实时/历史流量、持久化统计、按规则/节点/路由维度聚合和大视窗图表。
5. Network：内核启动/停止、桌面代理、TUN、shell proxy 指引、DNS/LAN 摘要。
6. Logs：kernel log、subscription log、TUI task log。
7. Settings：语言、外观、端口、API、DNS、LAN、traffic retention、secret、doctor、geodata、upgrade、install-state。
8. Help：从 action registry 生成，也可通过 `?` 弹窗打开。

页面收敛规则：

- 不再单独设置 Dashboard 页面。健康摘要固定出现在顶部状态栏和每页右上角，需要更多细节时进入 Network 或 Settings / Diagnostics。
- 不再单独设置 Service 页面。生命周期按钮进入 Network，因为用户理解的是“代理是否打开”。
- 不再单独设置 Config、Security、Maintenance、Doctor 一级页面。它们都属于低频设置、诊断或维护，进入统一 Settings 页。
- Network 页面只命名为 Network。TUN 只是 Network 管辖的一个分块。
- Traffic 页面独立于 Network。Network 只负责开关和指导；Traffic 负责统计、图表、历史和路由维度分析。
- Logs 保持一级页面，因为事故排查和日常失败反馈都需要快速访问。

每个页面最多 3 个主要区域：

- 列表区
- 详情区
- 动作区

复杂操作用弹窗或向导，不在主页面内堆满表单。

### 1.4 中英双语和统一 Settings

TUI 必须支持中文和英文，并且语言设置必须能在 TUI 内调整。

```text
 Settings / General ----------------------------------------------------------+
| Language          [ 简体中文 v ]                                             |
| Theme             [ Default v ]                                              |
| Default page      [ Auto v ]                                                 |
| Mouse support     [on v]                                                     |
| Confirm dangerous actions [on v]                                             |
|                                                                              |
| [Save settings] [Reset UI settings]                                          |
+------------------------------------------------------------------------------+
```

语言规则：

- 所有页面标题、按钮、状态、错误摘要、Help、Command Palette 都必须来自 i18n message key。
- 默认语言：如果 `LANG` 以 `zh` 开头则用 `zh-CN`，否则用 `en-US`。
- 用户选择写入 TUI 自己的设置文件，不写入 Mihomo runtime。
- 未翻译 key 必须回退英文，并在 debug/task log 中可见，不能显示空白。
- 双语不是主题皮肤；它属于 Settings / General 的核心设置。

Settings 分段：

```text
 Settings --------------------------------------------------------------------+
| General | Core | Ports/API | DNS/LAN | Traffic | Security | Diagnostics | Updates |
+------------------------------------------------------------------------------+
| General: language, theme, default page, mouse, refresh interval              |
| Core: service name, kernel name, start/stop/restart fallback info            |
| Ports/API: mixed/http/socks/controller/secret link                           |
| DNS/LAN: fake-ip/redir-host/off, allow-lan                                   |
| Traffic: sampling interval, retention, storage path, default chart           |
| Security: API exposure, reveal/set secret                                    |
| Diagnostics: doctor, config doctor, test URL                                 |
| Updates: geodata, clashctl upgrade, kernel upgrade, install-state            |
+------------------------------------------------------------------------------+
```

## 2. 当前 Go 命令面映射

下表是当前 `cmd/clashctl` 的实际命令面和 TUI 页面归属。`method` 表示 TUI 应优先调用的方式。

| CLI command | Source | TUI page | Action id | Method | Confirm |
| --- | --- | --- | --- | --- | --- |
| `status` | `status.go` | Network / top status | `service.status.refresh` | structured runtime + API + optional CLI | no |
| `start` | `start.go` | Network | `service.start` | `clashctl start` | yes |
| `stop` | `stop.go` | Network | `service.stop` | `clashctl stop` | yes |
| `restart` | `restart.go` | Network | `service.restart` | `clashctl restart` | yes |
| `log` | `log.go` | Logs | `logs.kernel.open` | `clashctl log` or file/journal reader | no |
| `doctor` | `doctor.go` | Network / Settings Diagnostics | `doctor.run` | `clashctl doctor` | no |
| `version` | `version.go` | Settings / Updates | `service.version` | direct API + CLI fallback | no |
| `test [url]` | `test.go` | Settings / Diagnostics | `service.connectivity.test` | `clashctl test [url]` | no |
| `node list` | `node.go` | Proxies | `proxy.groups.refresh` | direct `/proxies` API | no |
| `node switch <group> <node>` | `node.go` | Proxies | `proxy.node.switch` | direct `/proxies/{group}` API | no |
| `node delay [group]` | `node.go` | Proxies | `proxy.delay.test` | direct `/proxies/{name}/delay` API | no |
| `sub list` | `sub_list.go` | Subscriptions | `sub.list.refresh` | read `profiles.yaml` + optional CLI | no |
| `sub add <url|path> --name --interval --update-proxy --user-agent --convert --tag` | `sub_add.go` | Subscriptions | `sub.add` | `clashctl sub add` | form submit |
| `sub import [directory]` | `sub_import.go` | Subscriptions | `sub.import` | `clashctl sub import` | form submit |
| `sub use <id>` | `sub_use.go` | Subscriptions | `sub.use` | `clashctl sub use` | yes |
| `sub update [id]` | `sub_update.go` | Subscriptions | `sub.update` | `clashctl sub update` | no |
| `sub update --auto` | `sub_update.go` | Subscriptions | `sub.auto_update.enable` | `clashctl sub update --auto` | yes |
| `sub remove <id>` | `sub_remove.go` | Subscriptions | `sub.remove` | `clashctl sub remove` | dangerous |
| `sub log` | `sub_log.go` | Logs / Subscriptions | `logs.subscription.open` | `clashctl sub log` | no |
| `config view` | `config_cmd.go` | Settings / Ports/API | `config.runtime.view` | read runtime or CLI | no |
| `config raw` | `config_cmd.go` | Settings / Ports/API | `config.raw.view` | read config or CLI | no |
| `config merge` | `config_cmd.go` | Settings / Ports/API | `config.merge` | `clashctl config merge` | yes |
| `config merge --autofix` | `config_cmd.go` | Settings / Ports/API | `config.merge.autofix` | `clashctl config merge --autofix` | yes |
| `config edit` | `config_cmd.go` | Settings / Ports/API | `config.edit.external` | launch `$EDITOR` outside TUI after suspend | yes |
| `config set-port <mixed> --http --socks` | `config_cmd.go` | Settings / Ports/API | `config.ports.save` | `clashctl config set-port` | form submit |
| `config set-api <host:port> --secret --allow-unsafe` | `config_cmd.go` | Settings / Security | `config.api.save` | `clashctl config set-api` | unsafe confirm |
| `config set-dns-mode <mode>` | `config_cmd.go` | Settings / DNS/LAN | `config.dns.save` | `clashctl config set-dns-mode` | form submit |
| `config set-lan <on|off>` | `config_cmd.go` | Settings / DNS/LAN | `config.lan.save` | `clashctl config set-lan` | yes if on |
| `config doctor` | `config_cmd.go` | Network / Settings Diagnostics | `doctor.config.run` | `clashctl config doctor` | no |
| `tun` | `tun.go` | Network | `tun.status.refresh` | structured runtime + device probe | no |
| `tun on` | `tun.go` | Network | `tun.enable` | `clashctl tun on` | dangerous |
| `tun off` | `tun.go` | Network | `tun.disable` | `clashctl tun off` | yes |
| `proxy` | `proxy.go` | Network | `network.proxy.on` / `network.proxy.off` | environment display and shell guidance | no |
| `proxy on` | `proxy.go` | Network | `network.proxy.on` | `clashctl proxy on` | no |
| `proxy off` | `proxy.go` | Network | `network.proxy.off` | `clashctl proxy off` | no |
| `proxy desktop status` | `proxy_desktop.go` | Network | `proxy.desktop.status` | `clashctl proxy desktop status` | no |
| `proxy desktop on` | `proxy_desktop.go` | Network | `proxy.desktop.on` | `clashctl proxy desktop on` | yes |
| `proxy desktop off` | `proxy_desktop.go` | Network | `proxy.desktop.off` | `clashctl proxy desktop off` | yes |
| `secret` | `secret.go` | Settings / Security | `security.secret.status` | runtime parse | no |
| `secret show` | `secret.go` | Settings / Security | `security.secret.reveal` | `clashctl secret show` | sensitive |
| `secret <new-secret>` | `secret.go` | Settings / Security | `security.secret.set` | `clashctl secret <value>` | sensitive |
| `geodata update --version` | `geodata.go` | Settings / Updates | `settings.geodata.version` | `clashctl geodata update --version <tag|latest>` | yes |
| `upgrade` | `upgrade.go` | Settings / Updates | `maintenance.api_upgrade` | `clashctl upgrade` | yes |
| `upgrade-kernel --allow-unsigned` | `upgrade_kernel.go` | Settings / Updates | `maintenance.kernel_upgrade` | `clashctl upgrade-kernel` | dangerous |
| `env` | `env.go` | Network | `network.env` | `clashctl env` | no |
| `tui` | `tui.go` | Help | deliberate CLI-only | not exposed inside TUI | no |

规则：

- `tui` 自启动命令不需要在 TUI 内作为功能按钮。
- `install.sh`、`update.sh`、`uninstall.sh` 不是 Go CLI 命令，TUI 不直接执行卸载；只在 Settings / Updates 展示 install-state 和 release 信息。
- 对已经有 direct Mihomo API 的操作，优先 direct API；对涉及文件、systemd、回滚、下载、权限和审计的操作，必须调用 `clashctl`，复用 Go 控制面的安全语义。

### 2.1 必须新增的 Go 命令面

当前源码已经有 profile 元数据字段 `Name/Updated/Interval`，TUI 也能从 `/connections` 拿到 `download_total`、`upload_total`、`rule`、`rulePayload` 和 `chains`。缺口是 Go 侧没有稳定的订阅元数据修改命令、流量采样器、持久化存储和历史查询命令。为了让 TUI 不是只显示本地临时状态，必须先补齐以下 CLI/API 面：

| New CLI command | Source to add | TUI page | Action id | Method | Confirm |
| --- | --- | --- | --- | --- | --- |
| `sub rename <id> <name>` | `cmd/clashctl/sub_edit.go` | Subscriptions | `sub.rename` | `clashctl sub rename` | no |
| `sub set-url <id> <url>` | `cmd/clashctl/sub_edit.go` | Subscriptions | `sub.url.save` | `clashctl sub set-url` | yes |
| `sub set-interval <id> <duration|off>` | `cmd/clashctl/sub_edit.go` | Subscriptions | `sub.interval.save` | `clashctl sub set-interval` | no |
| `sub set-update-proxy <id> <direct|system|core|auto>` | `cmd/clashctl/sub_edit.go` | Subscriptions | `sub.update_proxy.save` | `clashctl sub set-update-proxy` | no |
| `sub set-user-agent <id> <ua>` | `cmd/clashctl/sub_edit.go` | Subscriptions | `sub.user_agent.save` | `clashctl sub set-user-agent` | no |
| `sub set-convert <id> <auto|off|force>` | `cmd/clashctl/sub_edit.go` | Subscriptions | `sub.convert.save` | `clashctl sub set-convert` | no |
| `sub tag add/remove <id> <tag>` | `cmd/clashctl/sub_edit.go` | Subscriptions | `sub.tags.save` | `clashctl sub tag` | no |
| `sub update --scheduled` | `cmd/clashctl/sub_update.go` | Subscriptions | `sub.scheduled_update.run` | cron/systemd timer invokes CLI | no |
| `traffic status` | `cmd/clashctl/traffic.go` | Traffic / top status | `traffic.status.refresh` | local store + Mihomo API | no |
| `traffic sample --once` | `cmd/clashctl/traffic.go` | Traffic | `traffic.sample.once` | local collector core | no |
| `traffic collector start/stop/restart/status` | `cmd/clashctl/traffic_collector.go` | Traffic / Settings | `traffic.collector.manage` | detached collector process with pid/log metadata; future systemd/timer backend | start: no, stop/restart: yes |
| `traffic collect --daemon` | `cmd/clashctl/traffic.go` | Traffic / Settings | `traffic.collect.loop` | collector worker loop invoked by manager or service | yes |
| `traffic history --range <dur> --step <dur> --by <dimension>` | `cmd/clashctl/traffic.go` | Traffic | `traffic.history.query` | local store query | no |
| `traffic top --range <dur> --by rule|route|group|node|host|process` | `cmd/clashctl/traffic.go` | Traffic | `traffic.top.query` | local store query | no |
| `traffic export --format json|csv` | `cmd/clashctl/traffic.go` | Traffic / Settings | `traffic.export` | local store read | no |
| `traffic prune --retention <dur>` | `cmd/clashctl/traffic.go` | Settings / Traffic | `settings.traffic.prune_retention` | local store mutation | yes |
| `traffic reset` | `cmd/clashctl/traffic.go` | Settings / Traffic | `traffic.reset` | local store deletion | dangerous |

新增命令规则：

- 订阅 metadata 修改必须和现有 add/update/use/remove 一样使用文件锁、原子写和回滚；不得让 TUI 直接改 `profiles.yaml`。
- `sub update --scheduled` 不再表示固定 12 小时全量更新，而是读取每个 profile 的 update policy，只更新到期项，并把失败写入 profile `last_error`。
- 流量采样必须是 Go kernel/control 面能力，不依赖 TUI 进程常驻；TUI 只负责查询和展示。
- `traffic` 查询命令默认输出人类可读表格，支持 `--json` 给 TUI 和脚本使用。
- `traffic reset` 必须二次确认；TUI 中只能从 Settings / Traffic 进入，不能放在 Traffic 观察页主动作区。

## 3. 全局交互模型

### 3.1 输入设备

所有功能必须同时支持键盘和鼠标。

键盘全局约定：

| Key | Behavior |
| --- | --- |
| `1..7` | 切换一级页面：Sub、Proxy、Conn、Traffic、Network、Logs、Settings |
| `j/k` 或 `Up/Down` | 当前列表上下移动 |
| `h/l` 或 `Left/Right` | 当前页面内区域切换或折叠展开 |
| `Enter` | 执行当前主动作 |
| `Space` | 选择/勾选当前项 |
| `/` | 当前页面搜索 |
| `Ctrl-P` | 打开命令面板 |
| `Esc` | 关闭弹窗、取消搜索、返回上一层 |
| `?` | Help |
| `Ctrl-S` | 打开 Settings |
| `Ctrl-L` | 切换语言 zh-CN/en-US，实际保存仍走 Settings |
| `r` | 刷新当前页面 |
| `q` | 退出 |
| `PageUp/PageDown` | 当前滚动区域翻页 |
| `Home/End` | 当前滚动区域到顶部/底部 |

鼠标全局约定：

- 点击侧栏项切换页面。
- 点击表格行选中。
- 点击按钮执行动作。
- 点击输入框聚焦。
- 点击弹窗按钮确认/取消。
- 滚轮滚动鼠标所在 panel。
- 右键或长按不作为必须能力，避免终端兼容性问题。
- 双击不作为唯一入口；双击可以加速操作，但必须有单击 + 按钮/Enter 的等价入口。

### 3.2 Hitbox registry

渲染阶段记录所有可交互区域：

```text
Hitbox {
  id: "sub.update.selected",
  rect: Rect,
  kind: Button | NavItem | TableRow | Input | ScrollArea | PopupButton,
  action: ActionId,
  focus_target: Option<FocusId>,
  scroll_target: Option<ScrollId>,
}
```

事件阶段只做分发：

```text
mouse event -> hitbox lookup -> action dispatch / focus / scroll
key event   -> focused widget -> action dispatch
```

禁止在 `main.rs` 中长期堆叠页面坐标判断。坐标判断必须集中在 `mouse.rs` 或等价模块。

### 3.3 Action registry

所有动作必须登记在统一表中：

```text
Action {
  id: "maintenance.kernel_upgrade",
  label: "Upgrade kernel",
  page: Settings,
  section: Updates,
  shortcut: None,
  danger: Dangerous,
  executor: Cli(["upgrade-kernel"]),
  success_rule: ExitCodeZero,
  audit: true,
}
```

用途：

- 生成 Help 页面。
- 生成命令面板。
- 做 CLI 覆盖测试。
- 做鼠标按钮和快捷键一致性校验。
- 做危险操作确认。

### 3.4 异步任务模型

所有可能超过 200ms 的动作都必须后台执行：

- API 请求
- `clashctl` 子进程
- 下载和更新
- 测速
- 日志读取
- doctor

任务栏显示：

```text
Tasks: [sub.update #3 running 00:08] [doctor done] [kernel upgrade failed]
```

任务结果统一进 Result Drawer：

```text
+ Result ----------------------------------------------------------------------+
| action: sub.update                                                            |
| command: clashctl sub update 3                                                |
| exit: 1                                                                       |
| rollback: not needed                                                          |
| stderr: download failed: HTTP 403                                             |
| next: check subscription URL or proxy, then retry                             |
+------------------------------------------------------------------------------+
| [Copy command] [Open logs] [Close]                                            |
+------------------------------------------------------------------------------+
```

### 3.5 危险操作确认

以下操作必须确认：

- stop
- restart
- tun on/off
- sub use
- sub remove
- config merge/autofix
- config set-api with non-loopback or `--allow-unsafe`
- config set-lan on
- secret show
- secret set
- proxy desktop on/off
- geodata update
- upgrade
- upgrade-kernel
- upgrade-kernel --allow-unsigned 必须二次确认

确认弹窗必须展示：

- 将执行的命令
- 影响范围
- 是否有回滚
- 是否需要 root
- 成功判定

## 4. 页面详细设计

### 4.0 V2 权威页面模型

本节取代旧 4.1-4.12 的 12 页一级导航方案。旧草图只作为 panel 细节素材保留，不再作为实现时的页面数量和顺序依据。

参考 Clash Verge Rev 的大众信息架构时，只吸收“Profiles/Proxies/Connections/Settings 高频清晰分组”和“Settings 集中低频配置”的原则；不照搬桌面端视觉、路由命名或页面密度。当前项目仍是 SSH/命令行优先的 Ratatui 控制台。

启动默认页：

- 没有任何订阅：进入 Subscriptions，并聚焦 Add。
- 有订阅且 API 可用：进入 Proxies。
- API 不可用、kernel 未运行或代理端口异常：进入 Network。
- 用户在 Settings / General 设置了固定默认页时，以用户设置优先。

一级导航：

```text
+------------------------------------------------------------------------------+
| clash-tui  Proxies                              API ok | Kernel on | TUN off |
+-------------------+----------------------------------------------------------+
| 1 Subscriptions   |  page title / breadcrumb                                  |
| 2 Proxies         |  +----------------------------------------------------+  |
| 3 Connections     |  | focused page content                               |  |
| 4 Traffic         |  |                                                    |  |
| 5 Network         |  +----------------------------------------------------+  |
| 6 Logs            |  +---------------------------+  +---------------------+  |
| 7 Settings        |  | details / preview        |  | actions             |  |
| ? Help            |  |                          |  |                     |  |
+-------------------+----------------------------------------------------------+
| ready | Ctrl-P commands | / search | wheel scrolls hovered panel             |
+------------------------------------------------------------------------------+
```

窄终端：

```text
+------------------------------------------------------------------------------+
| 1 Sub  2 Proxy  3 Conn  4 Traffic  5 Network  6 Logs  7 Settings  ?          |
+------------------------------------------------------------------------------+
| Proxies > Proxy group                                                         |
+------------------------------------------------------------------------------+
| primary list / form                                                           |
| detail                                                                        |
| actions                                                                       |
+------------------------------------------------------------------------------+
| ready | active: nodes | wheel: hovered panel                                  |
+------------------------------------------------------------------------------+
```

#### 4.0.1 Subscriptions

目标：让新用户第一步就能添加、导入、更新和切换订阅；老用户可以快速看 active profile。

```text
+ Subscriptions ----------------------------------------- [Add] [Import] [Sync] +
| Active: [2] MyAirport                 Auto update: off      Last: 10:31       |
+--------------------------------------+---------------------------------------+
| Profiles                             | Detail                                |
| * [2] MyAirport       58 nodes       | URL: https://...                      |
|   [1] Backup          43 nodes       | Updated: 2026-07-02 10:31             |
|   [3] Local file       9 nodes       | Status: ok                            |
|                                      | Nodes: 58                             |
+--------------------------------------+---------------------------------------+
| Actions: [Use] [Update] [Remove] [Open sub log] [Set auto update]             |
+------------------------------------------------------------------------------+
```

规则：

- Add/Import 使用弹窗表单，不在主页面塞长表单。
- Remove 是 dangerous confirmation。
- Use 调用 `clashctl sub use <id>`，如果 kernel 原本未运行，不在 TUI 中暗示它会启动。
- Update 失败进入 Result Drawer，显示命令、退出码、stdout/stderr 摘要。

#### 4.0.2 Proxies

目标：节点选择、模式切换和测速，是成熟代理客户端的核心页。

```text
+ Proxies --------------------------------------- Mode: [Rule v]  [Test all] ---+
| Search [hong kong________________] Sort [delay v] Refresh [r]                 |
+------------------------------+-----------------------------------------------+
| Groups                       | Nodes in "Proxy"                               |
| > Proxy              HK-01   | > * HK-01                         42 ms       |
|   Auto               JP-02   |     JP-02                         85 ms       |
|   Streaming          SG-01   |     US-01                        180 ms       |
+------------------------------+-----------------------------------------------+
| Details                                      | Actions                         |
| Type: Selector                               | [Switch] [Test selected]        |
| Current: HK-01                               | [Test group]                    |
+------------------------------------------------------------------------------+
```

规则：

- Mode 下拉使用 direct Mihomo API。
- Switch 使用 direct Mihomo API，失败显示 HTTP status 和 body 摘要。
- Groups 和 Nodes 各自独立滚动，鼠标滚轮只滚动 hovered panel。
- Search 同时匹配 group 和 node 名称。

#### 4.0.3 Connections

目标：观察连接、定位异常连接、关闭单个或全部连接。

```text
+ Connections ---------------------------------------------- [Close all] -------+
| Filter [host/rule/node__________] Sort [upload v]                             |
+------------------------------------------------+-----------------------------+
| Connections                                    | Detail                      |
| > github.com:443      HK-01     120 KB  Proxy  | Host: github.com            |
|   google.com:443      JP-02      20 KB  Proxy  | Network: tcp                |
|   192.168.1.10:22     DIRECT      4 KB  Direct | Rule: MATCH                 |
+------------------------------------------------+-----------------------------+
| Actions: [Close selected] [Copy host] [Open Logs]                             |
+------------------------------------------------------------------------------+
```

规则：

- Close selected 和 Close all 都要确认。
- Detail 显示 rule、chain、download/upload、process name（API 有则显示）。
- 搜索和排序必须不改变真实连接状态。

#### 4.0.4 Traffic

目标：提供比桌面客户端更强的流量观察能力，支持实时/历史、持久化、不同维度聚合，并把大而长的图表作为页面主体。

```text
+ Traffic ---------------- Range [1h v]  Chart [Line v]  By [Route v] --------+
| Down 12.4 MB/s  Up 1.2 MB/s  Total 18.2 GB  Collector ok  Retention 30d      |
+------------------------------------------------------------------------------+
|                                                                              |
|  16 MB/s +--------------------------------------------------------------+     |
|          |                         /\                                   |     |
|  12 MB/s |             /\         /  \        download                  |     |
|          |            /  \  /\   /    \                                 |     |
|   8 MB/s |      /\   /    \/  \_/      \___                             |     |
|          |     /  \_/                                                     |   |
|   4 MB/s |___/                                  upload                    |   |
|          +--------------------------------------------------------------+     |
|          14:00            14:15             14:30             14:45           |
|                                                                              |
+---------------------------------------------------------+--------------------+
| Breakdown: route                                       | Detail             |
| > RULE Proxy -> group Proxy -> HK-01       8.2 GB 45%  | Rule: Proxy        |
|   RULE Streaming -> Streaming -> JP-02     4.1 GB 22%  | Payload: geosite   |
|   DIRECT local/wg/lan                      1.5 GB  8%  | Chain: HK-01       |
|                                                         | [Open connections] |
+---------------------------------------------------------+--------------------+
```

规则：

- 图表区必须占可用内容高度的至少 60%，优先保证纵向长度；窄终端下先折叠 Detail，再降低 Breakdown 高度，不压缩主图到不可读。
- Chart 支持 `Line` 和 `Bar`，后续可加 stacked area；首版不得只做 sparkline 小条。
- Range 支持 `5m/1h/6h/24h/7d`，Resolution 支持 `1s/10s/1m/5m`，当查询范围很大时自动选择不超过图表宽度 4 倍的数据点。
- By 维度支持 `total`、`route`、`rule`、`proxy group`、`node`、`host`、`process`。这里的 route 指 Mihomo 路由决策维度，不是 Linux kernel route。
- route 聚合键定义为 `rule + rule_payload + outbound chain/current node`，用于回答“哪条规则/哪个出口产生了多少流量”。
- 鼠标滚轮在图表区滚动历史窗口，在 Breakdown 区滚动排名列表；点击图表点显示该时间片明细，点击 Breakdown 行固定过滤条件。
- TUI 只查询 `clashctl traffic history/top/status --json` 或本地只读 store API；不得只依赖当前 TUI 内存中的 `TrafficHistory`。
- Collector 不可用时仍显示 Mihomo 当前连接总量和即时速率，但历史区域必须标注 `collector stopped`，不得伪装为完整历史。

#### 4.0.5 Network

目标：负责“代理是否打开、系统是否走代理、TUN 是否开启、当前 shell 如何设置代理”。页面名只叫 Network。

```text
+ Network ---------------------------------------------------------------------+
| Kernel: running  Proxy: :7897  Desktop: off  Shell: manual  TUN: off          |
+-----------------------------+-----------------------------+------------------+
| Core proxy                  | TUN                         | Shell proxy      |
| [Start] [Stop] [Restart]    | Config: disabled            | Current shell    |
| API: ok                     | Device: none                | cannot be edited |
| Proxy port: listening       | Route guard: ok/risk/none   | from inside TUI  |
|                             | [Enable TUN] [Disable TUN]  | [Copy enable]    |
+-----------------------------+-----------------------------+------------------+
| Desktop proxy                                                               |
| GNOME: unknown  KDE: unknown      [Status] [Enable desktop] [Disable desktop]|
+------------------------------------------------------------------------------+
| Diagnostics: [Run doctor] [Config doctor] [Test URL]  Last risk: none        |
+------------------------------------------------------------------------------+
```

规则：

- Start/Stop/Restart 放在 Network，因为用户理解的是开闭代理。
- TUN on/off 必须调用 Go CLI，复用 root、route guard 和回滚语义。
- Shell proxy 不能伪装成可直接修改父 shell，只提供 `eval "$(clashctl proxy on)"` 和关闭命令。
- Desktop proxy on/off 需要确认，因为它会修改用户桌面会话设置。
- DNS/LAN 只显示摘要，修改入口跳到 Settings / DNS/LAN。
- Doctor 和 Config doctor 在 Network 页提供就地入口，用于 start/restart/TUN/desktop proxy 前查看安装、权限、route guard、API 和配置风险；完整诊断历史、分组和跳转仍进入 Settings / Diagnostics。

#### 4.0.6 Logs

目标：快速排查失败，支持 kernel/subscription/TUI task 三类日志。

```text
+ Logs --------------------------------------- Source [Kernel v] Level [info v]+
| Search [____________________] Pause [ ] [Jump latest] [Clear local]          |
+------------------------------------------------------------------------------+
| 10:31:02 info  mixed listening at 127.0.0.1:7897                              |
| 10:31:05 warn  geodata missing                                                |
| 10:31:06 error subscription update failed: HTTP 403                           |
+------------------------------------------------------------------------------+
| [Open result drawer] [Copy visible]                                           |
+------------------------------------------------------------------------------+
```

规则：

- Clear 只清 TUI 本地缓冲，不删除文件。
- 鼠标滚轮滚动日志内容。
- Result Drawer 可从任意页面打开，但 Logs 是主要入口。

#### 4.0.7 Settings

目标：统一设置页。低频、危险、维护、诊断和语言设置都进入这里，不再占一级页面。

```text
+ Settings --------------------------------------------------------------------+
| General | Core | Ports/API | DNS/LAN | Traffic | Security | Diagnostics | Updates |
+-----------------------------+--------------------------------+---------------+
| Sections                    | Current section                 | Actions       |
| > General                   | Language [简体中文 v]          | [Save]        |
|   Core                      | Theme    [Default v]           | [Reset]       |
|   Ports/API                 | Default page [Auto v]          |               |
|   DNS/LAN                   | Mouse support [on v]           |               |
|   Traffic                   | Refresh interval [2s____]      |               |
|   Security                  | Confirm dangerous [on v]       |               |
|   Diagnostics               |                                |               |
|   Updates                   |                                |               |
+-----------------------------+--------------------------------+---------------+
```

Settings sections:

- General：language、theme、default page、mouse support、refresh interval、confirm dangerous actions。
- Core：service name、kernel name、init type、base dir、runtime/log path，只允许查看和复制，不直接改 systemd unit。
- Ports/API：mixed/http/socks、external-controller、config merge/autofix、external editor。
- DNS/LAN：DNS mode、allow-lan、LAN exposure warning。
- Traffic：collector status、sampling interval、raw/rollup retention、storage path、default chart/range/dimension、prune/reset/export。
- Security：API secret status、reveal once、set secret、unsafe API listen confirmation。
- Diagnostics：doctor、config doctor、test URL、terminal size/mouse capture checks。
- Updates：geodata update、clashctl upgrade、kernel upgrade、install-state。

#### 4.0.8 Help and Command Palette

Help 默认是弹窗或轻量页面，不与高频页面争抢一级导航空间。

```text
+ Help ------------------------------------------------------------------------+
| Page           Action                         Key       Mouse                 |
| Subscriptions  Use selected subscription      Enter     Use button            |
| Proxies        Switch selected node           Enter     Switch button         |
| Connections    Close selected connection      c         Close button          |
| Traffic        Change chart dimension         -         By selector           |
| Network        Enable TUN                     -         Enable TUN button     |
| Settings       Save settings                  Ctrl-S    Save button           |
+------------------------------------------------------------------------------+
```

Command Palette:

```text
+ Command Palette -------------------------------------------------------------+
| > language                                                                    |
| settings.general.language       Change UI language                            |
| proxy.node.switch               Switch selected node                          |
| sub.add                         Add subscription                              |
+------------------------------------------------------------------------------+
```

规则：

- Help、快捷键、按钮、鼠标 hitbox 必须来自同一个 action registry。
- 每个 action 需要 `label_key` 和 `description_key`，由 i18n 层渲染中英文。
- 如果某个 action 有 Help 文案但没有 executor 或 hitbox，测试必须失败。

以下 4.1-4.13 是对 4.0 权威页面模型的局部展开，不定义新的一级页面。一级页面仍然只有 Subscriptions、Proxies、Connections、Traffic、Network、Logs、Settings、Help。

### 4.1 Top Status Summary

目标：打开 TUI 后 5 秒内从顶部状态栏和当前页摘要知道“代理是否可用、下一步做什么”。不新增 Dashboard 一级页面。

不放复杂表格，只放摘要和入口。

```text
+ Top status details ----------------------------------------------------------+
| Health: API ok | Kernel running | Proxy :7890 | TUN off | Secret set         |
+-------------------------------+-------------------------------+--------------+
| Current proxy                 | Traffic                       | Next actions |
| Group: Proxy                  | Up:   12 KB/s                 | [Start]      |
| Node : HK-01                  | Down: 220 KB/s                | [Doctor]     |
| Mode : Rule                   | Conn: 18                      | [Add sub]    |
| [Open Proxies] [Test delay]   | [Open Traffic] [Connections]  | [Logs]       |
+-------------------------------+-------------------------------+--------------+
| Subscription                  | Issues                                        |
| Active: MySub                 | [!] geodata missing -> Open Settings/Updates |
| Updated: 2026-07-02 10:31     | [!] desktop proxy unsupported -> Open Network |
| [Open Subscriptions]          |                                                |
+------------------------------------------------------------------------------+
```

鼠标：

- 点击状态块跳转对应页面。
- 点击 Next actions 按钮执行或打开弹窗。
- 滚轮在 Issues 内滚动。

键盘：

- `Enter` 默认打开 Proxies。
- `r` 刷新当前页和顶部状态摘要。
- `d` 运行 Doctor。

数据来源：

- `status.go` 等价信息来自 `RuntimeInfo`、`ServiceManager`、Mihomo API。
- Issues 来自 `doctor` 和 `config doctor` 的缓存结果。

### 4.2 Network / Lifecycle

目标：在 Network 页面内管理内核生命周期和服务健康，不新增 Service 一级页面。

```text
+ Network / Lifecycle ---------------------------------------------------------+
| State: running | init: systemd | service: clashctl | kernel: mihomo           |
+-------------------------------+-------------------------------+--------------+
| Lifecycle                    | Service facts                  | Health       |
| [Start] [Stop] [Restart]     | PID: 1234                      | API: ok      |
| [Run doctor] [Config doctor] | Uptime: 01:23:44               | Proxy: ok    |
| [Test URL]                   | Runtime: .../runtime.yaml      | Config: ok   |
| Test URL: [generate_204____] | Log: .../logs/mihomo.log       | Secret: set  |
+------------------------------+--------------------------------+--------------+
| Last result                                                                  |
| clashctl restart -> ok                                                        |
+------------------------------------------------------------------------------+
```

行为：

- Stop/Restart 需要确认。
- Test URL 输入框默认 `http://www.gstatic.com/generate_204`，可编辑。
- Doctor 和 Config doctor 默认在 Network 页结果区运行，用户需要完整诊断分组和历史时可跳转 Settings / Diagnostics。

命令映射：

- `start` -> `service.start`
- `stop` -> `service.stop`
- `restart` -> `service.restart`
- `status` -> `service.status.refresh`
- `doctor` -> `doctor.run`
- `config doctor` -> `doctor.config.run`
- `test [url]` -> `service.connectivity.test`
- `version` -> `service.version`

### 4.3 Proxies

目标：完成节点选择、模式切换、搜索、排序、测速。

不把所有节点平铺到一个表里，采用“代理组列表 + 组内节点列表 + 详情动作”。

```text
+ Proxies -------------------------------------------------- Mode: [Rule v] ----+
| Search: [hong kong________________] Sort: [delay v] [Test all] [Refresh]      |
+------------------------------+-----------------------------------------------+
| Groups                       | Nodes in "Proxy"                               |
| > Proxy              HK-01   | > * HK-01                         42 ms       |
|   Auto               JP-02   |     JP-02                         85 ms       |
|   Streaming          SG-01   |     US-01                        180 ms       |
|   Microsoft          DIRECT  |     DIRECT                         -          |
|                              |                                               |
+------------------------------+-----------------------------------------------+
| Details                                      | Actions                         |
| Group type: Selector                         | [Switch] [Test selected]        |
| Current   : HK-01                            | [Test group] [Close conn by node]|
| API       : /proxies/Proxy                   |                                 |
+------------------------------------------------------------------------------+
```

鼠标：

- 点击 group 行选中 group。
- 点击 node 行选中 node。
- 点击 Switch 调用 direct Mihomo API。
- 滚轮在 group 或 node panel 内滚动，不影响另一个 panel。
- 点击 Mode 下拉打开模式选择弹窗。

键盘：

- `h/l` 在 groups/nodes/actions 间移动。
- `Enter` 在 node panel 执行 Switch。
- `d` 测当前节点延迟。
- `D` 测当前 group。
- `/` 搜索 group 或 node。
- `s` 循环排序。

模式选择弹窗：

```text
+ Select mode -----------------------------------------------------------------+
| ( ) Rule                                                                      |
| ( ) Global                                                                    |
| ( ) Direct                                                                    |
+------------------------------------------------------------------------------+
| [Apply] [Cancel]                                                              |
+------------------------------------------------------------------------------+
```

成功判定：

- `proxy.node.switch` 只有 API 返回 204 才更新本地 current。
- `proxy.mode.set` 只有 API 成功才更新 Mode。

### 4.4 Connections

目标：观察连接，关闭单个或全部连接。

```text
+ Connections ------------------------------------------------ [Close all] -----+
| Filter: [host/process/rule________] Sort: [traffic v] Pause: [ ]             |
+---------------------------------------------------------+--------------------+
| Connections                                             | Detail             |
| > google.com:443      HK-01       120 KB  rule: Proxy   | Host: google.com   |
|   github.com:443      JP-02        20 KB  rule: Proxy   | Network: tcp       |
|   1.1.1.1:53          DIRECT        1 KB  rule: DIRECT  | Chain: HK-01       |
|                                                         | Rule: Proxy        |
|                                                         | [Close selected]   |
+---------------------------------------------------------+--------------------+
```

鼠标：

- 点击连接行选中。
- 滚轮滚动连接表。
- 点击 Close selected 关闭当前连接。
- Close all 必须确认。

命令/API：

- direct `/connections`
- direct `DELETE /connections/{id}`
- direct `DELETE /connections`

### 4.5 Subscriptions

目标：订阅管理要像桌面端“配置文件管理”一样直接，但所有变更复用 Go CLI 回滚语义。

主页面：

```text
+ Subscriptions ------------------------------ [Add] [Import] [Refresh due] ---+
| Search: [name/url/tag________________] Scheduled updates: enabled [Settings] |
+---------------------------------------------------------+--------------------+
| Profiles                                                | Detail             |
| > * 1 MySub        12h  updated 2026-07-02  128 nodes   | ID: 1              |
|     2 Work         off  updated 2026-07-01   42 nodes   | Name: MySub        |
|     3 LocalTest    24h  updated -           unknown     | URL: https://...   |
|                                                         | Path: .../1.yaml   |
|                                                         | Updated: ...       |
|                                                         | Next: 16:31        |
|                                                         | [Use] [Edit]       |
|                                                         | [Update] [Remove]  |
+---------------------------------------------------------+--------------------+
| Last operation: sub update 1 -> ok                                           |
+------------------------------------------------------------------------------+
```

Add 弹窗：

```text
+ Add subscription ------------------------------------------------------------+
| Source URL/path   [https://example.com/sub.yaml_________________________]     |
| Name              [MySub_______________________________________________]     |
| Update interval   [12h________]     use off to disable scheduled updates     |
| Update network    [auto________]    direct/system/core/auto                  |
| Convert mode      [auto________]    auto/off/force                           |
| User-Agent        [clashctl___________________________________________]       |
| Tags              [home,stable________________________________________]       |
+------------------------------------------------------------------------------+
| [Add] [Cancel]                                                               |
+------------------------------------------------------------------------------+
```

Edit profile 弹窗：

```text
+ Edit subscription [1] -------------------------------------------------------+
| Name              [MySub_______________________________________________]     |
| Source URL/path   [https://example.com/sub.yaml_________________________]     |
| Update interval   [12h________]  off disables scheduled updates              |
| Update network    [auto________] direct/system/core/auto                     |
| Convert mode      [auto________] auto/off/force                              |
| User-Agent        [clashctl___________________________________________]       |
| Tags              [home,stable________________________________________]       |
+------------------------------------------------------------------------------+
| [Save] [Cancel]                                                              |
+------------------------------------------------------------------------------+
```

Import 弹窗：

```text
+ Import local profiles -------------------------------------------------------+
| Directory                                                                    |
| [/home/user/clash-profiles_______________________________________________]  |
|                                                                              |
| Imports every .yaml file. Failed files are reported individually.             |
+------------------------------------------------------------------------------+
| [Import] [Cancel]                                                            |
+------------------------------------------------------------------------------+
```

Remove 确认：

```text
+ Remove subscription ---------------------------------------------------------+
| Remove profile [2] Work?                                                      |
| This deletes the profile file and updates profiles.yaml.                      |
| Go CLI will restore the profile if metadata save fails.                       |
+------------------------------------------------------------------------------+
| [Remove] [Cancel]                                                            |
+------------------------------------------------------------------------------+
```

行为：

- Use/Remove 需要确认。
- Update 单个订阅不需要确认，但要显示任务进度。
- Auto update 不再只表示固定 12 小时 cron；它表示启动一个计划任务入口，由 `sub update --scheduled` 根据每个 profile 的策略判断是否到期。
- Add 保存调用 `sub add --name --interval --update-proxy --user-agent --convert --tag`。
- Edit 保存必须按变更顺序调用 `sub rename/set-url/set-interval/set-update-proxy/set-user-agent/set-convert/tag add|remove` 命令，不允许 TUI 直接写 `profiles.yaml`。
- `interval` 旧字段继续读取；新增字段使用 `update_interval`，保存时可兼容写回旧 `interval` 直到迁移完成。
- URL/path 修改需要确认，因为它改变后续下载来源；rename、interval、tag 不需要危险确认。
- 每个 profile 必须显示 `last_error` 和 `next_update`。失败不能永久关闭后续计划更新，除非用户显式把 update enabled 改为 off。
- profiles 读取错误必须在页面顶部显示错误条，不显示空列表伪装成功。

命令映射：

- `sub add <url|path> --name <name> --interval <duration|off> --update-proxy <direct|system|core|auto> --user-agent <ua> --convert <auto|off|force> --tag <tag>`
- `sub import`、`sub use`、`sub update`、`sub update --scheduled`、`sub update --auto`、`sub rename`、`sub set-url`、`sub set-interval`、`sub set-update-proxy`、`sub set-user-agent`、`sub set-convert`、`sub tag`、`sub remove`、`sub log`

### 4.6 Traffic

目标：把流量统计做成项目自己的成熟能力，不只是当前 TUI 首页里短暂的内存 sparkline。用户应该能看到实时速率、历史曲线、长时间总量、按路由/规则/节点/进程/域名的排行，并且退出 TUI 后数据仍然保留。

主页面：

```text
+ Traffic -------------------- [5m] [1h] [6h] [24h] [7d]  [Line v] [Route v] +
| Now: Down 12.4 MB/s  Up 1.2 MB/s | Total: Down 16.8 GB  Up 1.4 GB | 1m avg  |
+------------------------------------------------------------------------------+
|                                                                              |
|  primary chart viewport                                                       |
|                                                                              |
|  The chart owns the vertical space. On 120x40 it should be about 22 lines.    |
|  Mouse wheel over chart moves the time window. Click locks a time bucket.     |
|                                                                              |
|  line mode: upload/download lines                                             |
|  bar mode : bucket bars, grouped by selected dimension                        |
|                                                                              |
+--------------------------------------------------------+---------------------+
| Ranking: route                                        | Bucket detail       |
| > Proxy/geosite:google -> Proxy -> HK-01   8.2 GB     | 14:10 - 14:11      |
|   Streaming/geosite:youtube -> JP-02       3.7 GB     | Down: 560 MB       |
|   DIRECT/private -> DIRECT                 1.2 GB     | Up  : 42 MB        |
|                                                        | Connections: 31    |
| [Filter] [Open matching connections] [Export]          | [Copy row]         |
+--------------------------------------------------------+---------------------+
```

小终端降级：

```text
+ Traffic -------------------- [1h] [Line] [Route] ---------------------------+
| Down 12.4 MB/s  Up 1.2 MB/s  Collector ok                                   |
+------------------------------------------------------------------------------+
|                                                                              |
| chart viewport                                                               |
|                                                                              |
+------------------------------------------------------------------------------+
| Ranking: route                                                               |
| > Proxy/geosite:google -> HK-01  8.2 GB                                      |
|   DIRECT/private -> DIRECT       1.2 GB                                      |
+------------------------------------------------------------------------------+
```

交互规则：

- 顶部时间范围、图表类型、聚合维度都是 segmented/select 控件，支持鼠标点击和键盘左右切换。
- 主图点击某个时间桶后，右侧 Bucket detail 锁定该桶；再次点击空白解除锁定。
- Breakdown/Ranking 支持滚轮、PageUp/PageDown、Home/End；点击行会把该 route/rule/node 作为过滤条件应用到图表。
- `Open matching connections` 跳转 Connections 页，并自动带入当前过滤关键词。
- Export 调用 `clashctl traffic export --format csv|json`，不直接读取内部文件拼接。
- Collector 停止、store 损坏、API 不可用分别显示不同错误状态，并给出 `traffic status` 或 `doctor` 入口。

持久化设计：

```text
~/.clashctl/
  traffic/
    state.json                 # collector cursor, last sample, schema version
    samples.jsonl              # short raw samples, append-only
    rollup-10s.jsonl           # medium retention
    rollup-1m.jsonl            # long retention
    lock                       # collector/store lock
```

字段模型：

```text
TrafficSample:
  ts
  up_bps
  down_bps
  upload_total
  download_total
  active_connections
  breakdown[]:
    dimension      # total/rule/route/group/node/host/process/network
    key
    rule
    rule_payload
    chain
    node
    host
    process
    network
    upload_delta
    download_delta
```

采样来源：

- 优先读 Mihomo `/traffic` 获取实时上传/下载速率；如果接口不可用，退化为 `/connections` 的 `upload_total/download_total` 差值。
- 按 route/rule/node/host/process 的明细来自 `/connections` 中每条连接的 `upload`、`download`、`rule`、`rulePayload`、`chains` 和 metadata。
- Collector 持有上一轮连接 ID -> counters 的游标，只写 delta，避免同一连接累计流量被重复计入。
- 如果连接在两个采样间隔内创建又消失，可能丢失该连接的细分 delta；因此默认 raw sample interval 为 1s，`traffic status` 必须暴露 collector lag。
- TUI 当前的 `TrafficHistory` 只能作为临时 fallback；一旦 Go store 可用，图表必须来自 `traffic history`。

保留策略：

- raw 1s samples 默认保留 24h。
- 10s rollup 默认保留 7d。
- 1m rollup 默认保留 90d。
- 所有保留时间、采样间隔、默认 chart/range/dimension 进入 Settings / Traffic。
- `traffic prune` 只删除超出 retention 的数据；`traffic reset` 删除全部流量库，必须 dangerous confirmation。

路由统计语义：

- “不同路由的流量统计”在本项目中定义为 Mihomo route decision，不是 Linux `ip route`。
- route key = `rule + rule_payload + first meaningful outbound group + leaf node`。
- 当 `chains` 为空时，route key 使用 `rule + rule_payload + DIRECT/UNKNOWN`。
- 当节点切换导致同一 rule 的出口变化时，历史数据保留当时的 chain，不用当前代理组状态回填。

命令映射：

- `traffic status`
- `traffic sample --once`
- `traffic collector start --interval <dur>`
- `traffic collector stop`
- `traffic collector restart --interval <dur>`
- `traffic collector status --json`
- `traffic collect --daemon --interval <dur>`
- `traffic history --range <dur> --step <dur> --by <dimension> --json`
- `traffic top --range <dur> --by route|rule|group|node|host|process --json`
- `traffic export --format json|csv`
- `traffic prune --retention <dur>`
- `traffic reset`

### 4.7 Settings / Ports/API And DNS/LAN

目标：普通用户不手写 YAML 也能完成常见配置；高级用户可以只读预览 runtime/raw，必要时外部编辑。这些内容属于 Settings 页内的 Ports/API、DNS/LAN 分段，不新增 Config 一级页面。

页面内采用二级分段，不把所有表单铺开：

```text
+ Settings / Ports/API --------------------------------------------------------+
| [Ports] [API] [DNS] [LAN] [Preview] [Raw] [Doctor]                           |
+------------------------------------------------------------------------------+
| Ports                                                                        |
| Mixed port [7890 ]   HTTP [0    ]   SOCKS [0    ]                            |
|                                                                              |
| [Save ports] [Merge runtime] [Merge with autofix]                            |
|                                                                              |
| Notes                                                                        |
| - Save uses: clashctl config set-port <mixed> --http <p> --socks <p>         |
| - Merge validates runtime before replacing current runtime.yaml.              |
+------------------------------------------------------------------------------+
```

API segment：

```text
+ Settings / Ports/API / API --------------------------------------------------+
| External controller [127.0.0.1:9090____________]                             |
| Secret           [set new secret with save] [****************]               |
| Allow unsafe API listen [ ]                                                   |
|                                                                              |
| [Save API] [Open Settings / Security]                                         |
|                                                                              |
| Unsafe addresses require confirmation and explain LAN exposure risk.          |
+------------------------------------------------------------------------------+
```

DNS segment：

```text
+ Settings / DNS --------------------------------------------------------------+
| Mode                                                                         |
| ( ) fake-ip                                                                  |
| ( ) redir-host                                                               |
| ( ) off                                                                      |
|                                                                              |
| [Save DNS mode]                                                              |
|                                                                              |
| fake-ip applies default LAN filters from Go config.                           |
+------------------------------------------------------------------------------+
```

LAN segment：

```text
+ Settings / LAN --------------------------------------------------------------+
| Allow LAN proxy access [off v]                                                |
|                                                                              |
| [Save LAN]                                                                   |
|                                                                              |
| Turning LAN on exposes the proxy port to the local network.                   |
+------------------------------------------------------------------------------+
```

Preview segment：

```text
+ Settings / Preview --------------------------------- [Runtime] [Mixin] [Raw] +
| mixed-port: 7890                                                             |
| external-controller: 127.0.0.1:9090                                          |
| secret: ********                                                             |
| ...                                                                          |
+------------------------------------------------------------------------------+
| [Open external editor] [Reload]                                               |
+------------------------------------------------------------------------------+
```

Rules:

- Inline YAML editing is not a P2 requirement. First implementation uses external editor with raw mode suspend/resume.
- Form actions must call `clashctl config set-*` so Go rollback rules are reused.
- Preview must hide secret by default.

### 4.8 Network

目标：把 shell proxy、desktop proxy、TUN、DNS/LAN 风险放在 Network 页面，但用分块避免混淆。页面标题必须只显示 Network。

```text
+ Network ---------------------------------------------------------------------+
| [TUN] [Shell proxy] [Desktop proxy] [DNS/LAN summary]                        |
+-------------------------------+-------------------------------+--------------+
| TUN                           | Prerequisites                 | Actions      |
| Config: enabled               | /dev/net/tun: ok              | [Enable TUN] |
| Device: not detected          | ip command: ok                | [Disable TUN]|
| Route : unknown               | capability: unknown           | [Run doctor] |
|                               |                               |              |
+-------------------------------+-------------------------------+--------------+
| Guidance                                                                     |
| TUN enable requires root. If restart fails, clashctl restores old config.     |
+------------------------------------------------------------------------------+
```

Shell proxy segment：

```text
+ Shell proxy -----------------------------------------------------------------+
| Current parent shell env cannot be changed from TUI.                          |
|                                                                              |
| To enable in current shell:                                                   |
|   eval "$(clashctl proxy on)"                                                 |
|                                                                              |
| [Copy enable command] [Copy disable command]                                  |
+------------------------------------------------------------------------------+
```

Desktop proxy segment：

```text
+ Desktop proxy ---------------------------------------------------------------+
| GNOME: unknown          KDE: configurable via kwriteconfig6/5 if available    |
|                                                                              |
| [Status] [Enable desktop proxy] [Disable desktop proxy]                       |
|                                                                              |
| Runs: clashctl proxy desktop status/on/off                                    |
+------------------------------------------------------------------------------+
```

Rules:

- TUN on/off always uses CLI; direct YAML write in TUI is forbidden.
- TUN on/off shows root requirement before execution.
- Desktop proxy on/off requires confirmation because it changes user session settings.
- Shell proxy is shown as copyable command, not misleading “enabled” toggle.

### 4.9 Settings / Security

目标：在 Settings / Security 集中处理 secret、API 暴露、安全风险，不新增 Security 一级页面。

```text
+ Settings / Security ---------------------------------------------------------+
| API controller: 127.0.0.1:9090    Secret: set    LAN: off                    |
+-------------------------------+-------------------------------+--------------+
| Secret                        | API exposure                  | Actions      |
| Status: set                   | Listen: loopback              | [Set secret] |
| Value : ********              | Risk  : low                   | [Reveal]     |
|                               |                               | [Set API]    |
+-------------------------------+-------------------------------+--------------+
| Doctor warnings                                                               |
| No P0 security risk detected.                                                 |
+------------------------------------------------------------------------------+
```

Reveal confirmation:

```text
+ Reveal API secret -----------------------------------------------------------+
| The secret will be printed inside the terminal UI.                            |
| Do not reveal it while screen sharing or logging the terminal.                |
+------------------------------------------------------------------------------+
| [Reveal once] [Cancel]                                                        |
+------------------------------------------------------------------------------+
```

Set secret form:

```text
+ Set API secret --------------------------------------------------------------+
| New secret [______________________________________________________________]   |
| Restart required after save: yes                                              |
+------------------------------------------------------------------------------+
| [Save] [Cancel]                                                               |
+------------------------------------------------------------------------------+
```

Rules:

- Secret value never appears in top status, Settings preview, Logs, or Result Drawer unless action is `security.secret.reveal`.
- Reveal result expires when user changes page or after timeout.

### 4.10 Settings / Updates

目标：在 Settings / Updates 集中管理维护操作，全部有来源、版本、校验、回滚说明。不新增 Maintenance 一级页面。

```text
+ Settings / Updates ----------------------------------------------------------+
| [Geodata] [Geodata version] [Kernel upgrade] [API upgrade] [Install state]    |
+------------------------------------------------------------------------------+
| Geodata                                                                      |
| Current version: unknown                                                      |
| Source: MetaCubeX/meta-rules-dat                                              |
| Version/tag [latest____________________]                                      |
| Files: Country.mmdb  geosite.dat  geoip.dat                                  |
|                                                                              |
| [Update geodata]                                                             |
|                                                                              |
| Go CLI stages all files before replacement and rolls back on state failure.   |
+------------------------------------------------------------------------------+
```

Kernel upgrade segment:

```text
+ Kernel upgrade --------------------------------------------------------------+
| Current: v1.x from API                                                        |
| Latest : unknown until checked                                                |
| Checksum: required by default                                                 |
| Allow unsigned [ ]                                                            |
|                                                                              |
| [Check latest] [Upgrade kernel]                                               |
+------------------------------------------------------------------------------+
```

Install state segment:

```text
+ Install state ---------------------------------------------------------------+
| Base dir     : /home/user/clashctl                                            |
| Service name : clashctl                                                       |
| Kernel name  : mihomo                                                         |
| Components   : mihomo, yq, geodata, clashctl                                  |
|                                                                              |
| [Reload install-state] [Open Doctor]                                          |
+------------------------------------------------------------------------------+
```

Rules:

- `upgrade-kernel --allow-unsigned` requires two-step confirmation.
- Geodata update shows version/source URL and writes result from CLI output; a version form supports `latest` or a release tag.
- API upgrade (`clashctl upgrade`) is separated from binary kernel upgrade because risk and failure modes differ.

### 4.11 Logs

目标：把日志变成可用的诊断工具，而不是单一滚动文本。

```text
+ Logs ---------------------------------------------------------- Source: Kernel +
| Source [Kernel v] Level [info v] Search [________________] Pause [ ] [Clear] |
+------------------------------------------------------------------------------+
| 10:31:02 info  inbound mixed listening at 127.0.0.1:7890                      |
| 10:31:05 warn  rule provider geosite not found                                |
| 10:31:06 error proxy group missing node                                       |
|                                                                              |
|                                                                              |
+------------------------------------------------------------------------------+
| [Jump latest] [Open result drawer] [Copy visible]                             |
+------------------------------------------------------------------------------+
```

Sources:

- Kernel log: `clashctl log` or log file/journal reader.
- Subscription log: `clashctl sub log`.
- TUI task log: in-memory action results.

Rules:

- Logs page is the only place where wheel scroll behavior from current TUI remains unchanged, but generalized through scroll registry.
- Clear only clears local buffer, never deletes kernel log files.

### 4.12 Settings / Diagnostics

目标：Settings / Diagnostics 内的诊断结果要可行动，不只是文本。不新增 Doctor 一级页面。

```text
+ Settings / Diagnostics -------------------------------------- [Run all] ------+
| Filter: [fatal/warn/info v] Search [____________________]                    |
+------------------------------------------+-----------------------------------+
| Issues                                   | Detail                            |
| > [fatal] api secret empty               | Subject: api secret               |
|   [warn ] geodata missing                | Message: empty secret             |
|   [warn ] desktop proxy unsupported      | Suggested action: Settings/Security|
|                                          | Command: clashctl secret <value>  |
|                                          | [Open Settings/Security] [Copy]   |
+------------------------------------------+-----------------------------------+
```

Doctor sources:

- `clashctl doctor`
- `clashctl config doctor`
- optional TUI-only checks: terminal size, mouse capture, missing `clashctl` binary.

Rules:

- Fatal/warn/info use stable severity.
- Clicking an issue jumps to the page that can fix it.
- Doctor must not mutate state.

### 4.13 Help and Command Palette

Help page generated from Action registry:

```text
+ Help ------------------------------------------------------------------------+
| Page             Action                         Key        Mouse             |
| Network          Refresh status                 r          Refresh button    |
| Network          Start kernel                   Enter      Start button      |
| Proxies          Switch selected node           Enter      Switch button     |
| Subscriptions    Add subscription               a          Add button        |
| Logs             Pause logs                     p          Pause checkbox    |
+------------------------------------------------------------------------------+
```

Command palette:

```text
+ Command palette -------------------------------------------------------------+
| > update subscription                                                         |
|                                                                              |
| sub.update.selected        Update selected subscription                       |
| maintenance.geodata.update Settings/Updates: update geodata databases         |
| maintenance.kernel_upgrade Settings/Updates: upgrade Mihomo kernel binary     |
+------------------------------------------------------------------------------+
```

Rules:

- Help and command palette must use the same action registry.
- If an action is visible in Help but has no executor or hitbox, tests must fail.

## 5. Implementation plan

### Phase 0: Command inventory and action registry

Files:

- Add `tui/src/action_registry.rs`
- Add `tui/src/command.rs`
- Add `tui/src/task.rs`
- Add `tui/src/i18n.rs`
- Add `tui/src/settings.rs`
- Add `cmd/clashctl/sub_edit.go`
- Add `cmd/clashctl/traffic.go`
- Add `internal/traffic/store.go`
- Add `internal/traffic/sample.go`

Tasks:

- Define `Page`, `ActionId`, `DangerLevel`, `Executor`.
- Register every action in Section 2.
- Define the authoritative page enum in this exact order: `Subscriptions`, `Proxies`, `Connections`, `Traffic`, `Network`, `Logs`, `Settings`, `Help`.
- Define stable i18n message keys for page labels, action labels, button text, status text, errors and Help.
- Persist TUI-only settings: language, theme, default page, mouse support, refresh interval, confirm-dangerous-actions.
- Extend profile metadata with update policy fields while reading the existing `interval` field for compatibility.
- Define traffic store schema version, retention defaults, sample/rollup format and route key semantics.
- Replace scattered shortcut labels in `render_help` with generated Help.
- Add Rust test: all non CLI-only commands in command inventory have a page and action id.

Current local status:

- `tui/src/action_registry.rs` 已存在最小实现，登记 `id/page/label/shortcut/mouse/danger/executor`。
- Help 页面已由 action registry 生成；Command Palette 已支持 `Ctrl-P`、输入过滤、选择和执行；Proxies、Connections、Network、Traffic、Logs、Settings 的可见动作按钮已由 action registry 生成。
- Action registry 的 label/button/description 已接入中英双语；Help、Command Palette 和 registry 驱动按钮会按语言设置展示。
- Proxies 节点弹窗已支持鼠标行选择、滚轮移动选中项、Switch/Test/Close registry 按钮，并在弹窗打开时阻断底层表格误点。
- Settings / General 已接入 Language、Theme、Default page、Refresh interval、Mouse、Confirm 的 registry 按钮和快捷键；语言、主题、默认页、刷新间隔、鼠标捕获偏好、危险确认会写入 `settings.yaml`，主题会立即切换终端配色，默认页和鼠标捕获偏好会在下次 TUI 启动时生效，刷新间隔会立即影响自动数据刷新频率，关闭危险确认需要二次确认。
- Settings / Traffic 已接入 `traffic prune --retention` 表单动作，支持 `24h`、`7d`、`raw=24h rollup-10s=7d rollup-1m=90d` 输入并转换为 Go CLI 参数；`traffic prune --retention` 已从参数级 CLI-only 矩阵中移除。
- Settings / Traffic 已接入默认 Range、Chart、By 三项偏好，支持按钮、Settings 页快捷键和 Command Palette，保存到 `settings.yaml` 并影响下次 TUI 启动。
- Settings / Updates 已接入 `geodata update --version` 表单动作，支持 `latest` 或单个 release tag；`geodata update --version` 已从参数级 CLI-only 矩阵中移除。
- Network 已为 `clashctl env` 接入独立 `network.env` 动作，支持按钮、`e` 快捷键、Command Palette、Help 和鼠标点击；该动作只打印可 `eval` 的 shell 环境变量，不伪装成能修改父 shell。
- Network 页主要运行态标题和空状态提示已迁移到统一 i18n key，覆盖 Current Proxy、Actions、System Info、Command Output、No proxy selected 和 shell env 输出提示。
- Connections 页主要运行态标题、表头、空状态和动作提示已迁移到统一 i18n key，覆盖 Host、Type、Chain、No active connections 和 close selected/close all 提示。
- Proxies 页主表、动作区和节点弹窗主要运行态标签已迁移到统一 i18n key，覆盖代理组表头、空状态、节点数量单位、动作提示、节点表头和节点弹窗提示。
- Traffic、Settings、Logs 的主要运行态标签已迁移到统一 i18n key，覆盖 Traffic 摘要/历史图/排行/详情/collector 状态、Settings 卡片/分组/结果区、Logs 标题/空状态/动作提示。
- Settings prompt 已从单行输入升级为字段化结构表单，支持 Tab/上下键切换字段、字段级鼠标点击聚焦、字段级中英双语标签/提示，并在弹窗打开时阻断底层页面动作。
- 状态栏、确认弹窗、Command Palette、Help、Subscriptions 主视图和订阅 add/edit/quick-edit 弹窗的主要可见文案已迁移到统一 i18n key；Help/Command Palette 风险等级会按语言展示；Subscriptions 底部按钮、订阅表单 Save/Cancel、Settings prompt Continue/Cancel、确认弹窗 Confirm/Cancel 的 hitbox 已按翻译后按钮宽度注册。
- Traffic Line 图表已从两行 sparkline 升级为按图表视窗高度绘制的多行主图，Bar 模式继续用于桶状视图。
- Traffic 图表区已注册鼠标 hitbox：滚轮平移历史窗口，点击时间桶锁定 Bucket detail，点击图表空白清除锁定。
- Rust 测试已覆盖 action id 唯一性、Network/Settings/Traffic 的 CLI-backed action enum 均已登记、registry 驱动的 Proxies/Connections/Traffic/Logs/Settings 可见按钮都有 hitbox 分发、Network/Connections/Proxies/Traffic/Settings/Logs 运行态标签翻译、状态栏/全局弹窗/订阅字段翻译、Settings General 主题切换、Settings General 默认页循环、Settings General 刷新间隔设置、Settings / Traffic retention 参数解析、Settings / Traffic 默认视图 key 映射、Settings / Updates geodata version 参数解析、Settings prompt 字段化输入、字段级鼠标聚焦/底层阻断、订阅底部按钮和订阅表单按钮按翻译宽度注册 hitbox、Traffic Line 图表纵向视窗、Traffic 图表滚轮平移/点击桶锁定、节点弹窗鼠标选择/滚轮/底层阻断、Command Palette 的过滤/切页/Settings 表单执行、所有 registry action 均有中文 label/button/description、Command Palette 中文搜索、从 Go Cobra 源码反推命令树并校验 `cli_coverage` 矩阵无遗漏/无陈旧项，以及 `cli_variant_coverage` 参数变体映射/CLI-only 审计；当前 112 个 Rust 测试通过。
- `config set-api` 表单已支持 `controller=... secret=... allow-unsafe=true` 和 `host:port --secret value --allow-unsafe` 两种输入风格，确认弹窗会遮蔽 secret。
- 仍需继续迁移低频状态/错误消息到 i18n key；Section 2 剩余参数级缺口集中在 route-risk/unsigned 风险覆盖项的 CLI-only 策略，以及外部编辑器命令的 TUI suspend 策略。

Acceptance:

- Help page is generated.
- Switching language changes visible page/action labels without changing Mihomo config.
- Existing 5 pages still render.
- No behavior change required yet.

### Phase 1: Layout shell and navigation

Files:

- Add `tui/src/layout.rs`
- Add `tui/src/navigation.rs`
- Modify `tui/src/widgets/tab_bar.rs` into sidebar or replace with `widgets/nav.rs`

Tasks:

- Keep `WindowState`.
- Replace top tab bar with sidebar navigation for wide layout.
- Add compact top nav for narrow layout.
- Add breadcrumb/context header.
- Keep bottom status bar.
- Add startup default route: no profiles -> Subscriptions; API ok -> Proxies; API unavailable/kernel stopped -> Network; user default overrides.

Acceptance:

- Mouse click on sidebar switches page.
- Keyboard `1..7` switches page.
- Window move/zoom continues to work.

### Phase 2: Mouse hitbox and scroll registry

Files:

- Add `tui/src/mouse.rs`
- Add `tui/src/scroll.rs`

Tasks:

- Store hitboxes per frame.
- Register nav items, buttons, table rows, inputs, popup buttons.
- Generalize wheel scrolling from Logs-only to hovered scroll area.

Acceptance:

- Unit tests synthesize mouse events for nav click, row select, button click, popup confirm/cancel and panel scroll.
- No page needs custom long-term mouse coordinate branching in `main.rs`.

### Phase 3: Generic CLI executor

Current TUI only has `run_clashctl_sub(action, id)`.

Required:

```text
run_clashctl(args: &[String]) -> TaskResult
```

It must:

- use `/usr/local/bin/clashctl` fallback exactly as current code does,
- pass `CLASH_BASE_DIR`, `SERVICE_NAME`, `KERNEL_NAME`,
- capture stdout/stderr,
- retain exit code,
- redact secret-like output unless action explicitly permits reveal,
- feed Result Drawer and task log.

Acceptance:

- Existing subscription use/update calls migrate to generic executor without losing environment propagation.
- Failed CLI command shows command, exit code, stderr/stdout snippet.

### Phase 4: Page migration by risk order

Order:

1. Subscriptions
2. Proxies
3. Connections
4. Traffic
5. Network
6. Logs
7. Settings / General + Core
8. Settings / Ports/API + DNS/LAN + Traffic
9. Settings / Security + Diagnostics + Updates

Reason:

- Subscriptions and Proxies are the highest-frequency public workflows and should land before low-frequency maintenance UI.
- Connections is the next common operational page and mostly direct API.
- Traffic follows Connections because it reuses `/connections` detail, but it needs Go-side persistence before the final TUI chart is considered complete.
- Network owns kernel lifecycle, desktop proxy, shell proxy guidance and TUN, so it replaces the old split lifecycle/network model.
- Settings sections mutate files, services, secrets or downloads and need confirmation/task/error model first.

### Phase 5: Validation and external testing

Required local checks:

```bash
cd tui
cargo fmt --check
cargo test
cargo clippy -- -D warnings
```

Required behavior tests:

- action registry coverage test.
- i18n fallback test for missing zh-CN key.
- settings persistence test.
- sidebar navigation test.
- compact navigation test.
- startup default page test.
- mouse hitbox tests.
- scroll registry tests.
- confirmation dialog tests.
- CLI executor redaction test.
- result drawer failure display test.
- subscription metadata edit test: rename, interval, update proxy, user-agent, convert mode and tags are persisted atomically.
- scheduled subscription update test: only due profiles are updated and failures write `last_error` without disabling future runs.
- traffic sampler test: connection counter deltas are not double counted across samples.
- traffic route aggregation test: `rule + rule_payload + chain/node` yields stable historical route keys.
- traffic store rollup/prune test: raw, 10s and 1m retention policies delete only expired data.
- traffic chart layout test: the chart viewport keeps at least 60% of page height in supported terminal sizes.
- traffic mouse test: chart wheel pans history, breakdown wheel scrolls rows, row click filters the chart.

Required real terminal checks:

- 80x24, 100x30, 120x40 and 160x48 layouts.
- mouse click and wheel in a real terminal.
- slow API response.
- Mihomo stopped.
- invalid secret.
- zh-CN and en-US language switching.
- real node switch.
- real subscription update.
- root-required TUN action failure path.

## 6. Non-goals for this redesign

- 不在 TUI 内实现完整 YAML IDE。
- 不在 TUI 内直接修改 systemd unit 文件。
- 不在 TUI 内执行 uninstall。
- 不绕过 Go CLI 的回滚、安全和审计流程。
- 不把鼠标双击作为唯一入口。
- 不把未验证的桌面代理或 TUN 成功状态伪装为已完成。
- 不把 Service、Config、Security、Maintenance、Doctor 重新扩展为一级页面；这些只能作为 Settings 分段或命令面板动作存在。

## 7. Definition of Done

Ratatui 整改完成必须满足：

- Section 2 中除 deliberate CLI-only 外，每个 Go CLI 命令都有页面、动作和验收规则。
- 一级导航只包含 Subscriptions、Proxies、Connections、Traffic、Network、Logs、Settings、Help。
- 中英双语由统一 i18n key 驱动，Help、按钮、状态和错误摘要不允许硬编码单语文案。
- 所有页面都支持键盘和鼠标。
- 所有可滚动区域都支持滚轮。
- 所有危险操作都有确认弹窗。
- 所有长任务都走异步任务模型。
- 所有失败都进入 Result Drawer，并包含命令/API、退出码或 HTTP status、错误摘要和下一步建议。
- Help、快捷键、按钮和 action registry 一致。
- 真实 Mihomo API 和真实终端交互验证通过后，才能把 TUI 评级提升为成熟控制台。
