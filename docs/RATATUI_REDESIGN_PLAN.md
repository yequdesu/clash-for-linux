# Ratatui Redesign Plan

本文档定义 `clash-tui` 的详细整改设计。目标是在 Linux 命令行环境中，让 Ratatui 界面覆盖 `cmd/clashctl` 已提供的主要控制能力，并以清晰分区、低心智负担、鼠标和键盘双入口的方式接近桌面 GUI 的易用性。

本文档基于当前源码制定，不依赖旧 README：

- Go 控制面：`cmd/clashctl/*.go`
- 配置和运行状态：`internal/config/*`
- Mihomo API client：`internal/kernel/api.go`、`tui/src/api.rs`
- 当前 TUI：`tui/src/app.rs`、`tui/src/main.rs`、`tui/src/window.rs`、`tui/src/widgets/*`

## 1. 设计结论

### 1.1 保留现有窗口视觉，但替换导航结构

当前 TUI 的优势是已经有一个可移动、可缩放的居中窗口：

- `WindowState` 支持 `x/y/scale`，窗口可移动和缩放。
- 主窗口有边框、顶部标题、底部状态栏。
- 页面内部已经使用 Card/Table/Popup 风格。

整改方案：

- 保留可移动/缩放窗口外壳。
- 取消“顶栏 tab 作为主导航”的结构。
- 改为左侧导航栏 + 顶部上下文栏 + 主工作区 + 底部状态栏。
- 顶部只展示当前位置、搜索、任务状态和关键状态，不承载十几个页面入口。

原因：

- Go CLI 命令面已经远超过当前 5 个 tab。
- 顶栏继续横向扩展会导致标签拥挤、窄终端不可用。
- 左侧导航可以按任务分组，用户更容易理解“我要做什么”。

### 1.2 主布局

默认窗口布局：

```text
+------------------------------------------------------------------------------+
| clash-tui  Dashboard / Service                         API ok  TUN off  ? q  |
+-------------------+----------------------------------------------------------+
| 1 Dashboard       |                                                          |
| 2 Service         |  page title / breadcrumb                                  |
| 3 Proxies         |  +----------------------------------------------------+  |
| 4 Connections     |  | primary panel                                      |  |
| 5 Subscriptions   |  |                                                    |  |
| 6 Config          |  +----------------------------------------------------+  |
| 7 Network & TUN   |  +---------------------------+  +---------------------+  |
| 8 Security        |  | secondary panel           |  | action panel        |  |
| 9 Maintenance     |  |                           |  |                    |  |
| 0 Logs            |  +---------------------------+  +---------------------+  |
| D Doctor          |                                                          |
| H Help            |                                                          |
+-------------------+----------------------------------------------------------+
| ready | click rows/buttons, scroll panels, Ctrl-P command, / search          |
+------------------------------------------------------------------------------+
```

窄终端降级布局，小于 100 列时侧栏收起为数字导航：

```text
+------------------------------------------------------------------------------+
| 1 Dash 2 Svc 3 Proxy 4 Conn 5 Sub 6 Cfg 7 Net 8 Sec 9 Maint 0 Log D Doc H ? |
+------------------------------------------------------------------------------+
| Dashboard > Health                                                           |
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

每个页面只解决一个用户任务，不把所有信息塞进一页：

- Dashboard 只给摘要和下一步入口。
- Service 管生命周期和健康。
- Proxies 管节点、模式、测速。
- Connections 管连接观察和关闭。
- Subscriptions 管订阅。
- Config 管端口、API、DNS、LAN 和配置预览。
- Network & TUN 管 TUN、shell proxy、desktop proxy。
- Security 管 secret 和 API 暴露风险。
- Maintenance 管 geodata、upgrade、upgrade-kernel、install-state。
- Logs 管 kernel log、subscription log、TUI task log。
- Doctor 管诊断结果和修复引导。
- Help 从 action registry 生成。

每个页面最多 3 个主要区域：

- 列表区
- 详情区
- 动作区

复杂操作用弹窗或向导，不在主页面内堆满表单。

## 2. 当前 Go 命令面映射

下表是当前 `cmd/clashctl` 的实际命令面和 TUI 页面归属。`method` 表示 TUI 应优先调用的方式。

| CLI command | Source | TUI page | Action id | Method | Confirm |
| --- | --- | --- | --- | --- | --- |
| `status` | `status.go` | Dashboard / Service | `service.status.refresh` | structured runtime + API + optional CLI | no |
| `start` | `start.go` | Service | `service.start` | `clashctl start` | yes |
| `stop` | `stop.go` | Service | `service.stop` | `clashctl stop` | yes |
| `restart` | `restart.go` | Service | `service.restart` | `clashctl restart` | yes |
| `log` | `log.go` | Logs | `logs.kernel.open` | `clashctl log` or file/journal reader | no |
| `doctor` | `doctor.go` | Doctor / Service | `doctor.run` | `clashctl doctor` | no |
| `version` | `version.go` | Service / Maintenance | `service.version` | direct API + CLI fallback | no |
| `test [url]` | `test.go` | Service | `service.connectivity.test` | `clashctl test [url]` | no |
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
| `config view` | `config_cmd.go` | Config | `config.runtime.view` | read runtime or CLI | no |
| `config raw` | `config_cmd.go` | Config | `config.raw.view` | read config or CLI | no |
| `config merge` | `config_cmd.go` | Config | `config.merge` | `clashctl config merge` | yes |
| `config merge --autofix` | `config_cmd.go` | Config | `config.merge.autofix` | `clashctl config merge --autofix` | yes |
| `config edit` | `config_cmd.go` | Config | `config.edit.external` | launch `$EDITOR` outside TUI after suspend | yes |
| `config set-port <mixed> --http --socks` | `config_cmd.go` | Config | `config.ports.save` | `clashctl config set-port` | form submit |
| `config set-api <host:port> --secret --allow-unsafe` | `config_cmd.go` | Config / Security | `config.api.save` | `clashctl config set-api` | unsafe confirm |
| `config set-dns-mode <mode>` | `config_cmd.go` | Config / Network | `config.dns.save` | `clashctl config set-dns-mode` | form submit |
| `config set-lan <on|off>` | `config_cmd.go` | Config / Network | `config.lan.save` | `clashctl config set-lan` | yes if on |
| `config doctor` | `config_cmd.go` | Doctor / Config | `doctor.config.run` | `clashctl config doctor` | no |
| `tun` | `tun.go` | Network & TUN | `tun.status.refresh` | structured runtime + device probe | no |
| `tun on` | `tun.go` | Network & TUN | `tun.enable` | `clashctl tun on` | dangerous |
| `tun off` | `tun.go` | Network & TUN | `tun.disable` | `clashctl tun off` | yes |
| `proxy` | `proxy.go` | Network & TUN | `proxy.env.status` | environment display | no |
| `proxy on` | `proxy.go` | Network & TUN | `proxy.env.print_on` | `clashctl proxy on` | no |
| `proxy off` | `proxy.go` | Network & TUN | `proxy.env.print_off` | `clashctl proxy off` | no |
| `proxy desktop status` | `proxy_desktop.go` | Network & TUN | `proxy.desktop.status` | `clashctl proxy desktop status` | no |
| `proxy desktop on` | `proxy_desktop.go` | Network & TUN | `proxy.desktop.on` | `clashctl proxy desktop on` | yes |
| `proxy desktop off` | `proxy_desktop.go` | Network & TUN | `proxy.desktop.off` | `clashctl proxy desktop off` | yes |
| `secret` | `secret.go` | Security | `security.secret.status` | runtime parse | no |
| `secret show` | `secret.go` | Security | `security.secret.reveal` | `clashctl secret show` | sensitive |
| `secret <new-secret>` | `secret.go` | Security | `security.secret.set` | `clashctl secret <value>` | sensitive |
| `geodata update --version` | `geodata.go` | Maintenance | `maintenance.geodata.update` | `clashctl geodata update` | yes |
| `upgrade` | `upgrade.go` | Maintenance | `maintenance.api_upgrade` | `clashctl upgrade` | yes |
| `upgrade-kernel --allow-unsigned` | `upgrade_kernel.go` | Maintenance | `maintenance.kernel_upgrade` | `clashctl upgrade-kernel` | dangerous |
| `env` | `env.go` | Network & TUN | `proxy.env.print_on` | same as proxy exports | no |
| `tui` | `tui.go` | Help | deliberate CLI-only | not exposed inside TUI | no |

规则：

- `tui` 自启动命令不需要在 TUI 内作为功能按钮。
- `install.sh`、`update.sh`、`uninstall.sh` 不是 Go CLI 命令，TUI 不直接执行卸载；只在 Maintenance 展示 install-state 和 release 信息。
- 对已经有 direct Mihomo API 的操作，优先 direct API；对涉及文件、systemd、回滚、下载、权限和审计的操作，必须调用 `clashctl`，复用 Go 控制面的安全语义。

## 3. 全局交互模型

### 3.1 输入设备

所有功能必须同时支持键盘和鼠标。

键盘全局约定：

| Key | Behavior |
| --- | --- |
| `1..9,0,D,H` | 切换到对应侧栏页面 |
| `j/k` 或 `Up/Down` | 当前列表上下移动 |
| `h/l` 或 `Left/Right` | 当前页面内区域切换或折叠展开 |
| `Enter` | 执行当前主动作 |
| `Space` | 选择/勾选当前项 |
| `/` | 当前页面搜索 |
| `Ctrl-P` | 打开命令面板 |
| `Esc` | 关闭弹窗、取消搜索、返回上一层 |
| `?` | Help |
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

Tasks:

- Define `Page`, `ActionId`, `DangerLevel`, `Executor`.
- Register every action in Section 2.
- Replace scattered shortcut labels in `render_help` with generated Help.
- Add Rust test: all non CLI-only commands in command inventory have a page and action id.

Acceptance:

- Help page is generated.
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

Acceptance:

- Mouse click on sidebar switches page.
- Keyboard `1..9,0,D,H` switches page.
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

1. Dashboard and Service
2. Logs and Doctor
3. Proxies and Connections
4. Subscriptions
5. Config
6. Network & TUN
7. Security
8. Maintenance

Reason:

- Service/Doctor/Logs improve diagnosis without heavy mutation.
- Proxies/Connections already mostly direct API.
- Subscriptions/Config/TUN/Security/Maintenance mutate files, services or secrets and need confirmation/task/error model first.

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
- sidebar navigation test.
- compact navigation test.
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

## 7. Definition of Done

Ratatui 整改完成必须满足：

- Section 2 中除 deliberate CLI-only 外，每个 Go CLI 命令都有页面、动作和验收规则。
- 所有页面都支持键盘和鼠标。
- 所有可滚动区域都支持滚轮。
- 所有危险操作都有确认弹窗。
- 所有长任务都走异步任务模型。
- 所有失败都进入 Result Drawer，并包含命令/API、退出码或 HTTP status、错误摘要和下一步建议。
- Help、快捷键、按钮和 action registry 一致。
- 真实 Mihomo API 和真实终端交互验证通过后，才能把 TUI 评级提升为成熟控制台。
