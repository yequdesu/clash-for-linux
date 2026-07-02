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
- 一级页面从旧方案的 12 个收敛为 7 个：Subscriptions、Proxies、Connections、Network、Logs、Settings、Help。
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
| 4 Network         |  |                                                    |  |
| 5 Logs            |  +----------------------------------------------------+  |
| 6 Settings        |  +---------------------------+  +---------------------+  |
| ? Help            |  | detail / preview          |  | actions             |  |
|                   |  |                           |  |                    |  |
+-------------------+----------------------------------------------------------+
| ready | click rows/buttons, scroll panels, Ctrl-P command, / search          |
+------------------------------------------------------------------------------+
```

窄终端降级布局，小于 100 列时侧栏收起为数字导航：

```text
+------------------------------------------------------------------------------+
| 1 Sub 2 Proxy 3 Conn 4 Net 5 Log 6 Settings ? Help                           |
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
4. Network：内核启动/停止、桌面代理、TUN、shell proxy 指引、DNS/LAN 摘要。
5. Logs：kernel log、subscription log、TUI task log。
6. Settings：语言、外观、端口、API、DNS、LAN、secret、doctor、geodata、upgrade、install-state。
7. Help：从 action registry 生成，也可通过 `?` 弹窗打开。

页面收敛规则：

- 不再单独设置 Dashboard 页面。健康摘要固定出现在顶部状态栏和每页右上角，需要更多细节时进入 Network 或 Settings / Diagnostics。
- 不再单独设置 Service 页面。生命周期按钮进入 Network，因为用户理解的是“代理是否打开”。
- 不再单独设置 Config、Security、Maintenance、Doctor 一级页面。它们都属于低频设置、诊断或维护，进入统一 Settings 页。
- Network 页面不再命名为 `Network & TUN`。TUN 只是 Network 管辖的一个分块。
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
| General | Core | Ports/API | DNS/LAN | Security | Diagnostics | Updates      |
+------------------------------------------------------------------------------+
| General: language, theme, default page, mouse, refresh interval              |
| Core: service name, kernel name, start/stop/restart fallback info            |
| Ports/API: mixed/http/socks/controller/secret link                           |
| DNS/LAN: fake-ip/redir-host/off, allow-lan                                   |
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
| `doctor` | `doctor.go` | Settings / Diagnostics | `doctor.run` | `clashctl doctor` | no |
| `version` | `version.go` | Settings / Updates | `service.version` | direct API + CLI fallback | no |
| `test [url]` | `test.go` | Settings / Diagnostics | `service.connectivity.test` | `clashctl test [url]` | no |
| `node list` | `node.go` | Proxies | `proxy.groups.refresh` | direct `/proxies` API | no |
| `node switch <group> <node>` | `node.go` | Proxies | `proxy.node.switch` | direct `/proxies/{group}` API | no |
| `node delay [group]` | `node.go` | Proxies | `proxy.delay.test` | direct `/proxies/{name}/delay` API | no |
| `sub list` | `sub_list.go` | Subscriptions | `sub.list.refresh` | read `profiles.yaml` + optional CLI | no |
| `sub add <url|path>` | `sub_add.go` | Subscriptions | `sub.add` | `clashctl sub add` | form submit |
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
| `config doctor` | `config_cmd.go` | Settings / Diagnostics | `doctor.config.run` | `clashctl config doctor` | no |
| `tun` | `tun.go` | Network | `tun.status.refresh` | structured runtime + device probe | no |
| `tun on` | `tun.go` | Network | `tun.enable` | `clashctl tun on` | dangerous |
| `tun off` | `tun.go` | Network | `tun.disable` | `clashctl tun off` | yes |
| `proxy` | `proxy.go` | Network | `proxy.env.status` | environment display | no |
| `proxy on` | `proxy.go` | Network | `proxy.env.print_on` | `clashctl proxy on` | no |
| `proxy off` | `proxy.go` | Network | `proxy.env.print_off` | `clashctl proxy off` | no |
| `proxy desktop status` | `proxy_desktop.go` | Network | `proxy.desktop.status` | `clashctl proxy desktop status` | no |
| `proxy desktop on` | `proxy_desktop.go` | Network | `proxy.desktop.on` | `clashctl proxy desktop on` | yes |
| `proxy desktop off` | `proxy_desktop.go` | Network | `proxy.desktop.off` | `clashctl proxy desktop off` | yes |
| `secret` | `secret.go` | Settings / Security | `security.secret.status` | runtime parse | no |
| `secret show` | `secret.go` | Settings / Security | `security.secret.reveal` | `clashctl secret show` | sensitive |
| `secret <new-secret>` | `secret.go` | Settings / Security | `security.secret.set` | `clashctl secret <value>` | sensitive |
| `geodata update --version` | `geodata.go` | Settings / Updates | `maintenance.geodata.update` | `clashctl geodata update` | yes |
| `upgrade` | `upgrade.go` | Settings / Updates | `maintenance.api_upgrade` | `clashctl upgrade` | yes |
| `upgrade-kernel --allow-unsigned` | `upgrade_kernel.go` | Settings / Updates | `maintenance.kernel_upgrade` | `clashctl upgrade-kernel` | dangerous |
| `env` | `env.go` | Network | `proxy.env.print_on` | same as proxy exports | no |
| `tui` | `tui.go` | Help | deliberate CLI-only | not exposed inside TUI | no |

规则：

- `tui` 自启动命令不需要在 TUI 内作为功能按钮。
- `install.sh`、`update.sh`、`uninstall.sh` 不是 Go CLI 命令，TUI 不直接执行卸载；只在 Settings / Updates 展示 install-state 和 release 信息。
- 对已经有 direct Mihomo API 的操作，优先 direct API；对涉及文件、systemd、回滚、下载、权限和审计的操作，必须调用 `clashctl`，复用 Go 控制面的安全语义。

## 3. 全局交互模型

### 3.1 输入设备

所有功能必须同时支持键盘和鼠标。

键盘全局约定：

| Key | Behavior |
| --- | --- |
| `1..6` | 切换一级页面：Sub、Proxy、Conn、Network、Logs、Settings |
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
  page: Maintenance,
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
| 4 Network         |  |                                                    |  |
| 5 Logs            |  +----------------------------------------------------+  |
| 6 Settings        |  +---------------------------+  +---------------------+  |
| ? Help            |  | details / preview        |  | actions             |  |
+-------------------+----------------------------------------------------------+
| ready | Ctrl-P commands | / search | wheel scrolls hovered panel             |
+------------------------------------------------------------------------------+
```

窄终端：

```text
+------------------------------------------------------------------------------+
| 1 Sub  2 Proxy  3 Conn  4 Network  5 Logs  6 Settings  ?                     |
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

#### 4.0.4 Network

目标：负责“代理是否打开、系统是否走代理、TUN 是否开启、当前 shell 如何设置代理”。页面名只叫 Network，不再叫 `Network & TUN`。

```text
+ Network ---------------------------------------------------------------------+
| Kernel: running  Proxy: :7897  Desktop: off  Shell: manual  TUN: off          |
+-----------------------------+-----------------------------+------------------+
| Core proxy                  | TUN                         | Shell proxy      |
| [Start] [Stop] [Restart]    | Config: disabled            | Current shell    |
| API: ok                     | Device: none                | cannot be edited |
| Proxy port: listening       | SSH route: protected/none   | from inside TUI  |
|                             | [Enable TUN] [Disable TUN]  | [Copy enable]    |
+-----------------------------+-----------------------------+------------------+
| Desktop proxy                                                               |
| GNOME: unknown  KDE: unknown      [Status] [Enable desktop] [Disable desktop]|
+------------------------------------------------------------------------------+
```

规则：

- Start/Stop/Restart 放在 Network，因为用户理解的是开闭代理。
- TUN on/off 必须调用 Go CLI，复用 root、SSH route protection 和回滚语义。
- Shell proxy 不能伪装成可直接修改父 shell，只提供 `eval "$(clashctl proxy on)"` 和关闭命令。
- Desktop proxy on/off 需要确认，因为它会修改用户桌面会话设置。
- DNS/LAN 只显示摘要，修改入口跳到 Settings / DNS/LAN。

#### 4.0.5 Logs

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

#### 4.0.6 Settings

目标：统一设置页。低频、危险、维护、诊断和语言设置都进入这里，不再占一级页面。

```text
+ Settings --------------------------------------------------------------------+
| General | Core | Ports/API | DNS/LAN | Security | Diagnostics | Updates       |
+-----------------------------+--------------------------------+---------------+
| Sections                    | Current section                 | Actions       |
| > General                   | Language [简体中文 v]          | [Save]        |
|   Core                      | Theme    [Default v]           | [Reset]       |
|   Ports/API                 | Default page [Auto v]          |               |
|   DNS/LAN                   | Mouse support [on v]           |               |
|   Security                  | Confirm dangerous [on v]       |               |
|   Diagnostics               | Refresh interval [2s____]      |               |
|   Updates                   |                                |               |
+-----------------------------+--------------------------------+---------------+
```

Settings sections:

- General：language、theme、default page、mouse support、refresh interval、confirm dangerous actions。
- Core：service name、kernel name、init type、base dir、runtime/log path，只允许查看和复制，不直接改 systemd unit。
- Ports/API：mixed/http/socks、external-controller、config merge/autofix、external editor。
- DNS/LAN：DNS mode、allow-lan、LAN exposure warning。
- Security：API secret status、reveal once、set secret、unsafe API listen confirmation。
- Diagnostics：doctor、config doctor、test URL、terminal size/mouse capture checks。
- Updates：geodata update、clashctl upgrade、kernel upgrade、install-state。

#### 4.0.7 Help and Command Palette

Help 默认是弹窗或轻量页面，不与高频页面争抢一级导航空间。

```text
+ Help ------------------------------------------------------------------------+
| Page           Action                         Key       Mouse                 |
| Subscriptions  Use selected subscription      Enter     Use button            |
| Proxies        Switch selected node           Enter     Switch button         |
| Connections    Close selected connection      c         Close button          |
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

### 4.1 Dashboard

目标：打开 TUI 后 5 秒内知道“代理是否可用、下一步做什么”。

不放复杂表格，只放摘要和入口。

```text
+ Dashboard -------------------------------------------------------------------+
| Health: API ok | Kernel running | Proxy :7890 | TUN off | Secret set         |
+-------------------------------+-------------------------------+--------------+
| Current proxy                 | Traffic                       | Next actions |
| Group: Proxy                  | Up:   12 KB/s                 | [Start]      |
| Node : HK-01                  | Down: 220 KB/s                | [Doctor]     |
| Mode : Rule                   | Conn: 18                      | [Add sub]    |
| [Open Proxies] [Test delay]   | [Open Connections]            | [Logs]       |
+-------------------------------+-------------------------------+--------------+
| Subscription                  | Issues                                        |
| Active: MySub                 | [!] geodata missing -> Open Maintenance       |
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
- `r` 刷新 Dashboard。
- `d` 运行 Doctor。

数据来源：

- `status.go` 等价信息来自 `RuntimeInfo`、`ServiceManager`、Mihomo API。
- Issues 来自 `doctor` 和 `config doctor` 的缓存结果。

### 4.2 Service

目标：管理内核生命周期和服务健康。

```text
+ Service ---------------------------------------------------------------------+
| State: running | init: systemd | service: clashctl | kernel: mihomo           |
+-------------------------------+-------------------------------+--------------+
| Lifecycle                    | Service facts                  | Health       |
| [Start] [Stop] [Restart]     | PID: 1234                      | API: ok      |
| [Run doctor] [Test URL]      | Uptime: 01:23:44               | Proxy: ok    |
| Test URL: [generate_204____] | Runtime: .../runtime.yaml      | Config: ok   |
|                              | Log: .../logs/mihomo.log       | Secret: set  |
+------------------------------+--------------------------------+--------------+
| Last result                                                                  |
| clashctl restart -> ok                                                        |
+------------------------------------------------------------------------------+
```

行为：

- Stop/Restart 需要确认。
- Test URL 输入框默认 `http://www.gstatic.com/generate_204`，可编辑。
- Doctor 按钮跳转 Doctor 页面并运行 `doctor`。

命令映射：

- `start` -> `service.start`
- `stop` -> `service.stop`
- `restart` -> `service.restart`
- `status` -> `service.status.refresh`
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
+ Subscriptions ------------------------------------- [Add] [Import] [Refresh] +
| Search: [name/url___________________] Auto update: [status unknown] [Enable] |
+---------------------------------------------------------+--------------------+
| Profiles                                                | Detail             |
| > * 1 MySub        updated 2026-07-02  proxies 128      | ID: 1              |
|     2 Work         updated 2026-07-01  proxies 42       | URL: https://...   |
|     3 LocalTest    updated -           proxies unknown  | Path: .../1.yaml   |
|                                                         | Updated: ...       |
|                                                         | [Use] [Update]     |
|                                                         | [Remove] [Open log]|
+---------------------------------------------------------+--------------------+
| Last operation: sub update 1 -> ok                                           |
+------------------------------------------------------------------------------+
```

Add 弹窗：

```text
+ Add subscription ------------------------------------------------------------+
| URL or file path                                                              |
| [https://example.com/sub.yaml______________________________________________] |
|                                                                              |
| The first subscription will be activated automatically by clashctl.           |
+------------------------------------------------------------------------------+
| [Add] [Cancel]                                                               |
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
- Auto update 写 crontab，需要确认。
- profiles 读取错误必须在页面顶部显示错误条，不显示空列表伪装成功。

命令映射：

- `sub add`、`sub import`、`sub use`、`sub update`、`sub update --auto`、`sub remove`、`sub log`

### 4.6 Config

目标：普通用户不手写 YAML 也能完成常见配置；高级用户可以只读预览 runtime/raw，必要时外部编辑。

页面内采用二级分段，不把所有表单铺开：

```text
+ Config ----------------------------------------------------------------------+
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
+ Config / API ----------------------------------------------------------------+
| External controller [127.0.0.1:9090____________]                             |
| Secret           [set new secret with save] [****************]               |
| Allow unsafe API listen [ ]                                                   |
|                                                                              |
| [Save API] [Open Security]                                                    |
|                                                                              |
| Unsafe addresses require confirmation and explain LAN exposure risk.          |
+------------------------------------------------------------------------------+
```

DNS segment：

```text
+ Config / DNS ----------------------------------------------------------------+
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
+ Config / LAN ----------------------------------------------------------------+
| Allow LAN proxy access [off v]                                                |
|                                                                              |
| [Save LAN]                                                                   |
|                                                                              |
| Turning LAN on exposes the proxy port to the local network.                   |
+------------------------------------------------------------------------------+
```

Preview segment：

```text
+ Config / Preview ----------------------------------- [Runtime] [Mixin] [Raw] +
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

### 4.7 Network & TUN

目标：把 shell proxy、desktop proxy、TUN、DNS/LAN 风险放在同一个网络页面，但用分块避免混淆。

```text
+ Network & TUN ---------------------------------------------------------------+
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

### 4.8 Security

目标：集中处理 secret、API 暴露、安全风险。

```text
+ Security --------------------------------------------------------------------+
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

- Secret value never appears in Dashboard, Config preview, Logs, or Result Drawer unless action is `security.secret.reveal`.
- Reveal result expires when user changes page or after timeout.

### 4.9 Maintenance

目标：维护操作集中管理，全部有来源、版本、校验、回滚说明。

```text
+ Maintenance -----------------------------------------------------------------+
| [Geodata] [Kernel upgrade] [API upgrade] [Install state]                     |
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
- Geodata update shows version/source URL and writes result from CLI output.
- API upgrade (`clashctl upgrade`) is separated from binary kernel upgrade because risk and failure modes differ.

### 4.10 Logs

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

### 4.11 Doctor

目标：诊断结果要可行动，不只是文本。

```text
+ Doctor ------------------------------------------------------ [Run all] -------+
| Filter: [fatal/warn/info v] Search [____________________]                    |
+------------------------------------------+-----------------------------------+
| Issues                                   | Detail                            |
| > [fatal] api secret empty               | Subject: api secret               |
|   [warn ] geodata missing                | Message: empty secret             |
|   [warn ] desktop proxy unsupported      | Suggested action: Open Security   |
|                                          | Command: clashctl secret <value>  |
|                                          | [Open Security] [Copy command]    |
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

### 4.12 Help and Command Palette

Help page generated from Action registry:

```text
+ Help ------------------------------------------------------------------------+
| Page             Action                         Key        Mouse             |
| Dashboard        Refresh                        r          Refresh button    |
| Service          Start kernel                   Enter      Start button      |
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
| maintenance.geodata.update Update geodata databases                           |
| maintenance.kernel_upgrade Upgrade Mihomo kernel binary                       |
+------------------------------------------------------------------------------+
```

Rules:

- Help and command palette must use the same action registry.
- If an action is visible in Help but has no executor or hitbox, tests must fail.

## 5. Implementation plan

### Phase 0: Command inventory and action registry

Files:

- Add `tui/src/actions.rs`
- Add `tui/src/command.rs`
- Add `tui/src/task.rs`
- Add `tui/src/i18n.rs`
- Add `tui/src/settings.rs`

Tasks:

- Define `Page`, `ActionId`, `DangerLevel`, `Executor`.
- Register every action in Section 2.
- Define the authoritative page enum in this exact order: `Subscriptions`, `Proxies`, `Connections`, `Network`, `Logs`, `Settings`, `Help`.
- Define stable i18n message keys for page labels, action labels, button text, status text, errors and Help.
- Persist TUI-only settings: language, theme, default page, mouse support, refresh interval, confirm-dangerous-actions.
- Replace scattered shortcut labels in `render_help` with generated Help.
- Add Rust test: all non CLI-only commands in command inventory have a page and action id.

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
- Keyboard `1..6` switches page.
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
4. Network
5. Logs
6. Settings / General + Core
7. Settings / Ports/API + DNS/LAN
8. Settings / Security + Diagnostics + Updates

Reason:

- Subscriptions and Proxies are the highest-frequency public workflows and should land before low-frequency maintenance UI.
- Connections is the next common operational page and mostly direct API.
- Network owns kernel lifecycle, desktop proxy, shell proxy guidance and TUN, so it replaces the old Service and Network & TUN split.
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
- 一级导航只包含 Subscriptions、Proxies、Connections、Network、Logs、Settings、Help。
- 中英双语由统一 i18n key 驱动，Help、按钮、状态和错误摘要不允许硬编码单语文案。
- 所有页面都支持键盘和鼠标。
- 所有可滚动区域都支持滚轮。
- 所有危险操作都有确认弹窗。
- 所有长任务都走异步任务模型。
- 所有失败都进入 Result Drawer，并包含命令/API、退出码或 HTTP status、错误摘要和下一步建议。
- Help、快捷键、按钮和 action registry 一致。
- 真实 Mihomo API 和真实终端交互验证通过后，才能把 TUI 评级提升为成熟控制台。
