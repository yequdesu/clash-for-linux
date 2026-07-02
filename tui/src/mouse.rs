use ratatui::layout::Rect;

use crate::widgets::tab_bar::Tab;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkAction {
    Status,
    Doctor,
    ConfigDoctor,
    Start,
    Stop,
    Restart,
    TunOn,
    TunOff,
    Env,
    ProxyOn,
    ProxyOff,
    DesktopStatus,
    DesktopOn,
    DesktopOff,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsAction {
    Doctor,
    ConfigDoctor,
    ConfigView,
    ConfigRaw,
    ProxyTest,
    Version,
    ConfigMerge,
    ConfigMergeAutofix,
    SecretStatus,
    SecretReveal,
    GeodataUpdate,
    ApiUpgrade,
    KernelUpgrade,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrafficAction {
    SampleOnce,
    CollectorStart,
    CollectorStop,
    CollectorRestart,
    PruneDefault,
    Reset,
}

impl TrafficAction {
    pub fn all() -> &'static [Self] {
        &[
            Self::SampleOnce,
            Self::CollectorStart,
            Self::CollectorStop,
            Self::CollectorRestart,
            Self::PruneDefault,
            Self::Reset,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::SampleOnce => "traffic sample",
            Self::CollectorStart => "traffic collector start",
            Self::CollectorStop => "traffic collector stop",
            Self::CollectorRestart => "traffic collector restart",
            Self::PruneDefault => "traffic prune",
            Self::Reset => "traffic reset",
        }
    }

    pub fn args(self) -> Vec<String> {
        match self {
            Self::SampleOnce => vec!["traffic".into(), "sample".into()],
            Self::CollectorStart => vec!["traffic".into(), "collector".into(), "start".into()],
            Self::CollectorStop => vec!["traffic".into(), "collector".into(), "stop".into()],
            Self::CollectorRestart => vec!["traffic".into(), "collector".into(), "restart".into()],
            Self::PruneDefault => vec!["traffic".into(), "prune".into()],
            Self::Reset => vec!["traffic".into(), "reset".into(), "--yes".into()],
        }
    }

    pub fn requires_confirmation(self) -> bool {
        matches!(
            self,
            Self::CollectorStop | Self::CollectorRestart | Self::PruneDefault | Self::Reset
        )
    }
}

impl SettingsAction {
    pub fn all() -> &'static [Self] {
        &[
            Self::Doctor,
            Self::ConfigDoctor,
            Self::ConfigView,
            Self::ConfigRaw,
            Self::ProxyTest,
            Self::Version,
            Self::ConfigMerge,
            Self::ConfigMergeAutofix,
            Self::SecretStatus,
            Self::SecretReveal,
            Self::GeodataUpdate,
            Self::ApiUpgrade,
            Self::KernelUpgrade,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Doctor => "doctor",
            Self::ConfigDoctor => "config doctor",
            Self::ConfigView => "config view",
            Self::ConfigRaw => "config raw",
            Self::ProxyTest => "test",
            Self::Version => "version",
            Self::ConfigMerge => "config merge",
            Self::ConfigMergeAutofix => "config autofix",
            Self::SecretStatus => "secret status",
            Self::SecretReveal => "secret reveal",
            Self::GeodataUpdate => "geodata update",
            Self::ApiUpgrade => "api upgrade",
            Self::KernelUpgrade => "kernel upgrade",
        }
    }

    pub fn args(self) -> Vec<String> {
        match self {
            Self::Doctor => vec!["doctor".into()],
            Self::ConfigDoctor => vec!["config".into(), "doctor".into()],
            Self::ConfigView => vec!["config".into(), "view".into()],
            Self::ConfigRaw => vec!["config".into(), "raw".into()],
            Self::ProxyTest => vec!["test".into()],
            Self::Version => vec!["version".into()],
            Self::ConfigMerge => vec!["config".into(), "merge".into()],
            Self::ConfigMergeAutofix => vec!["config".into(), "merge".into(), "--autofix".into()],
            Self::SecretStatus => vec!["secret".into()],
            Self::SecretReveal => vec!["secret".into(), "show".into()],
            Self::GeodataUpdate => vec!["geodata".into(), "update".into()],
            Self::ApiUpgrade => vec!["upgrade".into()],
            Self::KernelUpgrade => vec!["upgrade-kernel".into()],
        }
    }

    pub fn requires_confirmation(self) -> bool {
        matches!(
            self,
            Self::ConfigMerge
                | Self::ConfigMergeAutofix
                | Self::SecretReveal
                | Self::GeodataUpdate
                | Self::ApiUpgrade
                | Self::KernelUpgrade
        )
    }

    pub fn redact_output(self) -> bool {
        !matches!(self, Self::SecretReveal)
    }
}

impl NetworkAction {
    pub fn all() -> &'static [Self] {
        &[
            Self::Status,
            Self::Doctor,
            Self::ConfigDoctor,
            Self::Start,
            Self::Stop,
            Self::Restart,
            Self::TunOn,
            Self::TunOff,
            Self::Env,
            Self::ProxyOn,
            Self::ProxyOff,
            Self::DesktopStatus,
            Self::DesktopOn,
            Self::DesktopOff,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Status => "status",
            Self::Doctor => "doctor",
            Self::ConfigDoctor => "config doctor",
            Self::Start => "start",
            Self::Stop => "stop",
            Self::Restart => "restart",
            Self::TunOn => "tun on",
            Self::TunOff => "tun off",
            Self::Env => "env",
            Self::ProxyOn => "proxy on",
            Self::ProxyOff => "proxy off",
            Self::DesktopStatus => "desktop status",
            Self::DesktopOn => "desktop on",
            Self::DesktopOff => "desktop off",
        }
    }

    pub fn args(self) -> Vec<String> {
        match self {
            Self::Status => vec!["status".into()],
            Self::Doctor => vec!["doctor".into()],
            Self::ConfigDoctor => vec!["config".into(), "doctor".into()],
            Self::Start => vec!["start".into()],
            Self::Stop => vec!["stop".into()],
            Self::Restart => vec!["restart".into()],
            Self::TunOn => vec!["tun".into(), "on".into()],
            Self::TunOff => vec!["tun".into(), "off".into()],
            Self::Env => vec!["env".into()],
            Self::ProxyOn => vec!["proxy".into(), "on".into()],
            Self::ProxyOff => vec!["proxy".into(), "off".into()],
            Self::DesktopStatus => vec!["proxy".into(), "desktop".into(), "status".into()],
            Self::DesktopOn => vec!["proxy".into(), "desktop".into(), "on".into()],
            Self::DesktopOff => vec!["proxy".into(), "desktop".into(), "off".into()],
        }
    }

    pub fn requires_confirmation(self) -> bool {
        matches!(
            self,
            Self::Stop
                | Self::Restart
                | Self::TunOn
                | Self::TunOff
                | Self::DesktopOn
                | Self::DesktopOff
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HitboxAction {
    SwitchTab(Tab),
    RunNetwork(NetworkAction),
    RunSettings(SettingsAction),
    BeginSecretSet,
    BeginConfigSetPorts,
    BeginConfigSetApi,
    BeginConfigSetDns,
    BeginConfigSetLan,
    BeginTrafficPruneRetention,
    BeginGeodataUpdateVersion,
    RunTraffic(TrafficAction),
    SelectProxy(usize),
    SelectProxyNode(usize),
    SelectSubscription(usize),
    BeginSubscriptionAdd,
    BeginSubscriptionImport,
    UseSubscription,
    UpdateSubscription,
    RemoveSubscription,
    ShowSubscriptionLog,
    EditSubscriptionName,
    EditSubscriptionInterval,
    EditSubscriptionUrl,
    EditSubscriptionUserAgent,
    EditSubscriptionUpdateProxy,
    EditSubscriptionConvertMode,
    EditSubscriptionAddTag,
    EditSubscriptionRemoveTag,
    SelectSubscriptionAddField(usize),
    SelectConnection(usize),
    SelectTrafficRow(usize),
    SelectTrafficBucket(usize),
    NextTrafficRange,
    ToggleTrafficChart,
    NextTrafficDimension,
    ExportTraffic,
    SubmitSubscriptionPrompt,
    CancelSubscriptionPrompt,
    ConfirmPendingAction,
    CancelPendingAction,
    FocusSettingsPromptValue,
    SelectSettingsPromptField(usize),
    SelectSettingsSection(usize),
    SubmitSettingsPrompt,
    CancelSettingsPrompt,
    FocusSudoPromptValue,
    SubmitSudoPrompt,
    CancelSudoPrompt,
    CycleUiLanguage,
    CycleThemePreference,
    CycleDefaultPage,
    CycleRefreshInterval,
    ToggleMousePreference,
    ToggleDangerousConfirmations,
    CycleDefaultTrafficRange,
    CycleDefaultTrafficChart,
    CycleDefaultTrafficDimension,
    CycleProxyMode,
    ToggleProxySort,
    ToggleNodePicker,
    CloseNodePicker,
    SwitchSelectedProxyNode,
    TestSelectedProxyDelay,
    TestAllProxyDelays,
    CloseSelectedConnection,
    CloseAllConnections,
    ToggleLogPause,
    CycleLogFilter,
    ClearLogs,
    ScrollLogs,
    ScrollProxyNodes,
    ScrollTrafficChart,
    ScrollTrafficRows,
    ScrollHelp,
    ScrollNetworkOutput,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hitbox {
    pub area: Rect,
    pub action: HitboxAction,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HitboxRegistry {
    boxes: Vec<Hitbox>,
}

impl HitboxRegistry {
    pub fn clear(&mut self) {
        self.boxes.clear();
    }

    pub fn register(&mut self, area: Rect, action: HitboxAction) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        self.boxes.push(Hitbox { area, action });
    }

    pub fn action_at(&self, x: u16, y: u16) -> Option<HitboxAction> {
        self.boxes
            .iter()
            .rev()
            .find(|hitbox| contains(hitbox.area, x, y))
            .map(|hitbox| hitbox.action.clone())
    }
}

fn contains(area: Rect, x: u16, y: u16) -> bool {
    x >= area.x
        && x < area.x.saturating_add(area.width)
        && y >= area.y
        && y < area.y.saturating_add(area.height)
}

#[cfg(test)]
mod tests {
    use super::{HitboxAction, HitboxRegistry, NetworkAction, SettingsAction, TrafficAction};
    use crate::widgets::tab_bar::Tab;
    use ratatui::layout::Rect;

    #[test]
    fn latest_hitbox_wins_for_overlaps() {
        let mut registry = HitboxRegistry::default();
        registry.register(
            Rect::new(0, 0, 10, 2),
            HitboxAction::SwitchTab(Tab::Subscriptions),
        );
        registry.register(Rect::new(2, 0, 4, 2), HitboxAction::SwitchTab(Tab::Traffic));

        assert_eq!(
            registry.action_at(3, 1),
            Some(HitboxAction::SwitchTab(Tab::Traffic))
        );
        assert_eq!(
            registry.action_at(1, 1),
            Some(HitboxAction::SwitchTab(Tab::Subscriptions))
        );
        assert_eq!(registry.action_at(20, 1), None);
    }

    #[test]
    fn network_actions_map_to_clashctl_args() {
        assert_eq!(NetworkAction::Start.args(), vec!["start".to_string()]);
        assert_eq!(NetworkAction::Stop.args(), vec!["stop".to_string()]);
        assert_eq!(NetworkAction::Doctor.args(), vec!["doctor".to_string()]);
        assert_eq!(
            NetworkAction::ConfigDoctor.args(),
            vec!["config".to_string(), "doctor".to_string()]
        );
        assert_eq!(
            NetworkAction::TunOn.args(),
            vec!["tun".to_string(), "on".to_string()]
        );
        assert_eq!(NetworkAction::Env.args(), vec!["env".to_string()]);
        assert_eq!(
            NetworkAction::ProxyOff.args(),
            vec!["proxy".to_string(), "off".to_string()]
        );
        assert_eq!(
            NetworkAction::DesktopStatus.args(),
            vec![
                "proxy".to_string(),
                "desktop".to_string(),
                "status".to_string()
            ]
        );
    }

    #[test]
    fn settings_actions_map_to_clashctl_args() {
        assert_eq!(SettingsAction::Doctor.args(), vec!["doctor".to_string()]);
        assert_eq!(
            SettingsAction::ConfigDoctor.args(),
            vec!["config".to_string(), "doctor".to_string()]
        );
        assert_eq!(
            SettingsAction::ConfigView.args(),
            vec!["config".to_string(), "view".to_string()]
        );
        assert_eq!(
            SettingsAction::ConfigRaw.args(),
            vec!["config".to_string(), "raw".to_string()]
        );
        assert_eq!(
            SettingsAction::ConfigMergeAutofix.args(),
            vec![
                "config".to_string(),
                "merge".to_string(),
                "--autofix".to_string()
            ]
        );
        assert_eq!(
            SettingsAction::GeodataUpdate.args(),
            vec!["geodata".to_string(), "update".to_string()]
        );
        assert_eq!(
            SettingsAction::SecretReveal.args(),
            vec!["secret".to_string(), "show".to_string()]
        );
        assert_eq!(
            SettingsAction::KernelUpgrade.args(),
            vec!["upgrade-kernel".to_string()]
        );
    }

    #[test]
    fn traffic_actions_map_to_clashctl_args() {
        assert_eq!(
            TrafficAction::SampleOnce.args(),
            vec!["traffic".to_string(), "sample".to_string()]
        );
        assert_eq!(
            TrafficAction::PruneDefault.args(),
            vec!["traffic".to_string(), "prune".to_string()]
        );
        assert_eq!(
            TrafficAction::CollectorStart.args(),
            vec![
                "traffic".to_string(),
                "collector".to_string(),
                "start".to_string()
            ]
        );
        assert_eq!(
            TrafficAction::CollectorRestart.args(),
            vec![
                "traffic".to_string(),
                "collector".to_string(),
                "restart".to_string()
            ]
        );
        assert_eq!(
            TrafficAction::Reset.args(),
            vec![
                "traffic".to_string(),
                "reset".to_string(),
                "--yes".to_string()
            ]
        );
    }

    #[test]
    fn dangerous_actions_require_confirmation() {
        assert!(NetworkAction::Stop.requires_confirmation());
        assert!(NetworkAction::TunOn.requires_confirmation());
        assert!(!NetworkAction::Status.requires_confirmation());
        assert!(!NetworkAction::Doctor.requires_confirmation());
        assert!(!NetworkAction::ConfigDoctor.requires_confirmation());

        assert!(SettingsAction::GeodataUpdate.requires_confirmation());
        assert!(SettingsAction::KernelUpgrade.requires_confirmation());
        assert!(SettingsAction::SecretReveal.requires_confirmation());
        assert!(!SettingsAction::Doctor.requires_confirmation());
        assert!(!SettingsAction::ConfigView.requires_confirmation());
        assert!(SettingsAction::ConfigView.redact_output());
        assert!(!SettingsAction::SecretReveal.redact_output());

        assert!(TrafficAction::PruneDefault.requires_confirmation());
        assert!(TrafficAction::Reset.requires_confirmation());
        assert!(TrafficAction::CollectorStop.requires_confirmation());
        assert!(TrafficAction::CollectorRestart.requires_confirmation());
        assert!(!TrafficAction::SampleOnce.requires_confirmation());
        assert!(!TrafficAction::CollectorStart.requires_confirmation());
    }
}
