use crate::settings::LanguageSetting;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Msg {
    PageSubscriptions,
    PageProxies,
    PageConnections,
    PageTraffic,
    PageNetwork,
    PageLogs,
    PageSettings,
    PageHelp,
    SettingsGeneral,
    SettingsLanguage,
    SettingsTheme,
    SettingsDefaultPage,
    SettingsRefreshInterval,
    SettingsMouse,
    SettingsConfirmDanger,
    SettingsCoreApi,
    SettingsTraffic,
    SettingsDiagnosticsUpdates,
    SettingsApi,
    SettingsKernel,
    SettingsNotConnected,
    SettingsDefaultChart,
    SettingsResultsPlaceholder,
    SettingsLastResult,
    SettingsDiagnostics,
    SettingsConfig,
    SettingsForms,
    SettingsSecurity,
    SettingsUpdates,
    CommonEnabled,
    CommonDisabled,
    CommonOn,
    CommonOff,
    CommonConfirm,
    CommonCancel,
    CommonContinue,
    CommonSave,
    CommonRequired,
    CommonOptional,
    CommonDefault,
    CommonEmpty,
    CommonGlobal,
    CommonSafe,
    CommonValue,
    RiskConfirm,
    RiskSensitive,
    RiskDangerous,
    ConfirmHint,
    ConfirmSystemStateWarning,
    SettingsPromptControlsHint,
    StatusSearch,
    StatusQuit,
    StatusSwitch,
    StatusRefresh,
    StatusMode,
    StatusSort,
    SortName,
    SortDelay,
    CommandPaletteTitle,
    CommandPaletteHint,
    CommandPaletteSearchHint,
    CommandPaletteNoMatch,
    HelpKey,
    HelpPage,
    HelpAction,
    HelpRisk,
    HelpDescription,
    HelpMoreActions,
    HelpGenerated,
    HelpActionsUnit,
    HelpCliBackedEnumActions,
    NetworkCurrentProxy,
    NetworkActions,
    NetworkActionsHint,
    NetworkSystemInfo,
    NetworkCommandOutput,
    NetworkNoProxySelected,
    NetworkLastCommandOutput,
    NetworkShellProxyPrinted,
    ProxyNoGroups,
    ProxyGroup,
    ProxyCurrent,
    ProxyDelay,
    ProxyNodes,
    ProxyNodesUnit,
    ProxyActions,
    ProxyActionsHint,
    ProxyTableHint,
    ProxyUnknownGroup,
    ProxyNode,
    ProxyNodePickerHint,
    ProxyNoNodes,
    ConnectionsActive,
    ConnectionsHost,
    ConnectionsType,
    ConnectionsChain,
    ConnectionsNoActive,
    ConnectionsActions,
    ConnectionsActionsHint,
    TrafficRange,
    TrafficChart,
    TrafficBy,
    TrafficFilter,
    TrafficAll,
    TrafficDown,
    TrafficUp,
    TrafficPeak,
    TrafficHistory,
    TrafficAllTraffic,
    TrafficHistoryHint,
    TrafficNoHistory,
    TrafficDownload,
    TrafficUpload,
    TrafficMax,
    TrafficBreakdown,
    TrafficNoBreakdown,
    TrafficTotal,
    TrafficSelected,
    TrafficDetail,
    TrafficDataSource,
    TrafficBucket,
    TrafficRate,
    TrafficStore,
    TrafficSamples,
    TrafficCollector,
    TrafficNever,
    TrafficRunning,
    TrafficStale,
    TrafficStopped,
    TrafficTracked,
    TrafficLast,
    TrafficStatusError,
    LogsActions,
    LogsActionsHint,
    LogsEmpty,
    SubscriptionsId,
    SubscriptionsName,
    SubscriptionsInterval,
    SubscriptionsNext,
    SubscriptionsStatus,
    SubscriptionsUrl,
    SubscriptionsNoProfiles,
    SubscriptionsOutputPlaceholder,
    SubscriptionsStatusError,
    SubscriptionsStatusActive,
    SubscriptionsStatusReady,
    SubscriptionsAddTitle,
    SubscriptionsEditTitle,
    SubscriptionsFormControlsHint,
    SubscriptionsQuickPromptControlsHint,
    SubscriptionsQuickUrl,
    SubscriptionsQuickInterval,
    SubscriptionsQuickUserAgent,
    SubscriptionsQuickUpdateProxy,
    SubscriptionsQuickConvert,
    SubscriptionsQuickAddTag,
    SubscriptionsQuickRemoveTag,
    HelpTitle,
    HelpSwitchTabs,
    HelpNavigateLists,
    HelpRefreshQuitHelp,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SettingsPromptFieldMsg {
    Secret,
    MixedPort,
    HttpPort,
    SocksPort,
    Controller,
    AllowUnsafe,
    DnsMode,
    LanAccess,
    RawRetention,
    Rollup10s,
    Rollup1m,
    Version,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SubscriptionFieldMsg {
    ImportDirectory,
    Source,
    Name,
    Interval,
    UpdateProxy,
    UserAgent,
    ConvertMode,
    Tags,
    AddTag,
    RemoveTag,
}

pub fn tr(language: LanguageSetting, msg: Msg) -> &'static str {
    if language.is_zh() {
        zh(msg)
    } else {
        en(msg)
    }
}

pub fn tr_settings_prompt_field_label(
    language: LanguageSetting,
    field: SettingsPromptFieldMsg,
) -> &'static str {
    if language.is_zh() {
        zh_settings_prompt_field_label(field)
    } else {
        en_settings_prompt_field_label(field)
    }
}

pub fn tr_settings_prompt_field_hint(
    language: LanguageSetting,
    field: SettingsPromptFieldMsg,
) -> &'static str {
    if language.is_zh() {
        zh_settings_prompt_field_hint(field)
    } else {
        en_settings_prompt_field_hint(field)
    }
}

pub fn tr_subscription_field_label(
    language: LanguageSetting,
    field: SubscriptionFieldMsg,
) -> &'static str {
    if language.is_zh() {
        zh_subscription_field_label(field)
    } else {
        en_subscription_field_label(field)
    }
}

pub fn tr_subscription_field_hint(
    language: LanguageSetting,
    field: SubscriptionFieldMsg,
) -> &'static str {
    if language.is_zh() {
        zh_subscription_field_hint(field)
    } else {
        en_subscription_field_hint(field)
    }
}

pub fn tr_action_label(
    language: LanguageSetting,
    action_id: &str,
    fallback: &'static str,
) -> &'static str {
    if language.is_zh() {
        zh_action_label(action_id).unwrap_or(fallback)
    } else {
        en_action_label(action_id).unwrap_or(fallback)
    }
}

pub fn tr_action_desc(
    language: LanguageSetting,
    action_id: &str,
    fallback: &'static str,
) -> &'static str {
    if language.is_zh() {
        zh_action_desc(action_id).unwrap_or(fallback)
    } else {
        en_action_desc(action_id).unwrap_or(fallback)
    }
}

pub fn tr_action_button(
    language: LanguageSetting,
    action_id: &str,
    fallback: &'static str,
) -> &'static str {
    if language.is_zh() {
        zh_action_button(action_id)
            .or_else(|| zh_action_label(action_id))
            .unwrap_or(fallback)
    } else {
        en_action_button(action_id)
            .or_else(|| en_action_label(action_id))
            .unwrap_or(fallback)
    }
}

fn en_settings_prompt_field_label(field: SettingsPromptFieldMsg) -> &'static str {
    match field {
        SettingsPromptFieldMsg::Secret => "Secret",
        SettingsPromptFieldMsg::MixedPort => "Mixed port",
        SettingsPromptFieldMsg::HttpPort => "HTTP port",
        SettingsPromptFieldMsg::SocksPort => "SOCKS port",
        SettingsPromptFieldMsg::Controller => "Controller",
        SettingsPromptFieldMsg::AllowUnsafe => "Allow unsafe",
        SettingsPromptFieldMsg::DnsMode => "DNS mode",
        SettingsPromptFieldMsg::LanAccess => "LAN access",
        SettingsPromptFieldMsg::RawRetention => "Raw retention",
        SettingsPromptFieldMsg::Rollup10s => "10s rollup",
        SettingsPromptFieldMsg::Rollup1m => "1m rollup",
        SettingsPromptFieldMsg::Version => "Version",
    }
}

fn zh_settings_prompt_field_label(field: SettingsPromptFieldMsg) -> &'static str {
    match field {
        SettingsPromptFieldMsg::Secret => "密钥",
        SettingsPromptFieldMsg::MixedPort => "混合端口",
        SettingsPromptFieldMsg::HttpPort => "HTTP 端口",
        SettingsPromptFieldMsg::SocksPort => "SOCKS 端口",
        SettingsPromptFieldMsg::Controller => "控制器",
        SettingsPromptFieldMsg::AllowUnsafe => "允许非安全监听",
        SettingsPromptFieldMsg::DnsMode => "DNS 模式",
        SettingsPromptFieldMsg::LanAccess => "LAN 访问",
        SettingsPromptFieldMsg::RawRetention => "原始保留期",
        SettingsPromptFieldMsg::Rollup10s => "10s 汇总",
        SettingsPromptFieldMsg::Rollup1m => "1m 汇总",
        SettingsPromptFieldMsg::Version => "版本",
    }
}

fn en_settings_prompt_field_hint(field: SettingsPromptFieldMsg) -> &'static str {
    match field {
        SettingsPromptFieldMsg::Secret => "Hidden API secret value.",
        SettingsPromptFieldMsg::MixedPort => "Required. Example: 7897.",
        SettingsPromptFieldMsg::HttpPort => "Optional. Example: 7898.",
        SettingsPromptFieldMsg::SocksPort => "Optional. Example: 7899.",
        SettingsPromptFieldMsg::Controller => "Required. Example: 127.0.0.1:9090.",
        SettingsPromptFieldMsg::AllowUnsafe => {
            "Optional. true/on allows non-loopback controller listeners."
        }
        SettingsPromptFieldMsg::DnsMode => "Allowed: fake-ip, redir-host, off.",
        SettingsPromptFieldMsg::LanAccess => "Allowed: on, off.",
        SettingsPromptFieldMsg::RawRetention => "Required. Example: 24h.",
        SettingsPromptFieldMsg::Rollup10s => "Optional. Example: 7d.",
        SettingsPromptFieldMsg::Rollup1m => "Optional. Example: 90d.",
        SettingsPromptFieldMsg::Version => "Use latest or one release tag.",
    }
}

fn zh_settings_prompt_field_hint(field: SettingsPromptFieldMsg) -> &'static str {
    match field {
        SettingsPromptFieldMsg::Secret => "隐藏的 API 密钥值。",
        SettingsPromptFieldMsg::MixedPort => "必填。例如：7897。",
        SettingsPromptFieldMsg::HttpPort => "可选。例如：7898。",
        SettingsPromptFieldMsg::SocksPort => "可选。例如：7899。",
        SettingsPromptFieldMsg::Controller => "必填。例如：127.0.0.1:9090。",
        SettingsPromptFieldMsg::AllowUnsafe => "可选。true/on 允许非 loopback 控制器监听。",
        SettingsPromptFieldMsg::DnsMode => "允许：fake-ip、redir-host、off。",
        SettingsPromptFieldMsg::LanAccess => "允许：on、off。",
        SettingsPromptFieldMsg::RawRetention => "必填。例如：24h。",
        SettingsPromptFieldMsg::Rollup10s => "可选。例如：7d。",
        SettingsPromptFieldMsg::Rollup1m => "可选。例如：90d。",
        SettingsPromptFieldMsg::Version => "使用 latest 或一个 release tag。",
    }
}

fn en_subscription_field_label(field: SubscriptionFieldMsg) -> &'static str {
    match field {
        SubscriptionFieldMsg::ImportDirectory => "Import directory",
        SubscriptionFieldMsg::Source => "Source URL/path",
        SubscriptionFieldMsg::Name => "Name",
        SubscriptionFieldMsg::Interval => "Update interval",
        SubscriptionFieldMsg::UpdateProxy => "Update proxy",
        SubscriptionFieldMsg::UserAgent => "User-Agent",
        SubscriptionFieldMsg::ConvertMode => "Convert mode",
        SubscriptionFieldMsg::Tags => "Tags",
        SubscriptionFieldMsg::AddTag => "Add tag",
        SubscriptionFieldMsg::RemoveTag => "Remove tag",
    }
}

fn zh_subscription_field_label(field: SubscriptionFieldMsg) -> &'static str {
    match field {
        SubscriptionFieldMsg::ImportDirectory => "导入目录",
        SubscriptionFieldMsg::Source => "来源 URL/路径",
        SubscriptionFieldMsg::Name => "名称",
        SubscriptionFieldMsg::Interval => "更新频率",
        SubscriptionFieldMsg::UpdateProxy => "更新代理",
        SubscriptionFieldMsg::UserAgent => "User-Agent",
        SubscriptionFieldMsg::ConvertMode => "转换模式",
        SubscriptionFieldMsg::Tags => "标签",
        SubscriptionFieldMsg::AddTag => "添加标签",
        SubscriptionFieldMsg::RemoveTag => "移除标签",
    }
}

fn en_subscription_field_hint(field: SubscriptionFieldMsg) -> &'static str {
    match field {
        SubscriptionFieldMsg::ImportDirectory => {
            "Enter a directory containing .yaml subscriptions."
        }
        SubscriptionFieldMsg::Source => {
            "Required. Subscription URL, file:// URL, or local YAML path."
        }
        SubscriptionFieldMsg::Name => "Optional display name.",
        SubscriptionFieldMsg::Interval => "Optional. Use 12h/24h/168h, or off.",
        SubscriptionFieldMsg::UpdateProxy => "Optional. Allowed: direct, system, core, auto.",
        SubscriptionFieldMsg::UserAgent => {
            "Optional. User-Agent used for first download and updates."
        }
        SubscriptionFieldMsg::ConvertMode => "Optional. Allowed: auto, off, force.",
        SubscriptionFieldMsg::Tags => "Optional. Comma-separated tags.",
        SubscriptionFieldMsg::AddTag => "Enter a tag to add.",
        SubscriptionFieldMsg::RemoveTag => "Enter a tag to remove.",
    }
}

fn zh_subscription_field_hint(field: SubscriptionFieldMsg) -> &'static str {
    match field {
        SubscriptionFieldMsg::ImportDirectory => "输入包含 .yaml 订阅的目录。",
        SubscriptionFieldMsg::Source => "必填。订阅 URL、file:// URL 或本地 YAML 路径。",
        SubscriptionFieldMsg::Name => "可选。显示名称。",
        SubscriptionFieldMsg::Interval => "可选。使用 12h/24h/168h，或 off。",
        SubscriptionFieldMsg::UpdateProxy => "可选。允许：direct、system、core、auto。",
        SubscriptionFieldMsg::UserAgent => "可选。首次下载和更新时使用的 User-Agent。",
        SubscriptionFieldMsg::ConvertMode => "可选。允许：auto、off、force。",
        SubscriptionFieldMsg::Tags => "可选。逗号分隔的标签。",
        SubscriptionFieldMsg::AddTag => "输入要添加的标签。",
        SubscriptionFieldMsg::RemoveTag => "输入要移除的标签。",
    }
}

fn en(msg: Msg) -> &'static str {
    match msg {
        Msg::PageSubscriptions => "Subscriptions",
        Msg::PageProxies => "Proxies",
        Msg::PageConnections => "Connections",
        Msg::PageTraffic => "Traffic",
        Msg::PageNetwork => "Network",
        Msg::PageLogs => "Logs",
        Msg::PageSettings => "Settings",
        Msg::PageHelp => "Help",
        Msg::SettingsGeneral => "General",
        Msg::SettingsLanguage => "Language",
        Msg::SettingsTheme => "Theme",
        Msg::SettingsDefaultPage => "Default page",
        Msg::SettingsRefreshInterval => "Refresh",
        Msg::SettingsMouse => "Mouse",
        Msg::SettingsConfirmDanger => "Confirm dangerous actions",
        Msg::SettingsCoreApi => "Core / API",
        Msg::SettingsTraffic => "Traffic",
        Msg::SettingsDiagnosticsUpdates => "Diagnostics / Updates",
        Msg::SettingsApi => "API",
        Msg::SettingsKernel => "Kernel",
        Msg::SettingsNotConnected => "not connected",
        Msg::SettingsDefaultChart => "Default chart",
        Msg::SettingsResultsPlaceholder => "Results appear here after running an action.",
        Msg::SettingsLastResult => "Last result",
        Msg::SettingsDiagnostics => "Diagnostics",
        Msg::SettingsConfig => "Config",
        Msg::SettingsForms => "Forms",
        Msg::SettingsSecurity => "Security",
        Msg::SettingsUpdates => "Updates",
        Msg::CommonEnabled => "enabled",
        Msg::CommonDisabled => "disabled",
        Msg::CommonOn => "on",
        Msg::CommonOff => "off",
        Msg::CommonConfirm => "Confirm",
        Msg::CommonCancel => "Cancel",
        Msg::CommonContinue => "Continue",
        Msg::CommonSave => "Save",
        Msg::CommonRequired => "required",
        Msg::CommonOptional => "optional",
        Msg::CommonDefault => "default",
        Msg::CommonEmpty => "empty",
        Msg::CommonGlobal => "Global",
        Msg::CommonSafe => "safe",
        Msg::CommonValue => "Value",
        Msg::RiskConfirm => "confirm",
        Msg::RiskSensitive => "sensitive",
        Msg::RiskDangerous => "dangerous",
        Msg::ConfirmHint => "Enter/y:confirm  Esc/n:cancel",
        Msg::ConfirmSystemStateWarning => "This action may change system state.",
        Msg::SettingsPromptControlsHint => {
            "Tab/Up/Down:field  Enter:continue  Esc:cancel  Backspace:delete"
        }
        Msg::StatusSearch => "Search",
        Msg::StatusQuit => "Quit",
        Msg::StatusSwitch => "Switch",
        Msg::StatusRefresh => "Refresh",
        Msg::StatusMode => "Mode",
        Msg::StatusSort => "Sort",
        Msg::SortName => "Name",
        Msg::SortDelay => "Delay",
        Msg::CommandPaletteTitle => "Command Palette",
        Msg::CommandPaletteHint => "Ctrl-P toggle  Enter run  Esc close  Up/Down select",
        Msg::CommandPaletteSearchHint => "search id, label, shortcut, page, command",
        Msg::CommandPaletteNoMatch => "No matching action",
        Msg::HelpKey => "Key",
        Msg::HelpPage => "Page",
        Msg::HelpAction => "Action",
        Msg::HelpRisk => "Risk",
        Msg::HelpDescription => "Description",
        Msg::HelpMoreActions => "More actions exist in the registry; enlarge the window to see additional rows.",
        Msg::HelpGenerated => "Help is generated from the TUI action registry",
        Msg::HelpActionsUnit => "actions",
        Msg::HelpCliBackedEnumActions => "CLI-backed enum actions",
        Msg::NetworkCurrentProxy => "Current Proxy",
        Msg::NetworkActions => "Actions",
        Msg::NetworkActionsHint => {
            "click action  Enter:start  x:stop  n:restart  v:status  V:doctor  c:config  t:TUN  e:env  p/P:proxy  d/D/O:desktop"
        }
        Msg::NetworkSystemInfo => "System Info",
        Msg::NetworkCommandOutput => "Command Output",
        Msg::NetworkNoProxySelected => "No proxy selected",
        Msg::NetworkLastCommandOutput => "Last command output appears here.",
        Msg::NetworkShellProxyPrinted => "Shell proxy commands are printed, not evaled inside TUI.",
        Msg::ProxyNoGroups => "No proxy groups found",
        Msg::ProxyGroup => "Group",
        Msg::ProxyCurrent => "Current",
        Msg::ProxyDelay => "Delay",
        Msg::ProxyNodes => "Nodes",
        Msg::ProxyNodesUnit => "nodes",
        Msg::ProxyActions => "Actions",
        Msg::ProxyActionsHint => {
            "registry driven  p:mode  s:sort  o:nodes  Enter/d:test  D:all"
        }
        Msg::ProxyTableHint => "Enter:test  Alt+Enter:next  o:nodes  d:current  D:all  /:search",
        Msg::ProxyUnknownGroup => "Unknown",
        Msg::ProxyNode => "Node",
        Msg::ProxyNodePickerHint => "click row, wheel, Switch/Test/Close",
        Msg::ProxyNoNodes => "No nodes in selected proxy group",
        Msg::ConnectionsActive => "Active",
        Msg::ConnectionsHost => "Host",
        Msg::ConnectionsType => "Type",
        Msg::ConnectionsChain => "Chain",
        Msg::ConnectionsNoActive => "No active connections",
        Msg::ConnectionsActions => "Actions",
        Msg::ConnectionsActionsHint => "registry driven  c:close selected  C:close all",
        Msg::TrafficRange => "Range",
        Msg::TrafficChart => "Chart",
        Msg::TrafficBy => "By",
        Msg::TrafficFilter => "Filter",
        Msg::TrafficAll => "all",
        Msg::TrafficDown => "Down",
        Msg::TrafficUp => "Up",
        Msg::TrafficPeak => "Peak",
        Msg::TrafficHistory => "History",
        Msg::TrafficAllTraffic => "all traffic",
        Msg::TrafficHistoryHint => "[/]:range  m:chart  y:dimension  click:bucket  wheel:pan  e:export",
        Msg::TrafficNoHistory => {
            "No traffic history yet. Run `clashctl traffic sample` or `clashctl traffic collect --daemon`."
        }
        Msg::TrafficDownload => "download",
        Msg::TrafficUpload => "upload",
        Msg::TrafficMax => "max",
        Msg::TrafficBreakdown => "Breakdown",
        Msg::TrafficNoBreakdown => "No traffic breakdown",
        Msg::TrafficTotal => "Total",
        Msg::TrafficSelected => "Selected",
        Msg::TrafficDetail => "Detail",
        Msg::TrafficDataSource => "Traffic data comes from `clashctl traffic history/top --json`.",
        Msg::TrafficBucket => "Bucket",
        Msg::TrafficRate => "rate",
        Msg::TrafficStore => "Store",
        Msg::TrafficSamples => "Samples",
        Msg::TrafficCollector => "Collector",
        Msg::TrafficNever => "never",
        Msg::TrafficRunning => "running",
        Msg::TrafficStale => "stale",
        Msg::TrafficStopped => "stopped",
        Msg::TrafficTracked => "tracked",
        Msg::TrafficLast => "last",
        Msg::TrafficStatusError => "status error",
        Msg::LogsActions => "Actions",
        Msg::LogsActionsHint => "registry driven  p:pause  f:filter  c:clear",
        Msg::LogsEmpty => "No logs yet.",
        Msg::SubscriptionsId => "ID",
        Msg::SubscriptionsName => "Name",
        Msg::SubscriptionsInterval => "Interval",
        Msg::SubscriptionsNext => "Next",
        Msg::SubscriptionsStatus => "Status",
        Msg::SubscriptionsUrl => "URL",
        Msg::SubscriptionsNoProfiles => "No subscriptions found. Press a to add, or I to import a directory.",
        Msg::SubscriptionsOutputPlaceholder => "Subscription command output appears here.",
        Msg::SubscriptionsStatusError => "error",
        Msg::SubscriptionsStatusActive => "active",
        Msg::SubscriptionsStatusReady => "ready",
        Msg::SubscriptionsAddTitle => "Add subscription",
        Msg::SubscriptionsEditTitle => "Edit subscription",
        Msg::SubscriptionsFormControlsHint => {
            "Tab/Down:next field  Shift+Tab/Up:previous field  Enter:save  Esc:cancel"
        }
        Msg::SubscriptionsQuickPromptControlsHint => "Enter:save  Esc:cancel  Backspace:delete",
        Msg::SubscriptionsQuickUrl => "URL",
        Msg::SubscriptionsQuickInterval => "Interval",
        Msg::SubscriptionsQuickUserAgent => "UA",
        Msg::SubscriptionsQuickUpdateProxy => "Proxy",
        Msg::SubscriptionsQuickConvert => "Convert",
        Msg::SubscriptionsQuickAddTag => "+Tag",
        Msg::SubscriptionsQuickRemoveTag => "-Tag",
        Msg::HelpTitle => "HELP",
        Msg::HelpSwitchTabs => "Switch tabs",
        Msg::HelpNavigateLists => "Navigate lists",
        Msg::HelpRefreshQuitHelp => "Refresh/Quit/Help",
    }
}

fn en_action_label(action_id: &str) -> Option<&'static str> {
    Some(match action_id {
        "nav.subscriptions" => "Open subscriptions",
        "nav.proxies" => "Open proxies",
        "nav.connections" => "Open connections",
        "nav.traffic" => "Open traffic",
        "nav.network" => "Open network",
        "nav.logs" => "Open logs",
        "nav.settings" => "Open settings",
        "nav.help" => "Open help",
        "settings.ui.language" => "Cycle UI language",
        "settings.ui.theme" => "Cycle UI theme",
        "settings.ui.default_page" => "Cycle default page",
        "settings.ui.refresh_interval" => "Cycle refresh interval",
        "settings.ui.mouse" => "Toggle mouse capture",
        "settings.ui.confirm" => "Toggle confirmations",
        "sub.add" => "Add subscription",
        "sub.import" => "Import profiles",
        "sub.use" => "Use subscription",
        "sub.update" => "Update subscription",
        "sub.edit" => "Edit subscription",
        "sub.log" => "Open subscription log",
        "sub.remove" => "Remove subscription",
        "proxy.mode.cycle" => "Cycle proxy mode",
        "proxy.sort.toggle" => "Toggle proxy sort",
        "proxy.node.open" => "Open node list",
        "proxy.node.switch" => "Switch node",
        "proxy.node.close" => "Close node list",
        "proxy.delay.selected" => "Test selected delay",
        "proxy.delay.all" => "Test all delays",
        "conn.close.selected" => "Close selected connection",
        "conn.close.all" => "Close all connections",
        "traffic.sample" => "Sample traffic",
        "traffic.collector.start" => "Start collector",
        "traffic.collector.stop" => "Stop collector",
        "traffic.collector.restart" => "Restart collector",
        "traffic.prune" => "Prune traffic",
        "traffic.reset" => "Reset traffic",
        "traffic.export" => "Export traffic",
        "settings.traffic.prune_retention" => "Prune by retention",
        "settings.traffic.default_range" => "Cycle default traffic range",
        "settings.traffic.default_chart" => "Cycle default traffic chart",
        "settings.traffic.default_dimension" => "Cycle default traffic dimension",
        "network.status" => "Status",
        "network.doctor" => "Doctor",
        "network.config_doctor" => "Config doctor",
        "network.start" => "Start",
        "network.stop" => "Stop",
        "network.restart" => "Restart",
        "network.tun.on" => "Enable TUN",
        "network.tun.off" => "Disable TUN",
        "network.env" => "Shell env exports",
        "network.proxy.on" => "Shell proxy on",
        "network.proxy.off" => "Shell proxy off",
        "network.desktop.status" => "Desktop status",
        "network.desktop.on" => "Desktop proxy on",
        "network.desktop.off" => "Desktop proxy off",
        "settings.doctor" => "Doctor",
        "settings.config_doctor" => "Config doctor",
        "settings.config.view" => "View config",
        "settings.config.raw" => "Raw config",
        "settings.proxy.test" => "Test connectivity",
        "settings.version" => "Show version",
        "settings.config.merge" => "Merge config",
        "settings.config.autofix" => "Autofix config",
        "settings.config.set_ports" => "Set ports",
        "settings.config.set_api" => "Set API",
        "settings.config.set_dns" => "Set DNS mode",
        "settings.config.set_lan" => "Set LAN",
        "settings.secret.status" => "Secret status",
        "settings.secret.reveal" => "Reveal secret",
        "settings.secret.set" => "Set secret",
        "settings.geodata.update" => "Update geodata",
        "settings.geodata.version" => "Update geodata version",
        "settings.api.upgrade" => "Upgrade clashctl",
        "settings.kernel.upgrade" => "Upgrade kernel",
        "logs.pause" => "Pause logs",
        "logs.filter" => "Cycle log filter",
        "logs.clear" => "Clear local logs",
        _ => return None,
    })
}

fn zh_action_label(action_id: &str) -> Option<&'static str> {
    Some(match action_id {
        "nav.subscriptions" => "打开订阅",
        "nav.proxies" => "打开代理",
        "nav.connections" => "打开连接",
        "nav.traffic" => "打开流量",
        "nav.network" => "打开网络",
        "nav.logs" => "打开日志",
        "nav.settings" => "打开设置",
        "nav.help" => "打开帮助",
        "settings.ui.language" => "切换界面语言",
        "settings.ui.theme" => "切换界面主题",
        "settings.ui.default_page" => "切换默认页面",
        "settings.ui.refresh_interval" => "切换刷新间隔",
        "settings.ui.mouse" => "切换鼠标捕获",
        "settings.ui.confirm" => "切换危险确认",
        "sub.add" => "添加订阅",
        "sub.import" => "导入配置",
        "sub.use" => "使用订阅",
        "sub.update" => "更新订阅",
        "sub.edit" => "编辑订阅",
        "sub.log" => "打开订阅日志",
        "sub.remove" => "删除订阅",
        "proxy.mode.cycle" => "切换代理模式",
        "proxy.sort.toggle" => "切换代理排序",
        "proxy.node.open" => "打开节点列表",
        "proxy.node.switch" => "切换节点",
        "proxy.node.close" => "关闭节点列表",
        "proxy.delay.selected" => "测试当前延迟",
        "proxy.delay.all" => "测试全部延迟",
        "conn.close.selected" => "关闭选中连接",
        "conn.close.all" => "关闭全部连接",
        "traffic.sample" => "采样流量",
        "traffic.collector.start" => "启动采集器",
        "traffic.collector.stop" => "停止采集器",
        "traffic.collector.restart" => "重启采集器",
        "traffic.prune" => "清理流量",
        "traffic.reset" => "重置流量",
        "traffic.export" => "导出流量",
        "settings.traffic.prune_retention" => "按保留期清理",
        "settings.traffic.default_range" => "切换默认流量范围",
        "settings.traffic.default_chart" => "切换默认流量图表",
        "settings.traffic.default_dimension" => "切换默认流量维度",
        "network.status" => "状态",
        "network.doctor" => "诊断",
        "network.config_doctor" => "配置诊断",
        "network.start" => "启动",
        "network.stop" => "停止",
        "network.restart" => "重启",
        "network.tun.on" => "开启 TUN",
        "network.tun.off" => "关闭 TUN",
        "network.env" => "打印 Shell 环境变量",
        "network.proxy.on" => "开启 Shell 代理",
        "network.proxy.off" => "关闭 Shell 代理",
        "network.desktop.status" => "桌面代理状态",
        "network.desktop.on" => "开启桌面代理",
        "network.desktop.off" => "关闭桌面代理",
        "settings.doctor" => "诊断",
        "settings.config_doctor" => "配置诊断",
        "settings.config.view" => "查看配置",
        "settings.config.raw" => "原始配置",
        "settings.proxy.test" => "连通性测试",
        "settings.version" => "查看版本",
        "settings.config.merge" => "合并配置",
        "settings.config.autofix" => "自动修复配置",
        "settings.config.set_ports" => "设置端口",
        "settings.config.set_api" => "设置 API",
        "settings.config.set_dns" => "设置 DNS 模式",
        "settings.config.set_lan" => "设置 LAN",
        "settings.secret.status" => "密钥状态",
        "settings.secret.reveal" => "显示密钥",
        "settings.secret.set" => "设置密钥",
        "settings.geodata.update" => "更新地理数据",
        "settings.geodata.version" => "指定版本更新地理数据",
        "settings.api.upgrade" => "升级 clashctl",
        "settings.kernel.upgrade" => "升级内核",
        "logs.pause" => "暂停日志",
        "logs.filter" => "切换日志级别",
        "logs.clear" => "清空本地日志",
        _ => return None,
    })
}

fn en_action_button(action_id: &str) -> Option<&'static str> {
    Some(match action_id {
        "nav.subscriptions" | "nav.proxies" | "nav.connections" | "nav.traffic" | "nav.network"
        | "nav.logs" | "nav.settings" | "nav.help" => "sidebar/tab",
        "settings.ui.language" => "Language",
        "settings.ui.theme" => "Theme",
        "settings.ui.default_page" => "Default page",
        "settings.ui.refresh_interval" => "Refresh",
        "settings.ui.mouse" => "Mouse",
        "settings.ui.confirm" => "Confirm",
        "sub.add" => "Add",
        "sub.import" => "Import",
        "sub.use" => "Use",
        "sub.update" => "Update",
        "sub.edit" => "Edit",
        "sub.log" => "Log",
        "sub.remove" => "Remove",
        "proxy.mode.cycle" => "Mode",
        "proxy.sort.toggle" => "Sort",
        "proxy.node.open" => "Nodes",
        "proxy.node.switch" => "Switch",
        "proxy.node.close" => "Close",
        "proxy.delay.selected" => "Test",
        "proxy.delay.all" => "Test all",
        "conn.close.selected" => "Close",
        "conn.close.all" => "Close all",
        "traffic.sample" => "Sample",
        "traffic.collector.start" => "Collect",
        "traffic.collector.stop" => "Stop",
        "traffic.collector.restart" => "Restart",
        "traffic.prune" => "Prune",
        "traffic.reset" => "Reset",
        "traffic.export" => "Export CSV",
        "settings.traffic.prune_retention" => "Prune retention",
        "settings.traffic.default_range" => "Default range",
        "settings.traffic.default_chart" => "Default chart",
        "settings.traffic.default_dimension" => "Default by",
        "network.status" => "status",
        "network.doctor" => "doctor",
        "network.config_doctor" => "config doctor",
        "network.start" => "start",
        "network.stop" => "stop",
        "network.restart" => "restart",
        "network.tun.on" => "tun on",
        "network.tun.off" => "tun off",
        "network.env" => "env",
        "network.proxy.on" => "proxy on",
        "network.proxy.off" => "proxy off",
        "network.desktop.status" => "desktop status",
        "network.desktop.on" => "desktop on",
        "network.desktop.off" => "desktop off",
        "settings.doctor" => "Doctor",
        "settings.config_doctor" => "Config doctor",
        "settings.config.view" => "View config",
        "settings.config.raw" => "Raw config",
        "settings.proxy.test" => "Test",
        "settings.version" => "Version",
        "settings.config.merge" => "Merge",
        "settings.config.autofix" => "Autofix",
        "settings.config.set_ports" => "Set ports",
        "settings.config.set_api" => "Set API",
        "settings.config.set_dns" => "DNS mode",
        "settings.config.set_lan" => "LAN",
        "settings.secret.status" => "Secret status",
        "settings.secret.reveal" => "Reveal secret",
        "settings.secret.set" => "Set secret",
        "settings.geodata.update" => "Geodata",
        "settings.geodata.version" => "Geodata version",
        "settings.api.upgrade" => "API upgrade",
        "settings.kernel.upgrade" => "Kernel upgrade",
        "logs.pause" => "Pause",
        "logs.filter" => "Level selector",
        "logs.clear" => "Clear",
        _ => return None,
    })
}

fn zh_action_button(action_id: &str) -> Option<&'static str> {
    Some(match action_id {
        "nav.subscriptions" | "nav.proxies" | "nav.connections" | "nav.traffic" | "nav.network"
        | "nav.logs" | "nav.settings" | "nav.help" => "侧栏/页签",
        "settings.ui.language" => "语言",
        "settings.ui.theme" => "主题",
        "settings.ui.default_page" => "默认页",
        "settings.ui.refresh_interval" => "刷新",
        "settings.ui.mouse" => "鼠标",
        "settings.ui.confirm" => "确认",
        "sub.add" => "添加",
        "sub.import" => "导入",
        "sub.use" => "使用",
        "sub.update" => "更新",
        "sub.edit" => "编辑",
        "sub.log" => "日志",
        "sub.remove" => "删除",
        "proxy.mode.cycle" => "模式",
        "proxy.sort.toggle" => "排序",
        "proxy.node.open" => "节点",
        "proxy.node.switch" => "切换",
        "proxy.node.close" => "关闭",
        "proxy.delay.selected" => "测速",
        "proxy.delay.all" => "全测",
        "conn.close.selected" => "关闭",
        "conn.close.all" => "全关",
        "traffic.sample" => "采样",
        "traffic.collector.start" => "采集",
        "traffic.collector.stop" => "停止",
        "traffic.collector.restart" => "重启",
        "traffic.prune" => "清理",
        "traffic.reset" => "重置",
        "traffic.export" => "导出 CSV",
        "settings.traffic.prune_retention" => "按期清理",
        "settings.traffic.default_range" => "默认范围",
        "settings.traffic.default_chart" => "默认图表",
        "settings.traffic.default_dimension" => "默认维度",
        "network.status" => "状态",
        "network.doctor" => "诊断",
        "network.config_doctor" => "配置诊断",
        "network.start" => "启动",
        "network.stop" => "停止",
        "network.restart" => "重启",
        "network.tun.on" => "开启 TUN",
        "network.tun.off" => "关闭 TUN",
        "network.env" => "环境变量",
        "network.proxy.on" => "开启代理",
        "network.proxy.off" => "关闭代理",
        "network.desktop.status" => "桌面状态",
        "network.desktop.on" => "开启桌面",
        "network.desktop.off" => "关闭桌面",
        "settings.doctor" => "诊断",
        "settings.config_doctor" => "配置诊断",
        "settings.config.view" => "查看配置",
        "settings.config.raw" => "原始配置",
        "settings.proxy.test" => "测试",
        "settings.version" => "版本",
        "settings.config.merge" => "合并",
        "settings.config.autofix" => "修复",
        "settings.config.set_ports" => "设置端口",
        "settings.config.set_api" => "设置 API",
        "settings.config.set_dns" => "DNS 模式",
        "settings.config.set_lan" => "LAN",
        "settings.secret.status" => "密钥状态",
        "settings.secret.reveal" => "显示密钥",
        "settings.secret.set" => "设置密钥",
        "settings.geodata.update" => "地理数据",
        "settings.geodata.version" => "指定版本",
        "settings.api.upgrade" => "升级 API",
        "settings.kernel.upgrade" => "升级内核",
        "logs.pause" => "暂停",
        "logs.filter" => "级别",
        "logs.clear" => "清空",
        _ => return None,
    })
}

fn en_action_desc(action_id: &str) -> Option<&'static str> {
    Some(match action_id {
        "nav.subscriptions" => "Open profile and subscription management.",
        "nav.proxies" => "Open proxy groups and node selection.",
        "nav.connections" => "Open active connection inspection.",
        "nav.traffic" => "Open persistent traffic charts and ranking.",
        "nav.network" => "Open kernel, TUN, shell proxy, and desktop proxy controls.",
        "nav.logs" => "Open kernel, subscription, and local task logs.",
        "nav.settings" => "Open UI settings, config tools, diagnostics, and updates.",
        "nav.help" => "Open action list generated from the registry.",
        "settings.ui.language" => "Cycle Auto, zh-CN, and en-US UI language and save it.",
        "settings.ui.theme" => "Cycle the terminal color palette and save it.",
        "settings.ui.default_page" => "Choose which page opens when the TUI starts.",
        "settings.ui.refresh_interval" => "Choose how often the TUI refreshes data and save it.",
        "settings.ui.mouse" => "Toggle terminal mouse capture preference for the next TUI launch.",
        "settings.ui.confirm" => "Toggle confirmation prompts for dangerous actions.",
        "sub.add" => "Open the parameterized add form.",
        "sub.import" => "Open the local YAML import form.",
        "sub.use" => "Activate the selected profile through clashctl.",
        "sub.update" => "Update the selected profile.",
        "sub.edit" => "Edit name, source, interval, proxy, user-agent, convert, and tags.",
        "sub.log" => "Show subscription operation logs.",
        "sub.remove" => "Remove the selected profile with rollback handled by clashctl.",
        "proxy.mode.cycle" => "Cycle Rule, Global, and Direct mode through the Mihomo API.",
        "proxy.sort.toggle" => "Toggle proxy group sorting between name and delay.",
        "proxy.node.open" => "Open or close the selected proxy group's node list.",
        "proxy.node.switch" => "Switch the selected group to the selected node.",
        "proxy.node.close" => "Close the proxy node list.",
        "proxy.delay.selected" => "Run delay test for the selected group.",
        "proxy.delay.all" => "Run delay test for every visible proxy group.",
        "conn.close.selected" => "Close one selected active connection.",
        "conn.close.all" => "Close every active connection.",
        "traffic.sample" => "Take one persistent traffic sample.",
        "traffic.collector.start" => "Start the background traffic collector.",
        "traffic.collector.stop" => "Stop the background traffic collector.",
        "traffic.collector.restart" => "Restart the background traffic collector.",
        "traffic.prune" => "Prune samples outside the retention window.",
        "traffic.reset" => "Delete all stored traffic data.",
        "traffic.export" => "Export the selected traffic view as CSV.",
        "settings.traffic.prune_retention" => "Open a form for traffic prune retention windows.",
        "settings.traffic.default_range" => {
            "Choose the default Traffic page time range and save it."
        }
        "settings.traffic.default_chart" => {
            "Choose the default Traffic page chart type and save it."
        }
        "settings.traffic.default_dimension" => {
            "Choose the default Traffic page aggregation dimension and save it."
        }
        "network.status" => "Refresh kernel and runtime status.",
        "network.doctor" => "Run install, permission, API, TUN, and route guard diagnostics.",
        "network.config_doctor" => "Run configuration diagnostics before risky network changes.",
        "network.start" => "Start the kernel using clashctl lifecycle rules.",
        "network.stop" => "Stop the kernel.",
        "network.restart" => "Restart the kernel.",
        "network.tun.on" => "Enable TUN through clashctl route guard and rollback logic.",
        "network.tun.off" => "Disable TUN through clashctl rollback-aware config writes.",
        "network.env" => "Print eval-ready proxy environment exports for the current shell.",
        "network.proxy.on" => "Print shell commands for enabling proxy variables.",
        "network.proxy.off" => "Print shell commands for clearing proxy variables.",
        "network.desktop.status" => "Inspect desktop proxy state.",
        "network.desktop.on" => "Enable desktop proxy settings.",
        "network.desktop.off" => "Disable desktop proxy settings.",
        "settings.doctor" => "Run the full environment diagnostic.",
        "settings.config_doctor" => "Run configuration diagnostics.",
        "settings.config.view" => "View merged runtime configuration with default redaction.",
        "settings.config.raw" => "View raw config with default redaction.",
        "settings.proxy.test" => "Run the default proxy connectivity test.",
        "settings.version" => "Show clashctl and kernel version information.",
        "settings.config.merge" => "Regenerate runtime config.",
        "settings.config.autofix" => "Regenerate runtime config with safe autofixes.",
        "settings.config.set_ports" => "Open the proxy port form.",
        "settings.config.set_api" => "Open the controller and secret form.",
        "settings.config.set_dns" => "Open the DNS mode form.",
        "settings.config.set_lan" => "Open the LAN exposure form.",
        "settings.secret.status" => "Show whether the API secret is configured.",
        "settings.secret.reveal" => "Reveal API secret after confirmation.",
        "settings.secret.set" => "Open the API secret form.",
        "settings.geodata.update" => "Download staged geodata and replace atomically.",
        "settings.geodata.version" => {
            "Open a form for updating geodata from a specific release tag or latest."
        }
        "settings.api.upgrade" => "Upgrade clashctl release assets.",
        "settings.kernel.upgrade" => "Upgrade Mihomo kernel with checksum and rollback checks.",
        "logs.pause" => "Pause or resume log updates.",
        "logs.filter" => "Cycle the minimum visible log level.",
        "logs.clear" => "Clear the TUI local log buffer.",
        _ => return None,
    })
}

fn zh_action_desc(action_id: &str) -> Option<&'static str> {
    Some(match action_id {
        "nav.subscriptions" => "打开配置文件和订阅管理。",
        "nav.proxies" => "打开代理组和节点选择。",
        "nav.connections" => "打开活动连接查看。",
        "nav.traffic" => "打开持久化流量图表和排行。",
        "nav.network" => "打开内核、TUN、Shell 代理和桌面代理控制。",
        "nav.logs" => "打开内核、订阅和本地任务日志。",
        "nav.settings" => "打开界面设置、配置工具、诊断和更新。",
        "nav.help" => "打开由动作注册表生成的动作列表。",
        "settings.ui.language" => "在自动、简体中文和英文界面之间切换并保存。",
        "settings.ui.theme" => "切换并保存终端配色方案。",
        "settings.ui.default_page" => "设置 TUI 启动时默认打开的页面。",
        "settings.ui.refresh_interval" => "设置并保存 TUI 数据刷新间隔。",
        "settings.ui.mouse" => "切换下次启动时的终端鼠标捕获偏好。",
        "settings.ui.confirm" => "切换高风险操作的确认弹窗。",
        "sub.add" => "打开参数化添加表单。",
        "sub.import" => "打开本地 YAML 导入表单。",
        "sub.use" => "通过 clashctl 激活选中的配置。",
        "sub.update" => "更新选中的配置。",
        "sub.edit" => "编辑名称、来源、频率、代理、UA、转换和标签。",
        "sub.log" => "查看订阅操作日志。",
        "sub.remove" => "删除选中的配置，失败时由 clashctl 回滚。",
        "proxy.mode.cycle" => "通过 Mihomo API 在 Rule、Global、Direct 间切换。",
        "proxy.sort.toggle" => "在名称和延迟排序之间切换代理组排序。",
        "proxy.node.open" => "打开或关闭选中代理组的节点列表。",
        "proxy.node.switch" => "把选中代理组切换到选中节点。",
        "proxy.node.close" => "关闭代理节点列表。",
        "proxy.delay.selected" => "测试选中代理组延迟。",
        "proxy.delay.all" => "测试全部可见代理组延迟。",
        "conn.close.selected" => "关闭一个选中的活动连接。",
        "conn.close.all" => "关闭全部活动连接。",
        "traffic.sample" => "执行一次持久化流量采样。",
        "traffic.collector.start" => "启动后台流量采集器。",
        "traffic.collector.stop" => "停止后台流量采集器。",
        "traffic.collector.restart" => "重启后台流量采集器。",
        "traffic.prune" => "清理超出保留窗口的采样。",
        "traffic.reset" => "删除全部已保存流量数据。",
        "traffic.export" => "把当前流量视图导出为 CSV。",
        "settings.traffic.prune_retention" => "打开流量保留期清理表单。",
        "settings.traffic.default_range" => "设置并保存流量页默认时间范围。",
        "settings.traffic.default_chart" => "设置并保存流量页默认图表类型。",
        "settings.traffic.default_dimension" => "设置并保存流量页默认聚合维度。",
        "network.status" => "刷新内核和运行状态。",
        "network.doctor" => "检查安装、权限、API、TUN 和 route guard。",
        "network.config_doctor" => "在高风险网络操作前检查配置。",
        "network.start" => "按 clashctl 生命周期规则启动内核。",
        "network.stop" => "停止内核。",
        "network.restart" => "重启内核。",
        "network.tun.on" => "通过 route guard 和回滚逻辑开启 TUN。",
        "network.tun.off" => "通过可回滚配置写入关闭 TUN。",
        "network.env" => "打印可用于当前 Shell eval 的代理环境变量。",
        "network.proxy.on" => "打印开启 Shell 代理变量的命令。",
        "network.proxy.off" => "打印清理 Shell 代理变量的命令。",
        "network.desktop.status" => "检查桌面代理状态。",
        "network.desktop.on" => "开启桌面代理设置。",
        "network.desktop.off" => "关闭桌面代理设置。",
        "settings.doctor" => "运行完整环境诊断。",
        "settings.config_doctor" => "运行配置诊断。",
        "settings.config.view" => "查看已合并运行配置，默认脱敏。",
        "settings.config.raw" => "查看原始配置，默认脱敏。",
        "settings.proxy.test" => "运行默认代理连通性测试。",
        "settings.version" => "显示 clashctl 和内核版本。",
        "settings.config.merge" => "重新生成运行配置。",
        "settings.config.autofix" => "带安全修复重新生成运行配置。",
        "settings.config.set_ports" => "打开代理端口表单。",
        "settings.config.set_api" => "打开控制器和密钥表单。",
        "settings.config.set_dns" => "打开 DNS 模式表单。",
        "settings.config.set_lan" => "打开 LAN 暴露表单。",
        "settings.secret.status" => "显示 API 密钥是否已配置。",
        "settings.secret.reveal" => "确认后显示 API 密钥。",
        "settings.secret.set" => "打开 API 密钥设置表单。",
        "settings.geodata.update" => "下载 staging 地理数据并原子替换。",
        "settings.geodata.version" => "打开指定 geodata 版本或 latest 的更新表单。",
        "settings.api.upgrade" => "升级 clashctl release 资源。",
        "settings.kernel.upgrade" => "带校验和回滚检查升级 Mihomo 内核。",
        "logs.pause" => "暂停或继续日志更新。",
        "logs.filter" => "切换最低可见日志级别。",
        "logs.clear" => "清空 TUI 本地日志缓冲。",
        _ => return None,
    })
}

fn zh(msg: Msg) -> &'static str {
    match msg {
        Msg::PageSubscriptions => "订阅",
        Msg::PageProxies => "代理",
        Msg::PageConnections => "连接",
        Msg::PageTraffic => "流量",
        Msg::PageNetwork => "网络",
        Msg::PageLogs => "日志",
        Msg::PageSettings => "设置",
        Msg::PageHelp => "帮助",
        Msg::SettingsGeneral => "通用",
        Msg::SettingsLanguage => "语言",
        Msg::SettingsTheme => "主题",
        Msg::SettingsDefaultPage => "默认页",
        Msg::SettingsRefreshInterval => "刷新",
        Msg::SettingsMouse => "鼠标",
        Msg::SettingsConfirmDanger => "危险操作确认",
        Msg::SettingsCoreApi => "内核 / API",
        Msg::SettingsTraffic => "流量",
        Msg::SettingsDiagnosticsUpdates => "诊断 / 更新",
        Msg::SettingsApi => "API",
        Msg::SettingsKernel => "内核",
        Msg::SettingsNotConnected => "未连接",
        Msg::SettingsDefaultChart => "默认图表",
        Msg::SettingsResultsPlaceholder => "运行操作后的结果会显示在这里。",
        Msg::SettingsLastResult => "上次结果",
        Msg::SettingsDiagnostics => "诊断",
        Msg::SettingsConfig => "配置",
        Msg::SettingsForms => "表单",
        Msg::SettingsSecurity => "安全",
        Msg::SettingsUpdates => "更新",
        Msg::CommonEnabled => "已启用",
        Msg::CommonDisabled => "已禁用",
        Msg::CommonOn => "开",
        Msg::CommonOff => "关",
        Msg::CommonConfirm => "确认",
        Msg::CommonCancel => "取消",
        Msg::CommonContinue => "继续",
        Msg::CommonSave => "保存",
        Msg::CommonRequired => "必填",
        Msg::CommonOptional => "可选",
        Msg::CommonDefault => "默认",
        Msg::CommonEmpty => "空",
        Msg::CommonGlobal => "全局",
        Msg::CommonSafe => "安全",
        Msg::CommonValue => "值",
        Msg::RiskConfirm => "需确认",
        Msg::RiskSensitive => "敏感",
        Msg::RiskDangerous => "危险",
        Msg::ConfirmHint => "Enter/y:确认  Esc/n:取消",
        Msg::ConfirmSystemStateWarning => "此操作可能改变系统状态。",
        Msg::SettingsPromptControlsHint => "Tab/上下:字段  Enter:继续  Esc:取消  Backspace:删除",
        Msg::StatusSearch => "搜索",
        Msg::StatusQuit => "退出",
        Msg::StatusSwitch => "切换",
        Msg::StatusRefresh => "刷新",
        Msg::StatusMode => "模式",
        Msg::StatusSort => "排序",
        Msg::SortName => "名称",
        Msg::SortDelay => "延迟",
        Msg::CommandPaletteTitle => "命令面板",
        Msg::CommandPaletteHint => "Ctrl-P 切换  Enter 运行  Esc 关闭  Up/Down 选择",
        Msg::CommandPaletteSearchHint => "搜索 id、标签、快捷键、页面、命令",
        Msg::CommandPaletteNoMatch => "没有匹配动作",
        Msg::HelpKey => "按键",
        Msg::HelpPage => "页面",
        Msg::HelpAction => "动作",
        Msg::HelpRisk => "风险",
        Msg::HelpDescription => "说明",
        Msg::HelpMoreActions => "注册表中还有更多动作；放大窗口可查看更多行。",
        Msg::HelpGenerated => "帮助由 TUI action registry 生成",
        Msg::HelpActionsUnit => "个动作",
        Msg::HelpCliBackedEnumActions => "个 CLI-backed enum 动作",
        Msg::NetworkCurrentProxy => "当前代理",
        Msg::NetworkActions => "操作",
        Msg::NetworkActionsHint => {
            "点击操作  Enter:启动  x:停止  n:重启  v:状态  V:诊断  c:配置  t:TUN  e:环境变量  p/P:代理  d/D/O:桌面"
        }
        Msg::NetworkSystemInfo => "系统信息",
        Msg::NetworkCommandOutput => "命令输出",
        Msg::NetworkNoProxySelected => "未选择代理",
        Msg::NetworkLastCommandOutput => "命令输出会显示在这里。",
        Msg::NetworkShellProxyPrinted => "Shell 代理命令只会打印，不会在 TUI 内 eval。",
        Msg::ProxyNoGroups => "没有代理组",
        Msg::ProxyGroup => "代理组",
        Msg::ProxyCurrent => "当前",
        Msg::ProxyDelay => "延迟",
        Msg::ProxyNodes => "节点",
        Msg::ProxyNodesUnit => "个节点",
        Msg::ProxyActions => "操作",
        Msg::ProxyActionsHint => {
            "registry 驱动  p:模式  s:排序  o:节点  Enter/d:测速  D:全测"
        }
        Msg::ProxyTableHint => "Enter:测速  Alt+Enter:下一个  o:节点  d:当前  D:全测  /:搜索",
        Msg::ProxyUnknownGroup => "未知",
        Msg::ProxyNode => "节点",
        Msg::ProxyNodePickerHint => "点击行、滚轮、切换/测速/关闭",
        Msg::ProxyNoNodes => "选中代理组没有节点",
        Msg::ConnectionsActive => "活动",
        Msg::ConnectionsHost => "主机",
        Msg::ConnectionsType => "类型",
        Msg::ConnectionsChain => "链路",
        Msg::ConnectionsNoActive => "没有活动连接",
        Msg::ConnectionsActions => "操作",
        Msg::ConnectionsActionsHint => "registry 驱动  c:关闭选中  C:关闭全部",
        Msg::TrafficRange => "范围",
        Msg::TrafficChart => "图表",
        Msg::TrafficBy => "维度",
        Msg::TrafficFilter => "过滤",
        Msg::TrafficAll => "全部",
        Msg::TrafficDown => "下载",
        Msg::TrafficUp => "上传",
        Msg::TrafficPeak => "峰值",
        Msg::TrafficHistory => "历史",
        Msg::TrafficAllTraffic => "全部流量",
        Msg::TrafficHistoryHint => "[/]:范围  m:图表  y:维度  点击:桶  滚轮:平移  e:导出",
        Msg::TrafficNoHistory => "还没有流量历史。运行 `clashctl traffic sample` 或 `clashctl traffic collect --daemon`。",
        Msg::TrafficDownload => "下载",
        Msg::TrafficUpload => "上传",
        Msg::TrafficMax => "最大",
        Msg::TrafficBreakdown => "排行",
        Msg::TrafficNoBreakdown => "没有流量排行",
        Msg::TrafficTotal => "总计",
        Msg::TrafficSelected => "已选",
        Msg::TrafficDetail => "详情",
        Msg::TrafficDataSource => "流量数据来自 `clashctl traffic history/top --json`。",
        Msg::TrafficBucket => "时间桶",
        Msg::TrafficRate => "速率",
        Msg::TrafficStore => "存储",
        Msg::TrafficSamples => "采样",
        Msg::TrafficCollector => "采集器",
        Msg::TrafficNever => "从未",
        Msg::TrafficRunning => "运行中",
        Msg::TrafficStale => "已失效",
        Msg::TrafficStopped => "已停止",
        Msg::TrafficTracked => "追踪",
        Msg::TrafficLast => "最近",
        Msg::TrafficStatusError => "状态错误",
        Msg::LogsActions => "操作",
        Msg::LogsActionsHint => "registry 驱动  p:暂停  f:过滤  c:清空",
        Msg::LogsEmpty => "还没有日志。",
        Msg::SubscriptionsId => "ID",
        Msg::SubscriptionsName => "名称",
        Msg::SubscriptionsInterval => "频率",
        Msg::SubscriptionsNext => "下次",
        Msg::SubscriptionsStatus => "状态",
        Msg::SubscriptionsUrl => "URL",
        Msg::SubscriptionsNoProfiles => "没有订阅。按 a 添加，或按 I 导入目录。",
        Msg::SubscriptionsOutputPlaceholder => "订阅命令输出会显示在这里。",
        Msg::SubscriptionsStatusError => "错误",
        Msg::SubscriptionsStatusActive => "已启用",
        Msg::SubscriptionsStatusReady => "就绪",
        Msg::SubscriptionsAddTitle => "添加订阅",
        Msg::SubscriptionsEditTitle => "编辑订阅",
        Msg::SubscriptionsFormControlsHint => "Tab/下:下个字段  Shift+Tab/上:上个字段  Enter:保存  Esc:取消",
        Msg::SubscriptionsQuickPromptControlsHint => "Enter:保存  Esc:取消  Backspace:删除",
        Msg::SubscriptionsQuickUrl => "URL",
        Msg::SubscriptionsQuickInterval => "频率",
        Msg::SubscriptionsQuickUserAgent => "UA",
        Msg::SubscriptionsQuickUpdateProxy => "代理",
        Msg::SubscriptionsQuickConvert => "转换",
        Msg::SubscriptionsQuickAddTag => "+标签",
        Msg::SubscriptionsQuickRemoveTag => "-标签",
        Msg::HelpTitle => "帮助",
        Msg::HelpSwitchTabs => "切换页面",
        Msg::HelpNavigateLists => "移动选择",
        Msg::HelpRefreshQuitHelp => "刷新/退出/帮助",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        tr, tr_settings_prompt_field_hint, tr_settings_prompt_field_label,
        tr_subscription_field_hint, tr_subscription_field_label, Msg, SettingsPromptFieldMsg,
        SubscriptionFieldMsg,
    };
    use crate::settings::LanguageSetting;

    #[test]
    fn translations_fallback_to_english_for_en_us() {
        assert_eq!(tr(LanguageSetting::EnUs, Msg::PageSettings), "Settings");
        assert_eq!(
            tr(LanguageSetting::EnUs, Msg::HelpSwitchTabs),
            "Switch tabs"
        );
    }

    #[test]
    fn translations_use_chinese_for_zh_cn() {
        assert_eq!(tr(LanguageSetting::ZhCn, Msg::PageSettings), "设置");
        assert_eq!(tr(LanguageSetting::ZhCn, Msg::HelpSwitchTabs), "切换页面");
    }

    #[test]
    fn network_runtime_labels_are_translated() {
        assert_eq!(
            tr(LanguageSetting::EnUs, Msg::NetworkCommandOutput),
            "Command Output"
        );
        assert_eq!(
            tr(LanguageSetting::ZhCn, Msg::NetworkCommandOutput),
            "命令输出"
        );
        assert_eq!(
            tr(LanguageSetting::ZhCn, Msg::NetworkNoProxySelected),
            "未选择代理"
        );
        assert!(tr(LanguageSetting::ZhCn, Msg::NetworkActionsHint).contains("环境变量"));
    }

    #[test]
    fn connection_runtime_labels_are_translated() {
        assert_eq!(tr(LanguageSetting::EnUs, Msg::ConnectionsHost), "Host");
        assert_eq!(tr(LanguageSetting::ZhCn, Msg::ConnectionsHost), "主机");
        assert_eq!(
            tr(LanguageSetting::ZhCn, Msg::ConnectionsNoActive),
            "没有活动连接"
        );
        assert!(tr(LanguageSetting::ZhCn, Msg::ConnectionsActionsHint).contains("关闭全部"));
    }

    #[test]
    fn proxy_runtime_labels_are_translated() {
        assert_eq!(tr(LanguageSetting::EnUs, Msg::ProxyGroup), "Group");
        assert_eq!(tr(LanguageSetting::ZhCn, Msg::ProxyGroup), "代理组");
        assert_eq!(tr(LanguageSetting::ZhCn, Msg::ProxyNoGroups), "没有代理组");
        assert!(tr(LanguageSetting::ZhCn, Msg::ProxyActionsHint).contains("模式"));
        assert!(tr(LanguageSetting::ZhCn, Msg::ProxyNodePickerHint).contains("滚轮"));
    }

    #[test]
    fn traffic_settings_and_logs_runtime_labels_are_translated() {
        assert_eq!(
            tr(LanguageSetting::EnUs, Msg::TrafficBreakdown),
            "Breakdown"
        );
        assert_eq!(tr(LanguageSetting::ZhCn, Msg::TrafficBreakdown), "排行");
        assert!(tr(LanguageSetting::ZhCn, Msg::TrafficNoHistory).contains("流量历史"));
        assert_eq!(
            tr(LanguageSetting::ZhCn, Msg::SettingsResultsPlaceholder),
            "运行操作后的结果会显示在这里。"
        );
        assert_eq!(tr(LanguageSetting::ZhCn, Msg::LogsEmpty), "还没有日志。");
    }

    #[test]
    fn global_popups_status_and_subscriptions_labels_are_translated() {
        assert_eq!(tr(LanguageSetting::ZhCn, Msg::CommonConfirm), "确认");
        assert_eq!(tr(LanguageSetting::ZhCn, Msg::StatusSearch), "搜索");
        assert_eq!(
            tr(LanguageSetting::ZhCn, Msg::CommandPaletteTitle),
            "命令面板"
        );
        assert_eq!(
            tr(LanguageSetting::ZhCn, Msg::SubscriptionsStatusActive),
            "已启用"
        );
        assert!(tr(LanguageSetting::ZhCn, Msg::SubscriptionsNoProfiles).contains("没有订阅"));
        assert!(tr(LanguageSetting::ZhCn, Msg::HelpMoreActions).contains("更多动作"));
        assert_eq!(tr(LanguageSetting::ZhCn, Msg::RiskDangerous), "危险");
        assert_eq!(
            tr(LanguageSetting::ZhCn, Msg::SubscriptionsQuickUpdateProxy),
            "代理"
        );
    }

    #[test]
    fn settings_prompt_fields_are_translated() {
        assert_eq!(
            tr_settings_prompt_field_label(
                LanguageSetting::EnUs,
                SettingsPromptFieldMsg::MixedPort
            ),
            "Mixed port"
        );
        assert_eq!(
            tr_settings_prompt_field_label(
                LanguageSetting::ZhCn,
                SettingsPromptFieldMsg::MixedPort
            ),
            "混合端口"
        );
        assert!(tr_settings_prompt_field_hint(
            LanguageSetting::ZhCn,
            SettingsPromptFieldMsg::AllowUnsafe
        )
        .contains("loopback"));
    }

    #[test]
    fn subscription_fields_are_translated() {
        assert_eq!(
            tr_subscription_field_label(LanguageSetting::EnUs, SubscriptionFieldMsg::Source),
            "Source URL/path"
        );
        assert_eq!(
            tr_subscription_field_label(LanguageSetting::ZhCn, SubscriptionFieldMsg::Source),
            "来源 URL/路径"
        );
        assert!(tr_subscription_field_hint(
            LanguageSetting::ZhCn,
            SubscriptionFieldMsg::UpdateProxy
        )
        .contains("direct"));
    }
}
