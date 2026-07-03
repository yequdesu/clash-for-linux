use crate::mouse::{NetworkAction, SettingsAction, TrafficAction};
use crate::ui::components::nav::Tab;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionDanger {
    Safe,
    Confirm,
    Sensitive,
    Dangerous,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionExecutor {
    Network(NetworkAction),
    Settings(SettingsAction),
    Traffic(TrafficAction),
    Api(ApiAction),
    Prompt(PromptAction),
    Internal(InternalAction),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiAction {
    CycleProxyMode,
    SwitchProxyNode,
    TestProxyDelay,
    CloseSelectedConnection,
    CloseAllConnections,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromptAction {
    SubAdd,
    SubImport,
    SubEdit,
    ConfigSetPort,
    ConfigSetApi,
    ConfigSetDnsMode,
    ConfigSetLan,
    SecretSet,
    TrafficPruneRetention,
    GeodataUpdateVersion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InternalAction {
    SwitchTab,
    CycleUiLanguage,
    CycleThemePreference,
    CycleDefaultPage,
    CycleRefreshInterval,
    ToggleMousePreference,
    ToggleDangerousConfirmations,
    SubUse,
    SubUpdate,
    SubLog,
    SubRemove,
    ToggleProxySort,
    ToggleNodePicker,
    CloseNodePicker,
    TestAllProxyDelays,
    TrafficExportCsv,
    CycleDefaultTrafficRange,
    CycleDefaultTrafficChart,
    CycleDefaultTrafficDimension,
    ToggleLogPause,
    CycleLogLevel,
    ClearLogs,
}

impl ActionExecutor {
    pub fn command(self) -> String {
        match self {
            Self::Network(action) => clashctl_command(action.args()),
            Self::Settings(action) => clashctl_command(action.args()),
            Self::Traffic(action) => clashctl_command(action.args()),
            Self::Api(action) => format!("mihomo api {}", action.command()),
            Self::Prompt(action) => format!("prompt {}", action.command()),
            Self::Internal(action) => action.command().to_string(),
        }
    }
}

impl ApiAction {
    pub fn command(self) -> &'static str {
        match self {
            Self::CycleProxyMode => "PATCH /configs mode",
            Self::SwitchProxyNode => "PUT /proxies/{group}",
            Self::TestProxyDelay => "GET /proxies/{name}/delay",
            Self::CloseSelectedConnection => "DELETE /connections/{id}",
            Self::CloseAllConnections => "DELETE /connections",
        }
    }
}

impl PromptAction {
    pub fn command(self) -> &'static str {
        match self {
            Self::SubAdd => "sub add",
            Self::SubImport => "sub import",
            Self::SubEdit => "sub edit",
            Self::ConfigSetPort => "config set-port",
            Self::ConfigSetApi => "config set-api",
            Self::ConfigSetDnsMode => "config set-dns-mode",
            Self::ConfigSetLan => "config set-lan",
            Self::SecretSet => "secret set",
            Self::TrafficPruneRetention => "traffic prune retention",
            Self::GeodataUpdateVersion => "geodata update version",
        }
    }
}

impl InternalAction {
    pub fn command(self) -> &'static str {
        match self {
            Self::SwitchTab => "switch tab",
            Self::CycleUiLanguage => "cycle ui language",
            Self::CycleThemePreference => "cycle theme preference",
            Self::CycleDefaultPage => "cycle default page",
            Self::CycleRefreshInterval => "cycle refresh interval",
            Self::ToggleMousePreference => "toggle mouse preference",
            Self::ToggleDangerousConfirmations => "toggle dangerous confirmations",
            Self::SubUse => "clashctl sub use <id>",
            Self::SubUpdate => "clashctl sub update <id>",
            Self::SubLog => "clashctl sub log",
            Self::SubRemove => "clashctl sub remove <id>",
            Self::ToggleProxySort => "toggle proxy sort",
            Self::ToggleNodePicker => "toggle node picker",
            Self::CloseNodePicker => "close node picker",
            Self::TestAllProxyDelays => "test all proxy delays",
            Self::TrafficExportCsv => "clashctl traffic export --format csv",
            Self::CycleDefaultTrafficRange => "cycle default traffic range",
            Self::CycleDefaultTrafficChart => "cycle default traffic chart",
            Self::CycleDefaultTrafficDimension => "cycle default traffic dimension",
            Self::ToggleLogPause => "toggle log pause",
            Self::CycleLogLevel => "cycle log level",
            Self::ClearLogs => "clear local logs",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionSpec {
    pub id: &'static str,
    pub page: Tab,
    pub label: &'static str,
    pub shortcut: &'static str,
    pub mouse: &'static str,
    pub description: &'static str,
    pub danger: ActionDanger,
    pub executor: ActionExecutor,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CliCoverage {
    pub command: &'static str,
    pub action_ids: &'static [&'static str],
    pub cli_only_reason: Option<&'static str>,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CliVariantCoverage {
    pub variant: &'static str,
    pub action_id: Option<&'static str>,
    pub executor_command: Option<&'static str>,
    pub cli_only_reason: Option<&'static str>,
}

pub fn all_actions() -> &'static [ActionSpec] {
    ACTIONS
}

#[cfg(test)]
pub fn cli_coverage() -> &'static [CliCoverage] {
    CLI_COVERAGE
}

#[cfg(test)]
pub fn cli_variant_coverage() -> &'static [CliVariantCoverage] {
    CLI_VARIANT_COVERAGE
}

pub fn network_action_specs() -> impl Iterator<Item = &'static ActionSpec> {
    ACTIONS
        .iter()
        .filter(|spec| matches!(spec.executor, ActionExecutor::Network(_)))
}

pub fn action_by_id(id: &str) -> Option<&'static ActionSpec> {
    ACTIONS.iter().find(|spec| spec.id == id)
}

pub fn traffic_action_specs() -> impl Iterator<Item = &'static ActionSpec> {
    ACTIONS
        .iter()
        .filter(|spec| spec.page == Tab::Traffic && spec.id.starts_with("traffic."))
}

pub fn proxy_action_specs() -> impl Iterator<Item = &'static ActionSpec> {
    ACTIONS.iter().filter(|spec| {
        spec.page == Tab::Proxies
            && spec.id.starts_with("proxy.")
            && !matches!(spec.id, "proxy.node.switch" | "proxy.node.close")
    })
}

pub fn connection_action_specs() -> impl Iterator<Item = &'static ActionSpec> {
    ACTIONS
        .iter()
        .filter(|spec| spec.page == Tab::Connections && spec.id.starts_with("conn."))
}

pub fn log_action_specs() -> impl Iterator<Item = &'static ActionSpec> {
    ACTIONS
        .iter()
        .filter(|spec| spec.page == Tab::Logs && spec.id.starts_with("logs."))
}

pub fn node_picker_action_specs() -> impl Iterator<Item = &'static ActionSpec> {
    [
        "proxy.delay.selected",
        "proxy.delay.all",
        "proxy.node.close",
    ]
    .into_iter()
    .filter_map(action_by_id)
}

pub fn cli_backed_enum_count() -> usize {
    NetworkAction::all().len() + SettingsAction::all().len() + TrafficAction::all().len()
}

fn clashctl_command(args: Vec<String>) -> String {
    if args.is_empty() {
        "clashctl".into()
    } else {
        format!("clashctl {}", args.join(" "))
    }
}

const ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        id: "nav.subscriptions",
        page: Tab::Subscriptions,
        label: "Open subscriptions",
        shortcut: "1",
        mouse: "sidebar/tab",
        description: "Open profile and subscription management.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Internal(InternalAction::SwitchTab),
    },
    ActionSpec {
        id: "nav.proxies",
        page: Tab::Proxies,
        label: "Open proxies",
        shortcut: "2",
        mouse: "sidebar/tab",
        description: "Open proxy groups and node selection.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Internal(InternalAction::SwitchTab),
    },
    ActionSpec {
        id: "nav.connections",
        page: Tab::Connections,
        label: "Open connections",
        shortcut: "3",
        mouse: "sidebar/tab",
        description: "Open active connection inspection.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Internal(InternalAction::SwitchTab),
    },
    ActionSpec {
        id: "nav.traffic",
        page: Tab::Traffic,
        label: "Open traffic",
        shortcut: "4",
        mouse: "sidebar/tab",
        description: "Open persistent traffic charts and ranking.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Internal(InternalAction::SwitchTab),
    },
    ActionSpec {
        id: "nav.network",
        page: Tab::Network,
        label: "Open network",
        shortcut: "5",
        mouse: "sidebar/tab",
        description: "Open kernel, TUN, shell proxy, and desktop proxy controls.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Internal(InternalAction::SwitchTab),
    },
    ActionSpec {
        id: "nav.logs",
        page: Tab::Logs,
        label: "Open logs",
        shortcut: "6",
        mouse: "sidebar/tab",
        description: "Open kernel, subscription, and local task logs.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Internal(InternalAction::SwitchTab),
    },
    ActionSpec {
        id: "nav.settings",
        page: Tab::Settings,
        label: "Open settings",
        shortcut: "7/Ctrl-S",
        mouse: "sidebar/tab",
        description: "Open UI settings, config tools, diagnostics, and updates.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Internal(InternalAction::SwitchTab),
    },
    ActionSpec {
        id: "nav.help",
        page: Tab::Help,
        label: "Open help",
        shortcut: "?/8",
        mouse: "sidebar/tab",
        description: "Open action list generated from the registry.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Internal(InternalAction::SwitchTab),
    },
    ActionSpec {
        id: "settings.ui.language",
        page: Tab::Settings,
        label: "Cycle UI language",
        shortcut: "Ctrl-L",
        mouse: "Language",
        description: "Cycle Auto, zh-CN, and en-US UI language and save it.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Internal(InternalAction::CycleUiLanguage),
    },
    ActionSpec {
        id: "settings.ui.theme",
        page: Tab::Settings,
        label: "Cycle UI theme",
        shortcut: "E",
        mouse: "Theme",
        description: "Cycle the terminal color palette and save it.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Internal(InternalAction::CycleThemePreference),
    },
    ActionSpec {
        id: "settings.ui.default_page",
        page: Tab::Settings,
        label: "Cycle default page",
        shortcut: "B",
        mouse: "Default page",
        description: "Choose which page opens when the TUI starts.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Internal(InternalAction::CycleDefaultPage),
    },
    ActionSpec {
        id: "settings.ui.refresh_interval",
        page: Tab::Settings,
        label: "Cycle refresh interval",
        shortcut: "F",
        mouse: "Refresh",
        description: "Choose how often the TUI refreshes data and save it.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Internal(InternalAction::CycleRefreshInterval),
    },
    ActionSpec {
        id: "settings.ui.mouse",
        page: Tab::Settings,
        label: "Toggle mouse capture",
        shortcut: "M",
        mouse: "Mouse",
        description: "Toggle terminal mouse capture preference for the next TUI launch.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Internal(InternalAction::ToggleMousePreference),
    },
    ActionSpec {
        id: "settings.ui.confirm",
        page: Tab::Settings,
        label: "Toggle confirmations",
        shortcut: "C",
        mouse: "Confirm",
        description: "Toggle confirmation prompts for dangerous actions.",
        danger: ActionDanger::Confirm,
        executor: ActionExecutor::Internal(InternalAction::ToggleDangerousConfirmations),
    },
    ActionSpec {
        id: "sub.add",
        page: Tab::Subscriptions,
        label: "Add subscription",
        shortcut: "a",
        mouse: "Add button",
        description: "Open the parameterized add form.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Prompt(PromptAction::SubAdd),
    },
    ActionSpec {
        id: "sub.import",
        page: Tab::Subscriptions,
        label: "Import profiles",
        shortcut: "I",
        mouse: "Import button",
        description: "Open the local YAML import form.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Prompt(PromptAction::SubImport),
    },
    ActionSpec {
        id: "sub.use",
        page: Tab::Subscriptions,
        label: "Use subscription",
        shortcut: "Enter",
        mouse: "Use button",
        description: "Activate the selected profile through clashctl.",
        danger: ActionDanger::Confirm,
        executor: ActionExecutor::Internal(InternalAction::SubUse),
    },
    ActionSpec {
        id: "sub.update",
        page: Tab::Subscriptions,
        label: "Update subscription",
        shortcut: "u",
        mouse: "Update button",
        description: "Update the selected profile.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Internal(InternalAction::SubUpdate),
    },
    ActionSpec {
        id: "sub.edit",
        page: Tab::Subscriptions,
        label: "Edit subscription",
        shortcut: "e",
        mouse: "Edit button",
        description: "Edit name, source, interval, proxy, user-agent, convert, and tags.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Prompt(PromptAction::SubEdit),
    },
    ActionSpec {
        id: "sub.log",
        page: Tab::Subscriptions,
        label: "Open subscription log",
        shortcut: "L",
        mouse: "Log",
        description: "Show subscription operation logs.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Internal(InternalAction::SubLog),
    },
    ActionSpec {
        id: "sub.remove",
        page: Tab::Subscriptions,
        label: "Remove subscription",
        shortcut: "X",
        mouse: "Remove button",
        description: "Remove the selected profile with rollback handled by clashctl.",
        danger: ActionDanger::Dangerous,
        executor: ActionExecutor::Internal(InternalAction::SubRemove),
    },
    ActionSpec {
        id: "proxy.mode.cycle",
        page: Tab::Proxies,
        label: "Cycle proxy mode",
        shortcut: "p",
        mouse: "Mode selector",
        description: "Cycle Rule, Global, and Direct mode through the Mihomo API.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Api(ApiAction::CycleProxyMode),
    },
    ActionSpec {
        id: "proxy.sort.toggle",
        page: Tab::Proxies,
        label: "Toggle proxy sort",
        shortcut: "s/S",
        mouse: "Sort button",
        description: "Toggle proxy group sorting between name and delay.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Internal(InternalAction::ToggleProxySort),
    },
    ActionSpec {
        id: "proxy.node.open",
        page: Tab::Proxies,
        label: "Open node list",
        shortcut: "o",
        mouse: "Nodes button",
        description: "Open or close the selected proxy group's node list.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Internal(InternalAction::ToggleNodePicker),
    },
    ActionSpec {
        id: "proxy.node.switch",
        page: Tab::Proxies,
        label: "Confirm node",
        shortcut: "Enter",
        mouse: "Selected node row",
        description: "Confirm the selected node in the node list.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Api(ApiAction::SwitchProxyNode),
    },
    ActionSpec {
        id: "proxy.node.close",
        page: Tab::Proxies,
        label: "Close node list",
        shortcut: "Esc/o",
        mouse: "Close",
        description: "Close the proxy node list.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Internal(InternalAction::CloseNodePicker),
    },
    ActionSpec {
        id: "proxy.delay.selected",
        page: Tab::Proxies,
        label: "Test selected delay",
        shortcut: "Enter/d",
        mouse: "Test selected",
        description: "Run delay test for the selected group.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Api(ApiAction::TestProxyDelay),
    },
    ActionSpec {
        id: "proxy.delay.all",
        page: Tab::Proxies,
        label: "Test all delays",
        shortcut: "D",
        mouse: "Test all",
        description: "Run delay test for every visible proxy group.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Internal(InternalAction::TestAllProxyDelays),
    },
    ActionSpec {
        id: "conn.close.selected",
        page: Tab::Connections,
        label: "Close selected connection",
        shortcut: "c",
        mouse: "Close selected",
        description: "Close one selected active connection.",
        danger: ActionDanger::Confirm,
        executor: ActionExecutor::Api(ApiAction::CloseSelectedConnection),
    },
    ActionSpec {
        id: "conn.close.all",
        page: Tab::Connections,
        label: "Close all connections",
        shortcut: "C",
        mouse: "Close all",
        description: "Close every active connection.",
        danger: ActionDanger::Dangerous,
        executor: ActionExecutor::Api(ApiAction::CloseAllConnections),
    },
    ActionSpec {
        id: "traffic.sample",
        page: Tab::Traffic,
        label: "Sample traffic",
        shortcut: "s",
        mouse: "Sample",
        description: "Take one persistent traffic sample.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Traffic(TrafficAction::SampleOnce),
    },
    ActionSpec {
        id: "traffic.collector.start",
        page: Tab::Traffic,
        label: "Start collector",
        shortcut: "b",
        mouse: "Collect",
        description: "Start the background traffic collector.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Traffic(TrafficAction::CollectorStart),
    },
    ActionSpec {
        id: "traffic.collector.stop",
        page: Tab::Traffic,
        label: "Stop collector",
        shortcut: "x",
        mouse: "Stop",
        description: "Stop the background traffic collector.",
        danger: ActionDanger::Confirm,
        executor: ActionExecutor::Traffic(TrafficAction::CollectorStop),
    },
    ActionSpec {
        id: "traffic.collector.restart",
        page: Tab::Traffic,
        label: "Restart collector",
        shortcut: "n",
        mouse: "Restart",
        description: "Restart the background traffic collector.",
        danger: ActionDanger::Confirm,
        executor: ActionExecutor::Traffic(TrafficAction::CollectorRestart),
    },
    ActionSpec {
        id: "traffic.prune",
        page: Tab::Traffic,
        label: "Prune traffic",
        shortcut: "p",
        mouse: "Prune",
        description: "Prune samples outside the retention window.",
        danger: ActionDanger::Confirm,
        executor: ActionExecutor::Traffic(TrafficAction::PruneDefault),
    },
    ActionSpec {
        id: "traffic.reset",
        page: Tab::Settings,
        label: "Reset traffic",
        shortcut: "D",
        mouse: "Reset",
        description: "Delete all stored traffic data.",
        danger: ActionDanger::Dangerous,
        executor: ActionExecutor::Traffic(TrafficAction::Reset),
    },
    ActionSpec {
        id: "traffic.export",
        page: Tab::Traffic,
        label: "Export traffic",
        shortcut: "e",
        mouse: "Export CSV",
        description: "Export the selected traffic view as CSV.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Internal(InternalAction::TrafficExportCsv),
    },
    ActionSpec {
        id: "settings.traffic.prune_retention",
        page: Tab::Settings,
        label: "Prune by retention",
        shortcut: "T",
        mouse: "Prune retention",
        description: "Open a form for traffic prune retention windows.",
        danger: ActionDanger::Confirm,
        executor: ActionExecutor::Prompt(PromptAction::TrafficPruneRetention),
    },
    ActionSpec {
        id: "settings.traffic.default_range",
        page: Tab::Settings,
        label: "Cycle default traffic range",
        shortcut: "R",
        mouse: "Default range",
        description: "Choose the default Traffic page time range and save it.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Internal(InternalAction::CycleDefaultTrafficRange),
    },
    ActionSpec {
        id: "settings.traffic.default_chart",
        page: Tab::Settings,
        label: "Cycle default traffic chart",
        shortcut: "H",
        mouse: "Default chart",
        description: "Choose the default Traffic page chart type and save it.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Internal(InternalAction::CycleDefaultTrafficChart),
    },
    ActionSpec {
        id: "settings.traffic.default_dimension",
        page: Tab::Settings,
        label: "Cycle default traffic dimension",
        shortcut: "y",
        mouse: "Default by",
        description: "Choose the default Traffic page aggregation dimension and save it.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Internal(InternalAction::CycleDefaultTrafficDimension),
    },
    ActionSpec {
        id: "network.status",
        page: Tab::Network,
        label: "Status",
        shortcut: "v",
        mouse: "status",
        description: "Refresh kernel and runtime status.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Network(NetworkAction::Status),
    },
    ActionSpec {
        id: "network.doctor",
        page: Tab::Network,
        label: "Doctor",
        shortcut: "V",
        mouse: "doctor",
        description: "Run install, permission, API, TUN, and route guard diagnostics.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Network(NetworkAction::Doctor),
    },
    ActionSpec {
        id: "network.config_doctor",
        page: Tab::Network,
        label: "Config doctor",
        shortcut: "c",
        mouse: "config doctor",
        description: "Run configuration diagnostics before risky network changes.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Network(NetworkAction::ConfigDoctor),
    },
    ActionSpec {
        id: "network.start",
        page: Tab::Network,
        label: "Start",
        shortcut: "Enter",
        mouse: "start",
        description: "Start the kernel using clashctl lifecycle rules.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Network(NetworkAction::Start),
    },
    ActionSpec {
        id: "network.stop",
        page: Tab::Network,
        label: "Stop",
        shortcut: "x",
        mouse: "stop",
        description: "Stop the kernel.",
        danger: ActionDanger::Confirm,
        executor: ActionExecutor::Network(NetworkAction::Stop),
    },
    ActionSpec {
        id: "network.restart",
        page: Tab::Network,
        label: "Restart",
        shortcut: "n",
        mouse: "restart",
        description: "Restart the kernel.",
        danger: ActionDanger::Confirm,
        executor: ActionExecutor::Network(NetworkAction::Restart),
    },
    ActionSpec {
        id: "network.tun.on",
        page: Tab::Network,
        label: "Enable TUN",
        shortcut: "t",
        mouse: "tun on",
        description: "Enable TUN through clashctl route guard and rollback logic.",
        danger: ActionDanger::Dangerous,
        executor: ActionExecutor::Network(NetworkAction::TunOn),
    },
    ActionSpec {
        id: "network.tun.off",
        page: Tab::Network,
        label: "Disable TUN",
        shortcut: "t",
        mouse: "tun off",
        description: "Disable TUN through clashctl rollback-aware config writes.",
        danger: ActionDanger::Confirm,
        executor: ActionExecutor::Network(NetworkAction::TunOff),
    },
    ActionSpec {
        id: "network.proxy.on",
        page: Tab::Network,
        label: "Shell proxy on",
        shortcut: "p",
        mouse: "proxy on",
        description: "Print shell commands for enabling proxy variables.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Network(NetworkAction::ProxyOn),
    },
    ActionSpec {
        id: "network.env",
        page: Tab::Network,
        label: "Shell env exports",
        shortcut: "e",
        mouse: "env",
        description: "Print eval-ready proxy environment exports for the current shell.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Network(NetworkAction::Env),
    },
    ActionSpec {
        id: "network.proxy.off",
        page: Tab::Network,
        label: "Shell proxy off",
        shortcut: "P",
        mouse: "proxy off",
        description: "Print shell commands for clearing proxy variables.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Network(NetworkAction::ProxyOff),
    },
    ActionSpec {
        id: "network.desktop.status",
        page: Tab::Network,
        label: "Desktop status",
        shortcut: "d",
        mouse: "desktop status",
        description: "Inspect desktop proxy state.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Network(NetworkAction::DesktopStatus),
    },
    ActionSpec {
        id: "network.desktop.on",
        page: Tab::Network,
        label: "Desktop proxy on",
        shortcut: "D",
        mouse: "desktop on",
        description: "Enable desktop proxy settings.",
        danger: ActionDanger::Confirm,
        executor: ActionExecutor::Network(NetworkAction::DesktopOn),
    },
    ActionSpec {
        id: "network.desktop.off",
        page: Tab::Network,
        label: "Desktop proxy off",
        shortcut: "O",
        mouse: "desktop off",
        description: "Disable desktop proxy settings.",
        danger: ActionDanger::Confirm,
        executor: ActionExecutor::Network(NetworkAction::DesktopOff),
    },
    ActionSpec {
        id: "settings.doctor",
        page: Tab::Settings,
        label: "Doctor",
        shortcut: "d",
        mouse: "Doctor",
        description: "Run the full environment diagnostic.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Settings(SettingsAction::Doctor),
    },
    ActionSpec {
        id: "settings.config_doctor",
        page: Tab::Settings,
        label: "Config doctor",
        shortcut: "c",
        mouse: "Config doctor",
        description: "Run configuration diagnostics.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Settings(SettingsAction::ConfigDoctor),
    },
    ActionSpec {
        id: "settings.config.view",
        page: Tab::Settings,
        label: "View config",
        shortcut: "V",
        mouse: "View config",
        description: "View merged runtime configuration with default redaction.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Settings(SettingsAction::ConfigView),
    },
    ActionSpec {
        id: "settings.config.raw",
        page: Tab::Settings,
        label: "Raw config",
        shortcut: "W",
        mouse: "Raw config",
        description: "View raw config with default redaction.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Settings(SettingsAction::ConfigRaw),
    },
    ActionSpec {
        id: "settings.proxy.test",
        page: Tab::Settings,
        label: "Test connectivity",
        shortcut: "t",
        mouse: "Test",
        description: "Run the default proxy connectivity test.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Settings(SettingsAction::ProxyTest),
    },
    ActionSpec {
        id: "settings.version",
        page: Tab::Settings,
        label: "Show version",
        shortcut: "v",
        mouse: "Version",
        description: "Show clashctl and kernel version information.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Settings(SettingsAction::Version),
    },
    ActionSpec {
        id: "settings.config.merge",
        page: Tab::Settings,
        label: "Merge config",
        shortcut: "m",
        mouse: "Merge",
        description: "Regenerate runtime config.",
        danger: ActionDanger::Confirm,
        executor: ActionExecutor::Settings(SettingsAction::ConfigMerge),
    },
    ActionSpec {
        id: "settings.config.autofix",
        page: Tab::Settings,
        label: "Autofix config",
        shortcut: "a",
        mouse: "Autofix",
        description: "Regenerate runtime config with safe autofixes.",
        danger: ActionDanger::Confirm,
        executor: ActionExecutor::Settings(SettingsAction::ConfigMergeAutofix),
    },
    ActionSpec {
        id: "settings.config.set_ports",
        page: Tab::Settings,
        label: "Set ports",
        shortcut: "P",
        mouse: "Set ports",
        description: "Open the proxy port form.",
        danger: ActionDanger::Confirm,
        executor: ActionExecutor::Prompt(PromptAction::ConfigSetPort),
    },
    ActionSpec {
        id: "settings.config.set_api",
        page: Tab::Settings,
        label: "Set API",
        shortcut: "A",
        mouse: "Set API",
        description: "Open the controller and secret form.",
        danger: ActionDanger::Confirm,
        executor: ActionExecutor::Prompt(PromptAction::ConfigSetApi),
    },
    ActionSpec {
        id: "settings.config.set_dns",
        page: Tab::Settings,
        label: "Set DNS mode",
        shortcut: "Y",
        mouse: "DNS mode",
        description: "Open the DNS mode form.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Prompt(PromptAction::ConfigSetDnsMode),
    },
    ActionSpec {
        id: "settings.config.set_lan",
        page: Tab::Settings,
        label: "Set LAN",
        shortcut: "L",
        mouse: "LAN",
        description: "Open the LAN exposure form.",
        danger: ActionDanger::Confirm,
        executor: ActionExecutor::Prompt(PromptAction::ConfigSetLan),
    },
    ActionSpec {
        id: "settings.secret.status",
        page: Tab::Settings,
        label: "Secret status",
        shortcut: "s",
        mouse: "Secret status",
        description: "Show whether the API secret is configured.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Settings(SettingsAction::SecretStatus),
    },
    ActionSpec {
        id: "settings.secret.reveal",
        page: Tab::Settings,
        label: "Reveal secret",
        shortcut: "S",
        mouse: "Reveal secret",
        description: "Reveal API secret after confirmation.",
        danger: ActionDanger::Sensitive,
        executor: ActionExecutor::Settings(SettingsAction::SecretReveal),
    },
    ActionSpec {
        id: "settings.secret.set",
        page: Tab::Settings,
        label: "Set secret",
        shortcut: "N",
        mouse: "Set secret",
        description: "Open the API secret form.",
        danger: ActionDanger::Sensitive,
        executor: ActionExecutor::Prompt(PromptAction::SecretSet),
    },
    ActionSpec {
        id: "settings.geodata.update",
        page: Tab::Settings,
        label: "Update geodata",
        shortcut: "z",
        mouse: "Geodata",
        description: "Download staged geodata and replace atomically.",
        danger: ActionDanger::Confirm,
        executor: ActionExecutor::Settings(SettingsAction::GeodataUpdate),
    },
    ActionSpec {
        id: "settings.geodata.version",
        page: Tab::Settings,
        label: "Update geodata version",
        shortcut: "Z",
        mouse: "Geodata version",
        description: "Open a form for updating geodata from a specific release tag or latest.",
        danger: ActionDanger::Confirm,
        executor: ActionExecutor::Prompt(PromptAction::GeodataUpdateVersion),
    },
    ActionSpec {
        id: "settings.api.upgrade",
        page: Tab::Settings,
        label: "Upgrade clashctl",
        shortcut: "u",
        mouse: "API upgrade",
        description: "Upgrade clashctl release assets.",
        danger: ActionDanger::Confirm,
        executor: ActionExecutor::Settings(SettingsAction::ApiUpgrade),
    },
    ActionSpec {
        id: "settings.kernel.upgrade",
        page: Tab::Settings,
        label: "Upgrade kernel",
        shortcut: "K",
        mouse: "Kernel upgrade",
        description: "Upgrade Mihomo kernel with checksum and rollback checks.",
        danger: ActionDanger::Dangerous,
        executor: ActionExecutor::Settings(SettingsAction::KernelUpgrade),
    },
    ActionSpec {
        id: "logs.pause",
        page: Tab::Logs,
        label: "Pause logs",
        shortcut: "p",
        mouse: "Pause",
        description: "Pause or resume log updates.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Internal(InternalAction::ToggleLogPause),
    },
    ActionSpec {
        id: "logs.filter",
        page: Tab::Logs,
        label: "Cycle log filter",
        shortcut: "f",
        mouse: "Level selector",
        description: "Cycle the minimum visible log level.",
        danger: ActionDanger::Safe,
        executor: ActionExecutor::Internal(InternalAction::CycleLogLevel),
    },
    ActionSpec {
        id: "logs.clear",
        page: Tab::Logs,
        label: "Clear local logs",
        shortcut: "c",
        mouse: "Clear",
        description: "Clear the TUI local log buffer.",
        danger: ActionDanger::Confirm,
        executor: ActionExecutor::Internal(InternalAction::ClearLogs),
    },
];

#[cfg(test)]
const CLI_COVERAGE: &[CliCoverage] = &[
    CliCoverage {
        command: "start",
        action_ids: &["network.start"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "stop",
        action_ids: &["network.stop"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "restart",
        action_ids: &["network.restart"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "status",
        action_ids: &["network.status"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "log",
        action_ids: &["nav.logs"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "proxy",
        action_ids: &[
            "network.proxy.on",
            "network.proxy.off",
            "network.desktop.status",
            "network.desktop.on",
            "network.desktop.off",
        ],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "tun",
        action_ids: &["network.tun.on", "network.tun.off"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "secret",
        action_ids: &[
            "settings.secret.status",
            "settings.secret.reveal",
            "settings.secret.set",
        ],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "upgrade",
        action_ids: &["settings.api.upgrade"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "upgrade-kernel",
        action_ids: &["settings.kernel.upgrade"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "config",
        action_ids: &["nav.settings"],
        cli_only_reason: Some("namespace command that prints help"),
    },
    CliCoverage {
        command: "config edit",
        action_ids: &[],
        cli_only_reason: Some("external editor command; TUI does not suspend into $EDITOR yet"),
    },
    CliCoverage {
        command: "config view",
        action_ids: &["settings.config.view"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "config raw",
        action_ids: &["settings.config.raw"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "config merge",
        action_ids: &["settings.config.merge", "settings.config.autofix"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "config set-port",
        action_ids: &["settings.config.set_ports"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "config set-api",
        action_ids: &["settings.config.set_api"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "config set-dns-mode",
        action_ids: &["settings.config.set_dns"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "config set-lan",
        action_ids: &["settings.config.set_lan"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "config doctor",
        action_ids: &["settings.config_doctor", "network.config_doctor"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "sub",
        action_ids: &["nav.subscriptions"],
        cli_only_reason: Some("namespace command that prints help"),
    },
    CliCoverage {
        command: "sub add",
        action_ids: &["sub.add"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "sub list",
        action_ids: &["nav.subscriptions"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "sub remove",
        action_ids: &["sub.remove"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "sub use",
        action_ids: &["sub.use"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "sub update",
        action_ids: &["sub.update"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "sub import",
        action_ids: &["sub.import"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "sub log",
        action_ids: &["sub.log"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "sub rename",
        action_ids: &["sub.edit"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "sub set-url",
        action_ids: &["sub.edit"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "sub set-interval",
        action_ids: &["sub.edit"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "sub set-update-proxy",
        action_ids: &["sub.edit"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "sub set-user-agent",
        action_ids: &["sub.edit"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "sub set-convert",
        action_ids: &["sub.edit"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "sub tag",
        action_ids: &["sub.edit"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "node",
        action_ids: &["nav.proxies"],
        cli_only_reason: Some("namespace command that prints help"),
    },
    CliCoverage {
        command: "node list",
        action_ids: &["nav.proxies"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "node switch",
        action_ids: &["proxy.node.switch"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "node delay",
        action_ids: &["proxy.delay.selected"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "env",
        action_ids: &["network.env"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "test",
        action_ids: &["settings.proxy.test"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "tui",
        action_ids: &[],
        cli_only_reason: Some("self-launch command; not exposed inside the TUI"),
    },
    CliCoverage {
        command: "doctor",
        action_ids: &["settings.doctor", "network.doctor"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "geodata",
        action_ids: &["settings.geodata.update"],
        cli_only_reason: Some("namespace command that prints help"),
    },
    CliCoverage {
        command: "geodata update",
        action_ids: &["settings.geodata.update"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "version",
        action_ids: &["settings.version"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "traffic",
        action_ids: &["nav.traffic"],
        cli_only_reason: Some("namespace command that prints help"),
    },
    CliCoverage {
        command: "traffic status",
        action_ids: &["nav.traffic"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "traffic sample",
        action_ids: &["traffic.sample"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "traffic collect",
        action_ids: &[],
        cli_only_reason: Some("collector worker loop; managed by traffic collector start/restart"),
    },
    CliCoverage {
        command: "traffic history",
        action_ids: &["nav.traffic"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "traffic top",
        action_ids: &["nav.traffic"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "traffic export",
        action_ids: &["traffic.export"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "traffic prune",
        action_ids: &["traffic.prune"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "traffic reset",
        action_ids: &["traffic.reset"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "traffic collector",
        action_ids: &["nav.traffic"],
        cli_only_reason: Some("namespace command that prints help"),
    },
    CliCoverage {
        command: "traffic collector status",
        action_ids: &["nav.traffic"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "traffic collector start",
        action_ids: &["traffic.collector.start"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "traffic collector stop",
        action_ids: &["traffic.collector.stop"],
        cli_only_reason: None,
    },
    CliCoverage {
        command: "traffic collector restart",
        action_ids: &["traffic.collector.restart"],
        cli_only_reason: None,
    },
];

#[cfg(test)]
const CLI_VARIANT_COVERAGE: &[CliVariantCoverage] = &[
    CliVariantCoverage {
        variant: "proxy on",
        action_id: Some("network.proxy.on"),
        executor_command: Some("clashctl proxy on"),
        cli_only_reason: None,
    },
    CliVariantCoverage {
        variant: "proxy off",
        action_id: Some("network.proxy.off"),
        executor_command: Some("clashctl proxy off"),
        cli_only_reason: None,
    },
    CliVariantCoverage {
        variant: "proxy desktop status",
        action_id: Some("network.desktop.status"),
        executor_command: Some("clashctl proxy desktop status"),
        cli_only_reason: None,
    },
    CliVariantCoverage {
        variant: "proxy desktop on",
        action_id: Some("network.desktop.on"),
        executor_command: Some("clashctl proxy desktop on"),
        cli_only_reason: None,
    },
    CliVariantCoverage {
        variant: "proxy desktop off",
        action_id: Some("network.desktop.off"),
        executor_command: Some("clashctl proxy desktop off"),
        cli_only_reason: None,
    },
    CliVariantCoverage {
        variant: "env",
        action_id: Some("network.env"),
        executor_command: Some("clashctl env"),
        cli_only_reason: None,
    },
    CliVariantCoverage {
        variant: "tun on",
        action_id: Some("network.tun.on"),
        executor_command: Some("clashctl tun on"),
        cli_only_reason: None,
    },
    CliVariantCoverage {
        variant: "tun off",
        action_id: Some("network.tun.off"),
        executor_command: Some("clashctl tun off"),
        cli_only_reason: None,
    },
    CliVariantCoverage {
        variant: "start --allow-route-risk",
        action_id: None,
        executor_command: None,
        cli_only_reason: Some("risk override remains CLI-only; TUI uses guarded default start"),
    },
    CliVariantCoverage {
        variant: "restart --allow-route-risk",
        action_id: None,
        executor_command: None,
        cli_only_reason: Some("risk override remains CLI-only; TUI uses guarded default restart"),
    },
    CliVariantCoverage {
        variant: "tun on --allow-route-risk",
        action_id: None,
        executor_command: None,
        cli_only_reason: Some(
            "risk override remains CLI-only; TUI uses guarded default TUN enable",
        ),
    },
    CliVariantCoverage {
        variant: "sub use --allow-route-risk",
        action_id: None,
        executor_command: None,
        cli_only_reason: Some(
            "risk override remains CLI-only; TUI uses guarded default subscription switch",
        ),
    },
    CliVariantCoverage {
        variant: "upgrade-kernel --allow-route-risk",
        action_id: None,
        executor_command: None,
        cli_only_reason: Some(
            "risk override remains CLI-only; TUI uses guarded default kernel upgrade",
        ),
    },
    CliVariantCoverage {
        variant: "upgrade-kernel --allow-unsigned",
        action_id: None,
        executor_command: None,
        cli_only_reason: Some("unsigned kernel upgrade is an explicit CLI risk override"),
    },
    CliVariantCoverage {
        variant: "config merge --autofix",
        action_id: Some("settings.config.autofix"),
        executor_command: Some("clashctl config merge --autofix"),
        cli_only_reason: None,
    },
    CliVariantCoverage {
        variant: "config set-port --http --socks",
        action_id: Some("settings.config.set_ports"),
        executor_command: Some("prompt config set-port"),
        cli_only_reason: None,
    },
    CliVariantCoverage {
        variant: "config set-api --secret",
        action_id: Some("settings.config.set_api"),
        executor_command: Some("prompt config set-api"),
        cli_only_reason: None,
    },
    CliVariantCoverage {
        variant: "config set-api --allow-unsafe",
        action_id: Some("settings.config.set_api"),
        executor_command: Some("prompt config set-api"),
        cli_only_reason: None,
    },
    CliVariantCoverage {
        variant: "config set-dns-mode fake-ip",
        action_id: Some("settings.config.set_dns"),
        executor_command: Some("prompt config set-dns-mode"),
        cli_only_reason: None,
    },
    CliVariantCoverage {
        variant: "config set-dns-mode redir-host",
        action_id: Some("settings.config.set_dns"),
        executor_command: Some("prompt config set-dns-mode"),
        cli_only_reason: None,
    },
    CliVariantCoverage {
        variant: "config set-dns-mode off",
        action_id: Some("settings.config.set_dns"),
        executor_command: Some("prompt config set-dns-mode"),
        cli_only_reason: None,
    },
    CliVariantCoverage {
        variant: "config set-lan on",
        action_id: Some("settings.config.set_lan"),
        executor_command: Some("prompt config set-lan"),
        cli_only_reason: None,
    },
    CliVariantCoverage {
        variant: "config set-lan off",
        action_id: Some("settings.config.set_lan"),
        executor_command: Some("prompt config set-lan"),
        cli_only_reason: None,
    },
    CliVariantCoverage {
        variant: "secret show",
        action_id: Some("settings.secret.reveal"),
        executor_command: Some("clashctl secret show"),
        cli_only_reason: None,
    },
    CliVariantCoverage {
        variant: "secret <new-secret>",
        action_id: Some("settings.secret.set"),
        executor_command: Some("prompt secret set"),
        cli_only_reason: None,
    },
    CliVariantCoverage {
        variant: "sub add --name --interval --update-proxy --user-agent --convert --tag",
        action_id: Some("sub.add"),
        executor_command: Some("prompt sub add"),
        cli_only_reason: None,
    },
    CliVariantCoverage {
        variant: "sub rename <id> <name>",
        action_id: Some("sub.edit"),
        executor_command: Some("prompt sub edit"),
        cli_only_reason: None,
    },
    CliVariantCoverage {
        variant: "sub set-url <id> <url|path>",
        action_id: Some("sub.edit"),
        executor_command: Some("prompt sub edit"),
        cli_only_reason: None,
    },
    CliVariantCoverage {
        variant: "sub set-interval <id> <duration|off>",
        action_id: Some("sub.edit"),
        executor_command: Some("prompt sub edit"),
        cli_only_reason: None,
    },
    CliVariantCoverage {
        variant: "sub set-update-proxy <id> <direct|system|core|auto>",
        action_id: Some("sub.edit"),
        executor_command: Some("prompt sub edit"),
        cli_only_reason: None,
    },
    CliVariantCoverage {
        variant: "sub set-user-agent <id> <user-agent>",
        action_id: Some("sub.edit"),
        executor_command: Some("prompt sub edit"),
        cli_only_reason: None,
    },
    CliVariantCoverage {
        variant: "sub set-convert <id> <auto|off|force>",
        action_id: Some("sub.edit"),
        executor_command: Some("prompt sub edit"),
        cli_only_reason: None,
    },
    CliVariantCoverage {
        variant: "sub tag add <id> <tag>",
        action_id: Some("sub.edit"),
        executor_command: Some("prompt sub edit"),
        cli_only_reason: None,
    },
    CliVariantCoverage {
        variant: "sub tag remove <id> <tag>",
        action_id: Some("sub.edit"),
        executor_command: Some("prompt sub edit"),
        cli_only_reason: None,
    },
    CliVariantCoverage {
        variant: "sub update --scheduled --cron",
        action_id: None,
        executor_command: None,
        cli_only_reason: Some("scheduler/cron worker mode is not an interactive TUI action"),
    },
    CliVariantCoverage {
        variant: "node switch <group> <node>",
        action_id: Some("proxy.node.switch"),
        executor_command: Some("mihomo api PUT /proxies/{group}"),
        cli_only_reason: None,
    },
    CliVariantCoverage {
        variant: "node delay [group]",
        action_id: Some("proxy.delay.selected"),
        executor_command: Some("mihomo api GET /proxies/{name}/delay"),
        cli_only_reason: None,
    },
    CliVariantCoverage {
        variant: "traffic status --json",
        action_id: Some("nav.traffic"),
        executor_command: Some("switch tab"),
        cli_only_reason: None,
    },
    CliVariantCoverage {
        variant: "traffic history --range --step --by --json",
        action_id: Some("nav.traffic"),
        executor_command: Some("switch tab"),
        cli_only_reason: None,
    },
    CliVariantCoverage {
        variant: "traffic top --range --by --json",
        action_id: Some("nav.traffic"),
        executor_command: Some("switch tab"),
        cli_only_reason: None,
    },
    CliVariantCoverage {
        variant: "traffic export --format csv --by",
        action_id: Some("traffic.export"),
        executor_command: Some("clashctl traffic export --format csv"),
        cli_only_reason: None,
    },
    CliVariantCoverage {
        variant: "traffic prune --retention",
        action_id: Some("settings.traffic.prune_retention"),
        executor_command: Some("prompt traffic prune retention"),
        cli_only_reason: None,
    },
    CliVariantCoverage {
        variant: "traffic reset --yes",
        action_id: Some("traffic.reset"),
        executor_command: Some("clashctl traffic reset --yes"),
        cli_only_reason: None,
    },
    CliVariantCoverage {
        variant: "traffic collect --daemon",
        action_id: None,
        executor_command: None,
        cli_only_reason: Some(
            "collector worker loop is managed by traffic collector start/restart",
        ),
    },
    CliVariantCoverage {
        variant: "geodata update --version",
        action_id: Some("settings.geodata.version"),
        executor_command: Some("prompt geodata update version"),
        cli_only_reason: None,
    },
    CliVariantCoverage {
        variant: "config edit",
        action_id: None,
        executor_command: None,
        cli_only_reason: Some("external editor command; TUI suspend/resume is not implemented yet"),
    },
    CliVariantCoverage {
        variant: "tui",
        action_id: None,
        executor_command: None,
        cli_only_reason: Some("self-launch command; not exposed inside the TUI"),
    },
];

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs;
    use std::path::Path;

    use super::{action_by_id, all_actions, cli_coverage, cli_variant_coverage, ActionExecutor};
    use crate::i18n::{tr_action_button, tr_action_desc, tr_action_label};
    use crate::mouse::{NetworkAction, SettingsAction, TrafficAction};
    use crate::settings::LanguageSetting;

    #[test]
    fn action_ids_are_unique() {
        let mut ids = BTreeSet::new();
        for action in all_actions() {
            assert!(ids.insert(action.id), "duplicate action id {}", action.id);
        }
    }

    #[test]
    fn registry_covers_cli_backed_action_enums() {
        for action in NetworkAction::all() {
            assert!(
                all_actions()
                    .iter()
                    .any(|spec| spec.executor == ActionExecutor::Network(*action)),
                "missing network action {:?}",
                action
            );
        }
        for action in SettingsAction::all() {
            assert!(
                all_actions()
                    .iter()
                    .any(|spec| spec.executor == ActionExecutor::Settings(*action)),
                "missing settings action {:?}",
                action
            );
        }
        for action in TrafficAction::all() {
            assert!(
                all_actions()
                    .iter()
                    .any(|spec| spec.executor == ActionExecutor::Traffic(*action)),
                "missing traffic action {:?}",
                action
            );
        }
    }

    #[test]
    fn cli_coverage_matches_go_cobra_commands() {
        let source_commands = go_cli_command_paths();
        let coverage_commands = cli_coverage()
            .iter()
            .map(|coverage| coverage.command.to_string())
            .collect::<BTreeSet<_>>();

        let missing = source_commands
            .difference(&coverage_commands)
            .cloned()
            .collect::<Vec<_>>();
        let stale = coverage_commands
            .difference(&source_commands)
            .cloned()
            .collect::<Vec<_>>();

        assert!(
            missing.is_empty(),
            "Go CLI commands missing from TUI coverage matrix: {:?}",
            missing
        );
        assert!(
            stale.is_empty(),
            "TUI coverage commands not present in Go CLI tree: {:?}",
            stale
        );
    }

    #[test]
    fn cli_coverage_references_existing_actions_or_cli_only_reason() {
        let action_ids = all_actions()
            .iter()
            .map(|action| action.id)
            .collect::<BTreeSet<_>>();
        let mut commands = BTreeSet::new();

        for coverage in cli_coverage() {
            assert!(
                commands.insert(coverage.command),
                "duplicate CLI coverage entry for {}",
                coverage.command
            );
            if coverage.cli_only_reason.is_none() {
                assert!(
                    !coverage.action_ids.is_empty(),
                    "{} must have at least one TUI action or a cli_only_reason",
                    coverage.command
                );
            }
            for action_id in coverage.action_ids {
                assert!(
                    action_ids.contains(action_id),
                    "{} references unknown action id {}",
                    coverage.command,
                    action_id
                );
            }
        }
    }

    #[test]
    fn cli_variant_coverage_references_existing_actions_or_cli_only_reason() {
        let source_commands = go_cli_command_paths();
        let mut variants = BTreeSet::new();

        for coverage in cli_variant_coverage() {
            assert!(
                variants.insert(coverage.variant),
                "duplicate CLI variant coverage entry for {}",
                coverage.variant
            );

            if let Some(reason) = coverage.cli_only_reason {
                assert!(
                    coverage.action_id.is_none() && coverage.executor_command.is_none(),
                    "{} is CLI-only ({}) but still references a TUI action",
                    coverage.variant,
                    reason
                );
            } else {
                let action_id = coverage
                    .action_id
                    .unwrap_or_else(|| panic!("{} must reference a TUI action", coverage.variant));
                let action = action_by_id(action_id)
                    .unwrap_or_else(|| panic!("unknown action id {action_id}"));
                let expected = coverage.executor_command.unwrap_or_else(|| {
                    panic!(
                        "{} must declare its expected executor command",
                        coverage.variant
                    )
                });
                assert_eq!(
                    action.executor.command(),
                    expected,
                    "{} executor drifted",
                    coverage.variant
                );
            }

            assert!(
                has_source_command_for_variant(coverage.variant, &source_commands),
                "{} does not map to a Cobra command path",
                coverage.variant
            );
        }
    }

    #[test]
    fn registry_actions_have_chinese_translations() {
        for action in all_actions() {
            assert_ne!(
                tr_action_label(LanguageSetting::ZhCn, action.id, "__missing__"),
                "__missing__",
                "missing zh label for {}",
                action.id
            );
            assert_ne!(
                tr_action_button(LanguageSetting::ZhCn, action.id, "__missing__"),
                "__missing__",
                "missing zh button for {}",
                action.id
            );
            assert_ne!(
                tr_action_desc(LanguageSetting::ZhCn, action.id, "__missing__"),
                "__missing__",
                "missing zh description for {}",
                action.id
            );
        }
    }

    fn go_cli_command_paths() -> BTreeSet<String> {
        let cmd_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("repo root")
            .join("cmd")
            .join("clashctl");
        let mut uses = BTreeMap::<String, String>::new();
        let mut edges = BTreeMap::<String, Vec<String>>::new();

        for entry in fs::read_dir(&cmd_dir).expect("read cmd/clashctl") {
            let path = entry.expect("dir entry").path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("go") {
                continue;
            }
            if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with("_test.go"))
            {
                continue;
            }
            let source = fs::read_to_string(&path).expect("read go source");
            parse_cobra_uses(&source, &mut uses);
            parse_add_commands(&source, &mut edges);
        }

        let mut paths = BTreeSet::new();
        if let Some(root_children) = edges.get("rootCmd") {
            for child in root_children {
                collect_command_paths(child, "", &uses, &edges, &mut paths);
            }
        }
        paths
    }

    fn parse_cobra_uses(source: &str, uses: &mut BTreeMap<String, String>) {
        let lines = source.lines().collect::<Vec<_>>();
        for (idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if !trimmed.starts_with("var ") || !trimmed.contains("= &cobra.Command") {
                continue;
            }
            let Some(name) = trimmed
                .strip_prefix("var ")
                .and_then(|rest| rest.split_whitespace().next())
            else {
                continue;
            };
            for candidate in lines.iter().skip(idx).take(24) {
                if let Some(use_value) = extract_use_token(candidate) {
                    uses.insert(name.to_string(), use_value);
                    break;
                }
            }
        }
    }

    fn parse_add_commands(source: &str, edges: &mut BTreeMap<String, Vec<String>>) {
        let lines = source.lines().collect::<Vec<_>>();
        let mut idx = 0;
        while idx < lines.len() {
            let line = lines[idx];
            let Some(call_start) = line.find(".AddCommand(") else {
                idx += 1;
                continue;
            };
            let parent = line[..call_start].trim().to_string();
            let mut args = line[call_start + ".AddCommand(".len()..].to_string();
            while !args.contains(')') && idx + 1 < lines.len() {
                idx += 1;
                args.push(' ');
                args.push_str(lines[idx]);
            }
            let args = args.split(')').next().unwrap_or("");
            for token in identifier_tokens(args) {
                if token.ends_with("Cmd") {
                    edges.entry(parent.clone()).or_default().push(token);
                }
            }
            idx += 1;
        }
    }

    fn extract_use_token(line: &str) -> Option<String> {
        let use_pos = line.find("Use:")?;
        let after_use = &line[use_pos..];
        let first_quote = after_use.find('"')?;
        let rest = &after_use[first_quote + 1..];
        let second_quote = rest.find('"')?;
        let use_value = &rest[..second_quote];
        use_value.split_whitespace().next().map(str::to_string)
    }

    fn identifier_tokens(raw: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current = String::new();
        for ch in raw.chars() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                current.push(ch);
            } else if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
        }
        if !current.is_empty() {
            tokens.push(current);
        }
        tokens
    }

    fn collect_command_paths(
        var_name: &str,
        prefix: &str,
        uses: &BTreeMap<String, String>,
        edges: &BTreeMap<String, Vec<String>>,
        paths: &mut BTreeSet<String>,
    ) {
        let Some(use_token) = uses.get(var_name) else {
            return;
        };
        let path = if prefix.is_empty() {
            use_token.clone()
        } else {
            format!("{prefix} {use_token}")
        };
        paths.insert(path.clone());
        if let Some(children) = edges.get(var_name) {
            for child in children {
                collect_command_paths(child, &path, uses, edges, paths);
            }
        }
    }

    fn has_source_command_for_variant(variant: &str, source_commands: &BTreeSet<String>) -> bool {
        let variant_tokens = variant.split_whitespace().collect::<Vec<_>>();
        source_commands.iter().any(|command| {
            let command_tokens = command.split_whitespace().collect::<Vec<_>>();
            command_tokens.len() <= variant_tokens.len()
                && command_tokens
                    .iter()
                    .zip(variant_tokens.iter())
                    .all(|(command_token, variant_token)| command_token == variant_token)
        })
    }
}
