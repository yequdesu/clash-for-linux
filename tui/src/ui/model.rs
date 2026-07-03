use crate::api::ProfileEntry;
use crate::i18n::{SettingsPromptFieldMsg, SubscriptionFieldMsg};
use crate::mouse::{HitboxRegistry, NetworkAction, SettingsAction, TrafficAction};
use crate::ui::utils::{log_line_rank, profile_interval_label};
use crate::widgets::tab_bar::Tab;

pub(crate) struct UiState {
    pub active_page: Tab,
    pub hitboxes: HitboxRegistry,
}

impl UiState {
    pub(crate) fn new(active_page: Tab) -> Self {
        Self {
            active_page,
            hitboxes: HitboxRegistry::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevelFilter {
    Debug,
    Info,
    Warning,
    Error,
}

impl LogLevelFilter {
    pub(crate) fn api_level(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Debug => "Debug",
            Self::Info => "Info",
            Self::Warning => "Warning",
            Self::Error => "Error",
        }
    }

    pub(crate) fn next(self) -> Self {
        match self {
            Self::Debug => Self::Info,
            Self::Info => Self::Warning,
            Self::Warning => Self::Error,
            Self::Error => Self::Debug,
        }
    }

    pub(crate) fn min_rank(self) -> u8 {
        match self {
            Self::Debug => 0,
            Self::Info => 1,
            Self::Warning => 2,
            Self::Error => 3,
        }
    }

    pub(crate) fn matches_line(self, line: &str) -> bool {
        log_line_rank(line) >= self.min_rank()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsSection {
    General,
    Core,
    Traffic,
    Security,
    Diagnostics,
    Updates,
}

impl SettingsSection {
    pub(crate) fn all() -> &'static [SettingsSection] {
        &[
            SettingsSection::General,
            SettingsSection::Core,
            SettingsSection::Traffic,
            SettingsSection::Security,
            SettingsSection::Diagnostics,
            SettingsSection::Updates,
        ]
    }

    pub(crate) fn next(self) -> Self {
        let all = Self::all();
        let idx = all.iter().position(|section| *section == self).unwrap_or(0);
        all[(idx + 1) % all.len()]
    }

    pub(crate) fn prev(self) -> Self {
        let all = Self::all();
        let idx = all.iter().position(|section| *section == self).unwrap_or(0);
        all[(idx + all.len() - 1) % all.len()]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubscriptionEditField {
    ImportDirectory,
    Url,
    Interval,
    UpdateProxy,
    UserAgent,
    ConvertMode,
    AddTag,
    RemoveTag,
}

impl SubscriptionEditField {
    pub(crate) fn msg(self) -> SubscriptionFieldMsg {
        match self {
            Self::ImportDirectory => SubscriptionFieldMsg::ImportDirectory,
            Self::Url => SubscriptionFieldMsg::Source,
            Self::Interval => SubscriptionFieldMsg::Interval,
            Self::UpdateProxy => SubscriptionFieldMsg::UpdateProxy,
            Self::UserAgent => SubscriptionFieldMsg::UserAgent,
            Self::ConvertMode => SubscriptionFieldMsg::ConvertMode,
            Self::AddTag => SubscriptionFieldMsg::AddTag,
            Self::RemoveTag => SubscriptionFieldMsg::RemoveTag,
        }
    }

    pub(crate) fn initial_value(self, profile: &ProfileEntry) -> String {
        match self {
            Self::ImportDirectory => String::new(),
            Self::Url => profile.url.clone(),
            Self::Interval => profile_interval_label(profile),
            Self::UpdateProxy => {
                if profile.update_proxy.is_empty() {
                    "auto".into()
                } else {
                    profile.update_proxy.clone()
                }
            }
            Self::UserAgent => profile.user_agent.clone(),
            Self::ConvertMode => {
                if profile.convert_mode.is_empty() {
                    "auto".into()
                } else {
                    profile.convert_mode.clone()
                }
            }
            Self::AddTag => String::new(),
            Self::RemoveTag => profile.tags.first().cloned().unwrap_or_default(),
        }
    }

    pub(crate) fn args(self, id: i32, value: String) -> Vec<String> {
        let id = id.to_string();
        match self {
            Self::ImportDirectory => vec!["sub".into(), "import".into(), value],
            Self::Url => vec!["sub".into(), "set-url".into(), id, value],
            Self::Interval => vec!["sub".into(), "set-interval".into(), id, value],
            Self::UpdateProxy => vec!["sub".into(), "set-update-proxy".into(), id, value],
            Self::UserAgent => vec!["sub".into(), "set-user-agent".into(), id, value],
            Self::ConvertMode => vec!["sub".into(), "set-convert".into(), id, value],
            Self::AddTag => vec!["sub".into(), "tag".into(), "add".into(), id, value],
            Self::RemoveTag => vec!["sub".into(), "tag".into(), "remove".into(), id, value],
        }
    }

    pub(crate) fn status_target(self, profile_id: i32) -> String {
        match self {
            Self::ImportDirectory => "subscription import".into(),
            _ => format!("subscription {}", profile_id),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrafficRange {
    LastHour,
    SixHours,
    Day,
    Week,
}

impl TrafficRange {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::LastHour => "1h",
            Self::SixHours => "6h",
            Self::Day => "24h",
            Self::Week => "7d",
        }
    }

    pub(crate) fn range_arg(self) -> &'static str {
        match self {
            Self::LastHour => "1h",
            Self::SixHours => "6h",
            Self::Day => "24h",
            Self::Week => "168h",
        }
    }

    pub(crate) fn step_arg(self) -> &'static str {
        match self {
            Self::LastHour => "10s",
            Self::SixHours | Self::Day | Self::Week => "1m",
        }
    }

    pub(crate) fn next(self) -> Self {
        match self {
            Self::LastHour => Self::SixHours,
            Self::SixHours => Self::Day,
            Self::Day => Self::Week,
            Self::Week => Self::LastHour,
        }
    }

    pub(crate) fn prev(self) -> Self {
        match self {
            Self::LastHour => Self::Week,
            Self::SixHours => Self::LastHour,
            Self::Day => Self::SixHours,
            Self::Week => Self::Day,
        }
    }

    pub(crate) fn setting_key(self) -> &'static str {
        match self {
            Self::LastHour => "1h",
            Self::SixHours => "6h",
            Self::Day => "24h",
            Self::Week => "7d",
        }
    }

    pub(crate) fn from_setting_key(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "1h" => Self::LastHour,
            "6h" => Self::SixHours,
            "7d" | "168h" => Self::Week,
            _ => Self::Day,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrafficChartKind {
    Line,
    Bar,
}

impl TrafficChartKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Line => "Line",
            Self::Bar => "Bar",
        }
    }

    pub(crate) fn next(self) -> Self {
        match self {
            Self::Line => Self::Bar,
            Self::Bar => Self::Line,
        }
    }

    pub(crate) fn setting_key(self) -> &'static str {
        match self {
            Self::Line => "line",
            Self::Bar => "bar",
        }
    }

    pub(crate) fn from_setting_key(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "bar" => Self::Bar,
            _ => Self::Line,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrafficDimension {
    Route,
    Rule,
    Group,
    Node,
    Host,
    Process,
    Network,
}

impl TrafficDimension {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Route => "Route",
            Self::Rule => "Rule",
            Self::Group => "Group",
            Self::Node => "Node",
            Self::Host => "Host",
            Self::Process => "Process",
            Self::Network => "Network",
        }
    }

    pub(crate) fn arg(self) -> &'static str {
        match self {
            Self::Route => "route",
            Self::Rule => "rule",
            Self::Group => "group",
            Self::Node => "node",
            Self::Host => "host",
            Self::Process => "process",
            Self::Network => "network",
        }
    }

    pub(crate) fn next(self) -> Self {
        match self {
            Self::Route => Self::Rule,
            Self::Rule => Self::Group,
            Self::Group => Self::Node,
            Self::Node => Self::Host,
            Self::Host => Self::Process,
            Self::Process => Self::Network,
            Self::Network => Self::Route,
        }
    }

    pub(crate) fn setting_key(self) -> &'static str {
        self.arg()
    }

    pub(crate) fn from_setting_key(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "rule" => Self::Rule,
            "group" => Self::Group,
            "node" => Self::Node,
            "host" => Self::Host,
            "process" => Self::Process,
            "network" => Self::Network,
            _ => Self::Route,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubscriptionPrompt {
    pub field: SubscriptionEditField,
    pub profile_id: i32,
    pub value: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubscriptionAddField {
    Source,
    Name,
    Interval,
    UpdateProxy,
    UserAgent,
    ConvertMode,
    Tags,
}

impl SubscriptionAddField {
    pub(crate) fn all() -> &'static [Self] {
        &[
            Self::Source,
            Self::Name,
            Self::Interval,
            Self::UpdateProxy,
            Self::UserAgent,
            Self::ConvertMode,
            Self::Tags,
        ]
    }

    pub(crate) fn msg(self) -> SubscriptionFieldMsg {
        match self {
            Self::Source => SubscriptionFieldMsg::Source,
            Self::Name => SubscriptionFieldMsg::Name,
            Self::Interval => SubscriptionFieldMsg::Interval,
            Self::UpdateProxy => SubscriptionFieldMsg::UpdateProxy,
            Self::UserAgent => SubscriptionFieldMsg::UserAgent,
            Self::ConvertMode => SubscriptionFieldMsg::ConvertMode,
            Self::Tags => SubscriptionFieldMsg::Tags,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubscriptionAddForm {
    pub source: String,
    pub name: String,
    pub interval: String,
    pub update_proxy: String,
    pub user_agent: String,
    pub convert_mode: String,
    pub tags: String,
    pub active: usize,
}

impl SubscriptionAddForm {
    pub(crate) fn new() -> Self {
        Self {
            source: String::new(),
            name: String::new(),
            interval: String::new(),
            update_proxy: "auto".into(),
            user_agent: String::new(),
            convert_mode: "auto".into(),
            tags: String::new(),
            active: 0,
        }
    }

    pub(crate) fn active_field(&self) -> SubscriptionAddField {
        let fields = SubscriptionAddField::all();
        fields[self.active.min(fields.len().saturating_sub(1))]
    }

    pub(crate) fn active_value_mut(&mut self) -> &mut String {
        match self.active_field() {
            SubscriptionAddField::Source => &mut self.source,
            SubscriptionAddField::Name => &mut self.name,
            SubscriptionAddField::Interval => &mut self.interval,
            SubscriptionAddField::UpdateProxy => &mut self.update_proxy,
            SubscriptionAddField::UserAgent => &mut self.user_agent,
            SubscriptionAddField::ConvertMode => &mut self.convert_mode,
            SubscriptionAddField::Tags => &mut self.tags,
        }
    }

    pub(crate) fn set_active(&mut self, idx: usize) {
        self.active = idx.min(SubscriptionAddField::all().len().saturating_sub(1));
    }

    pub(crate) fn next_field(&mut self) {
        self.active = (self.active + 1) % SubscriptionAddField::all().len();
    }

    pub(crate) fn prev_field(&mut self) {
        let len = SubscriptionAddField::all().len();
        self.active = if self.active == 0 {
            len.saturating_sub(1)
        } else {
            self.active - 1
        };
    }

    pub(crate) fn push_char(&mut self, c: char) {
        self.active_value_mut().push(c);
    }

    pub(crate) fn pop_char(&mut self) {
        self.active_value_mut().pop();
    }

    pub(crate) fn args(&self) -> Result<Vec<String>, String> {
        let source = self.source.trim();
        if source.is_empty() {
            return Err("subscription source cannot be empty".into());
        }
        let mut args = vec!["sub".into(), "add".into(), source.into()];
        push_optional_arg(&mut args, "--name", &self.name);
        push_optional_arg(&mut args, "--interval", &self.interval);
        if !self.update_proxy.trim().is_empty() {
            let value = normalize_add_form_enum(
                &self.update_proxy,
                "update proxy",
                &["direct", "system", "core", "auto"],
            )?;
            args.push("--update-proxy".into());
            args.push(value);
        }
        push_optional_arg(&mut args, "--user-agent", &self.user_agent);
        if !self.convert_mode.trim().is_empty() {
            let value = normalize_add_form_enum(
                &self.convert_mode,
                "convert mode",
                &["auto", "off", "force"],
            )?;
            args.push("--convert".into());
            args.push(value);
        }
        for tag in split_add_form_tags(&self.tags) {
            args.push("--tag".into());
            args.push(tag);
        }
        Ok(args)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubscriptionEditForm {
    pub profile_id: i32,
    pub original_name: String,
    pub original_url: String,
    pub original_interval: String,
    pub original_update_proxy: String,
    pub original_user_agent: String,
    pub original_convert_mode: String,
    pub original_tags: Vec<String>,
    pub name: String,
    pub url: String,
    pub interval: String,
    pub update_proxy: String,
    pub user_agent: String,
    pub convert_mode: String,
    pub tags: String,
    pub active: usize,
}

impl SubscriptionEditForm {
    pub(crate) fn from_profile(profile: &ProfileEntry) -> Self {
        let interval = profile_interval_label(profile);
        let update_proxy = if profile.update_proxy.is_empty() {
            "auto".into()
        } else {
            profile.update_proxy.clone()
        };
        let convert_mode = if profile.convert_mode.is_empty() {
            "auto".into()
        } else {
            profile.convert_mode.clone()
        };
        Self {
            profile_id: profile.id,
            original_name: profile.name.clone(),
            original_url: profile.url.clone(),
            original_interval: interval.clone(),
            original_update_proxy: update_proxy.clone(),
            original_user_agent: profile.user_agent.clone(),
            original_convert_mode: convert_mode.clone(),
            original_tags: profile.tags.clone(),
            name: profile.name.clone(),
            url: profile.url.clone(),
            interval,
            update_proxy,
            user_agent: profile.user_agent.clone(),
            convert_mode,
            tags: profile.tags.join(", "),
            active: 0,
        }
    }

    pub(crate) fn active_field(&self) -> SubscriptionAddField {
        let fields = SubscriptionAddField::all();
        fields[self.active.min(fields.len().saturating_sub(1))]
    }

    pub(crate) fn active_value_mut(&mut self) -> &mut String {
        match self.active_field() {
            SubscriptionAddField::Source => &mut self.url,
            SubscriptionAddField::Name => &mut self.name,
            SubscriptionAddField::Interval => &mut self.interval,
            SubscriptionAddField::UpdateProxy => &mut self.update_proxy,
            SubscriptionAddField::UserAgent => &mut self.user_agent,
            SubscriptionAddField::ConvertMode => &mut self.convert_mode,
            SubscriptionAddField::Tags => &mut self.tags,
        }
    }

    pub(crate) fn set_active(&mut self, idx: usize) {
        self.active = idx.min(SubscriptionAddField::all().len().saturating_sub(1));
    }

    pub(crate) fn next_field(&mut self) {
        self.active = (self.active + 1) % SubscriptionAddField::all().len();
    }

    pub(crate) fn prev_field(&mut self) {
        let len = SubscriptionAddField::all().len();
        self.active = if self.active == 0 {
            len.saturating_sub(1)
        } else {
            self.active - 1
        };
    }

    pub(crate) fn push_char(&mut self, c: char) {
        self.active_value_mut().push(c);
    }

    pub(crate) fn pop_char(&mut self) {
        self.active_value_mut().pop();
    }

    pub(crate) fn commands(&self) -> Result<Vec<Vec<String>>, String> {
        let id = self.profile_id.to_string();
        let url = self.url.trim();
        if url.is_empty() {
            return Err("subscription source cannot be empty".into());
        }
        let mut commands = Vec::new();
        let name = self.name.trim();
        if name != self.original_name.trim() {
            if name.is_empty() {
                return Err("subscription name cannot be empty; enter a name or cancel".into());
            }
            commands.push(vec!["sub".into(), "rename".into(), id.clone(), name.into()]);
        }
        if url != self.original_url.trim() {
            commands.push(vec!["sub".into(), "set-url".into(), id.clone(), url.into()]);
        }

        let interval = self.interval.trim();
        if interval != self.original_interval.trim() {
            if interval.is_empty() {
                return Err("update interval cannot be empty; use a duration or off".into());
            }
            commands.push(vec![
                "sub".into(),
                "set-interval".into(),
                id.clone(),
                interval.into(),
            ]);
        }

        let update_proxy = self.update_proxy.trim();
        if update_proxy != self.original_update_proxy.trim() {
            let value = normalize_add_form_enum(
                update_proxy,
                "update proxy",
                &["direct", "system", "core", "auto"],
            )?;
            commands.push(vec![
                "sub".into(),
                "set-update-proxy".into(),
                id.clone(),
                value,
            ]);
        }

        let user_agent = self.user_agent.trim();
        if user_agent != self.original_user_agent.trim() {
            commands.push(vec![
                "sub".into(),
                "set-user-agent".into(),
                id.clone(),
                user_agent.into(),
            ]);
        }

        let convert_mode = self.convert_mode.trim();
        if convert_mode != self.original_convert_mode.trim() {
            let value =
                normalize_add_form_enum(convert_mode, "convert mode", &["auto", "off", "force"])?;
            commands.push(vec!["sub".into(), "set-convert".into(), id.clone(), value]);
        }

        let next_tags = split_add_form_tags(&self.tags);
        for tag in &self.original_tags {
            if !contains_tag(&next_tags, tag) {
                commands.push(vec![
                    "sub".into(),
                    "tag".into(),
                    "remove".into(),
                    id.clone(),
                    tag.clone(),
                ]);
            }
        }
        for tag in next_tags {
            if !contains_tag(&self.original_tags, &tag) {
                commands.push(vec![
                    "sub".into(),
                    "tag".into(),
                    "add".into(),
                    id.clone(),
                    tag,
                ]);
            }
        }
        Ok(commands)
    }
}

pub(crate) fn contains_tag(tags: &[String], tag: &str) -> bool {
    tags.iter()
        .any(|existing| existing.eq_ignore_ascii_case(tag))
}

pub(crate) fn push_optional_arg(args: &mut Vec<String>, flag: &str, value: &str) {
    let value = value.trim();
    if value.is_empty() {
        return;
    }
    args.push(flag.into());
    args.push(value.into());
}

pub(crate) fn normalize_add_form_enum(
    raw: &str,
    label: &str,
    allowed: &[&str],
) -> Result<String, String> {
    let value = raw.trim().to_ascii_lowercase();
    if allowed.iter().any(|candidate| *candidate == value) {
        return Ok(value);
    }
    Err(format!(
        "invalid {} {}, allowed: {}",
        label,
        raw.trim(),
        allowed.join("|")
    ))
}

pub(crate) fn split_add_form_tags(raw: &str) -> Vec<String> {
    let mut tags: Vec<String> = Vec::new();
    for tag in raw.split(',').map(str::trim).filter(|tag| !tag.is_empty()) {
        if !tags
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(tag))
        {
            tags.push(tag.into());
        }
    }
    tags
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsPromptKind {
    SecretSet,
    ConfigSetPorts,
    ConfigSetApi,
    ConfigSetDnsMode,
    ConfigSetLan,
    TrafficPruneRetention,
    GeodataUpdateVersion,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsPromptField {
    pub field: SettingsPromptFieldMsg,
    pub value: String,
    pub mask: bool,
}

impl SettingsPromptField {
    pub(crate) fn new(field: SettingsPromptFieldMsg, value: &'static str, mask: bool) -> Self {
        Self {
            field,
            value: value.into(),
            mask,
        }
    }

    pub(crate) fn display_value(&self) -> String {
        if self.value.is_empty() {
            "<empty>".into()
        } else if self.mask {
            "*".repeat(self.value.chars().count().min(64))
        } else {
            self.value.clone()
        }
    }
}

impl SettingsPromptKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::SecretSet => "Set API secret",
            Self::ConfigSetPorts => "Set proxy ports",
            Self::ConfigSetApi => "Set API controller",
            Self::ConfigSetDnsMode => "Set DNS mode",
            Self::ConfigSetLan => "Set LAN access",
            Self::TrafficPruneRetention => "Prune traffic retention",
            Self::GeodataUpdateVersion => "Update geodata version",
        }
    }

    pub(crate) fn hint(self) -> &'static str {
        match self {
            Self::SecretSet => "Enter the new API secret. It will be hidden in the UI.",
            Self::ConfigSetPorts => "Use: mixed=7897 http=7898 socks=7899. Only mixed is required.",
            Self::ConfigSetApi => {
                "Use: controller=127.0.0.1:9090 secret=<value> allow-unsafe=false."
            }
            Self::ConfigSetDnsMode => "Allowed: fake-ip, redir-host, off.",
            Self::ConfigSetLan => "Allowed: on, off.",
            Self::TrafficPruneRetention => "Use: 24h, 7d, or raw=24h rollup-10s=7d rollup-1m=90d.",
            Self::GeodataUpdateVersion => "Use: latest, or a meta-rules-dat release tag.",
        }
    }

    pub(crate) fn action_id(self) -> &'static str {
        match self {
            Self::SecretSet => "settings.secret.set",
            Self::ConfigSetPorts => "settings.config.set_ports",
            Self::ConfigSetApi => "settings.config.set_api",
            Self::ConfigSetDnsMode => "settings.config.set_dns",
            Self::ConfigSetLan => "settings.config.set_lan",
            Self::TrafficPruneRetention => "settings.traffic.prune_retention",
            Self::GeodataUpdateVersion => "settings.geodata.version",
        }
    }

    pub(crate) fn fields(self) -> Vec<SettingsPromptField> {
        match self {
            Self::SecretSet => vec![SettingsPromptField::new(
                SettingsPromptFieldMsg::Secret,
                "",
                true,
            )],
            Self::ConfigSetPorts => vec![
                SettingsPromptField::new(SettingsPromptFieldMsg::MixedPort, "", false),
                SettingsPromptField::new(SettingsPromptFieldMsg::HttpPort, "", false),
                SettingsPromptField::new(SettingsPromptFieldMsg::SocksPort, "", false),
            ],
            Self::ConfigSetApi => vec![
                SettingsPromptField::new(SettingsPromptFieldMsg::Controller, "", false),
                SettingsPromptField::new(SettingsPromptFieldMsg::Secret, "", true),
                SettingsPromptField::new(SettingsPromptFieldMsg::AllowUnsafe, "false", false),
            ],
            Self::ConfigSetDnsMode => vec![SettingsPromptField::new(
                SettingsPromptFieldMsg::DnsMode,
                "fake-ip",
                false,
            )],
            Self::ConfigSetLan => vec![SettingsPromptField::new(
                SettingsPromptFieldMsg::LanAccess,
                "off",
                false,
            )],
            Self::TrafficPruneRetention => vec![
                SettingsPromptField::new(SettingsPromptFieldMsg::RawRetention, "24h", false),
                SettingsPromptField::new(SettingsPromptFieldMsg::Rollup10s, "7d", false),
                SettingsPromptField::new(SettingsPromptFieldMsg::Rollup1m, "90d", false),
            ],
            Self::GeodataUpdateVersion => vec![SettingsPromptField::new(
                SettingsPromptFieldMsg::Version,
                "latest",
                false,
            )],
        }
    }

    pub(crate) fn fields_value(self, fields: &[SettingsPromptField]) -> String {
        let value = |idx: usize| {
            fields
                .get(idx)
                .map(|field| field.value.trim())
                .unwrap_or("")
        };
        let raw_first = value(0);
        let others_empty = fields
            .iter()
            .skip(1)
            .all(|field| field.value.trim().is_empty() || field.value.trim() == "false");
        if matches!(
            self,
            Self::ConfigSetPorts | Self::ConfigSetApi | Self::TrafficPruneRetention
        ) && others_empty
            && (raw_first.contains('=') || raw_first.split_whitespace().count() > 1)
        {
            return raw_first.into();
        }

        match self {
            Self::SecretSet
            | Self::ConfigSetDnsMode
            | Self::ConfigSetLan
            | Self::GeodataUpdateVersion => raw_first.into(),
            Self::ConfigSetPorts => {
                let mut parts = Vec::new();
                if !value(0).is_empty() {
                    parts.push(format!("mixed={}", value(0)));
                }
                if !value(1).is_empty() {
                    parts.push(format!("http={}", value(1)));
                }
                if !value(2).is_empty() {
                    parts.push(format!("socks={}", value(2)));
                }
                parts.join(" ")
            }
            Self::ConfigSetApi => {
                let mut parts = Vec::new();
                if !value(0).is_empty() {
                    parts.push(format!("controller={}", value(0)));
                }
                if !value(1).is_empty() {
                    parts.push(format!("secret={}", value(1)));
                }
                if !value(2).is_empty() {
                    parts.push(format!("allow-unsafe={}", value(2)));
                }
                parts.join(" ")
            }
            Self::TrafficPruneRetention => {
                let mut parts = Vec::new();
                if !value(0).is_empty() {
                    parts.push(format!("raw={}", value(0)));
                }
                if !value(1).is_empty() {
                    parts.push(format!("rollup-10s={}", value(1)));
                }
                if !value(2).is_empty() {
                    parts.push(format!("rollup-1m={}", value(2)));
                }
                parts.join(" ")
            }
        }
    }

    pub(crate) fn args(self, value: String) -> Result<Vec<String>, String> {
        match self {
            Self::SecretSet => Ok(vec!["secret".into(), value]),
            Self::ConfigSetPorts => parse_config_set_ports_args(&value),
            Self::ConfigSetApi => parse_config_set_api_args(&value),
            Self::ConfigSetDnsMode => {
                let mode = value.trim().to_ascii_lowercase();
                if !matches!(mode.as_str(), "fake-ip" | "redir-host" | "off") {
                    return Err("DNS mode must be fake-ip, redir-host, or off".into());
                }
                Ok(vec!["config".into(), "set-dns-mode".into(), mode])
            }
            Self::ConfigSetLan => {
                let mode = value.trim().to_ascii_lowercase();
                if !matches!(mode.as_str(), "on" | "off") {
                    return Err("LAN access must be on or off".into());
                }
                Ok(vec!["config".into(), "set-lan".into(), mode])
            }
            Self::TrafficPruneRetention => parse_traffic_prune_retention_args(&value),
            Self::GeodataUpdateVersion => parse_geodata_update_version_args(&value),
        }
    }

    pub(crate) fn command_preview(self, args: &[String]) -> String {
        match self {
            Self::SecretSet => "clashctl secret ********".into(),
            Self::ConfigSetApi => format!("clashctl {}", redact_secret_args(args).join(" ")),
            _ => format!("clashctl {}", args.join(" ")),
        }
    }
}

pub(crate) fn parse_config_set_ports_args(value: &str) -> Result<Vec<String>, String> {
    let mut mixed: Option<String> = None;
    let mut http: Option<String> = None;
    let mut socks: Option<String> = None;

    for token in value.split_whitespace() {
        if let Some((raw_key, raw_value)) = token.split_once('=') {
            let key = raw_key.to_ascii_lowercase();
            let port = validate_prompt_port(raw_value)?;
            match key.as_str() {
                "mixed" | "mixed-port" => mixed = Some(port),
                "http" | "port" => http = Some(port),
                "socks" | "socks-port" => socks = Some(port),
                _ => return Err(format!("unsupported port field: {}", raw_key)),
            }
        } else if mixed.is_none() {
            mixed = Some(validate_prompt_port(token)?);
        } else {
            return Err(format!("unexpected port token: {}", token));
        }
    }

    let Some(mixed) = mixed else {
        return Err("mixed port is required".into());
    };
    let mut args = vec!["config".into(), "set-port".into(), mixed];
    if let Some(http) = http {
        args.push("--http".into());
        args.push(http);
    }
    if let Some(socks) = socks {
        args.push("--socks".into());
        args.push(socks);
    }
    Ok(args)
}

pub(crate) fn parse_config_set_api_args(value: &str) -> Result<Vec<String>, String> {
    let mut controller: Option<String> = None;
    let mut secret: Option<String> = None;
    let mut allow_unsafe = false;
    let mut tokens = value.split_whitespace();

    while let Some(token) = tokens.next() {
        if token == "--allow-unsafe" {
            allow_unsafe = true;
            continue;
        }
        if let Some(raw) = token.strip_prefix("--allow-unsafe=") {
            allow_unsafe = parse_prompt_bool(raw, "allow-unsafe")?;
            continue;
        }
        if token == "--secret" {
            let Some(raw_secret) = tokens.next() else {
                return Err("--secret requires a value".into());
            };
            secret = Some(validate_non_empty_prompt_value("secret", raw_secret)?);
            continue;
        }
        if let Some(raw_secret) = token.strip_prefix("--secret=") {
            secret = Some(validate_non_empty_prompt_value("secret", raw_secret)?);
            continue;
        }

        if let Some((raw_key, raw_value)) = token.split_once('=') {
            let key = raw_key.trim().to_ascii_lowercase();
            match key.as_str() {
                "controller" | "api" | "external-controller" => {
                    controller = Some(validate_non_empty_prompt_value(
                        "API controller",
                        raw_value,
                    )?);
                }
                "secret" => {
                    secret = Some(validate_non_empty_prompt_value("secret", raw_value)?);
                }
                "allow-unsafe" | "unsafe" => {
                    allow_unsafe = parse_prompt_bool(raw_value, raw_key)?;
                }
                _ => return Err(format!("unsupported API field: {}", raw_key)),
            }
            continue;
        }

        if controller.is_none() {
            controller = Some(validate_non_empty_prompt_value("API controller", token)?);
        } else {
            return Err(format!("unexpected API token: {}", token));
        }
    }

    let Some(controller) = controller else {
        return Err("API controller cannot be empty".into());
    };
    let mut args = vec!["config".into(), "set-api".into(), controller];
    if let Some(secret) = secret {
        args.push("--secret".into());
        args.push(secret);
    }
    if allow_unsafe {
        args.push("--allow-unsafe".into());
    }
    Ok(args)
}

pub(crate) fn parse_traffic_prune_retention_args(value: &str) -> Result<Vec<String>, String> {
    let mut raw_retention: Option<String> = None;
    let mut rollup_10s_retention: Option<String> = None;
    let mut rollup_1m_retention: Option<String> = None;

    for token in value.split_whitespace() {
        if let Some((raw_key, raw_value)) = token.split_once('=') {
            let key = raw_key.trim().to_ascii_lowercase();
            let duration = normalize_traffic_retention_duration(raw_value)?;
            match key.as_str() {
                "raw" | "retention" => raw_retention = Some(duration),
                "10s" | "rollup-10s" | "rollup-10s-retention" => {
                    rollup_10s_retention = Some(duration);
                }
                "1m" | "rollup-1m" | "rollup-1m-retention" => {
                    rollup_1m_retention = Some(duration);
                }
                _ => return Err(format!("unsupported traffic retention field: {}", raw_key)),
            }
        } else if raw_retention.is_none() {
            raw_retention = Some(normalize_traffic_retention_duration(token)?);
        } else {
            return Err(format!("unexpected traffic retention token: {}", token));
        }
    }

    let Some(raw_retention) = raw_retention else {
        return Err("traffic retention is required".into());
    };
    let mut args = vec![
        "traffic".into(),
        "prune".into(),
        "--retention".into(),
        raw_retention,
    ];
    if let Some(retention) = rollup_10s_retention {
        args.push("--rollup-10s-retention".into());
        args.push(retention);
    }
    if let Some(retention) = rollup_1m_retention {
        args.push("--rollup-1m-retention".into());
        args.push(retention);
    }
    Ok(args)
}

pub(crate) fn parse_geodata_update_version_args(value: &str) -> Result<Vec<String>, String> {
    let version = validate_non_empty_prompt_value("geodata version", value)?;
    if version.split_whitespace().count() != 1 {
        return Err("geodata version must be a single release tag or latest".into());
    }
    Ok(vec![
        "geodata".into(),
        "update".into(),
        "--version".into(),
        version,
    ])
}

pub(crate) fn normalize_traffic_retention_duration(raw: &str) -> Result<String, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("traffic retention cannot be empty".into());
    }
    let chars = raw.chars().collect::<Vec<_>>();
    let mut idx = 0;
    let mut normalized = String::new();
    let mut has_positive_segment = false;

    while idx < chars.len() {
        let number_start = idx;
        while idx < chars.len() && chars[idx].is_ascii_digit() {
            idx += 1;
        }
        if number_start == idx {
            return Err(format!("invalid traffic retention duration: {}", raw));
        }
        let number = chars[number_start..idx].iter().collect::<String>();
        let value: u64 = number
            .parse()
            .map_err(|_| format!("invalid traffic retention duration: {}", raw))?;

        let unit_start = idx;
        while idx < chars.len() && chars[idx].is_ascii_alphabetic() {
            idx += 1;
        }
        if unit_start == idx {
            return Err(format!("traffic retention duration missing unit: {}", raw));
        }
        let unit = chars[unit_start..idx]
            .iter()
            .collect::<String>()
            .to_ascii_lowercase();

        match unit.as_str() {
            "ns" | "us" | "ms" | "s" | "m" | "h" => {
                normalized.push_str(&number);
                normalized.push_str(&unit);
            }
            "d" => {
                let hours = value
                    .checked_mul(24)
                    .ok_or_else(|| format!("traffic retention duration too large: {}", raw))?;
                normalized.push_str(&hours.to_string());
                normalized.push('h');
            }
            _ => {
                return Err(format!(
                    "unsupported traffic retention duration unit: {}",
                    unit
                ))
            }
        }
        has_positive_segment |= value > 0;
    }

    if !has_positive_segment {
        return Err("traffic retention must be positive".into());
    }
    Ok(normalized)
}

pub(crate) fn validate_non_empty_prompt_value(label: &str, raw: &str) -> Result<String, String> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(format!("{} cannot be empty", label));
    }
    Ok(value.into())
}

pub(crate) fn parse_prompt_bool(raw: &str, label: &str) -> Result<bool, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(format!("{} must be true/false or on/off", label)),
    }
}

pub(crate) fn redact_secret_args(args: &[String]) -> Vec<String> {
    let mut redacted = Vec::with_capacity(args.len());
    let mut redact_next = false;
    for arg in args {
        if redact_next {
            redacted.push("********".into());
            redact_next = false;
            continue;
        }
        redacted.push(arg.clone());
        if arg == "--secret" {
            redact_next = true;
        }
    }
    redacted
}

pub(crate) fn validate_prompt_port(raw: &str) -> Result<String, String> {
    let port: u16 = raw
        .trim()
        .parse()
        .map_err(|_| format!("invalid port: {}", raw))?;
    if port == 0 {
        return Err("port must be between 1 and 65535".into());
    }
    Ok(port.to_string())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsPrompt {
    pub kind: SettingsPromptKind,
    pub fields: Vec<SettingsPromptField>,
    pub active: usize,
}

impl SettingsPrompt {
    pub(crate) fn new(kind: SettingsPromptKind) -> Self {
        Self {
            kind,
            fields: kind.fields(),
            active: 0,
        }
    }

    pub(crate) fn active_field(&self) -> Option<&SettingsPromptField> {
        self.fields.get(self.active)
    }

    pub(crate) fn active_field_mut(&mut self) -> Option<&mut SettingsPromptField> {
        self.fields.get_mut(self.active)
    }

    pub(crate) fn next_field(&mut self) {
        if !self.fields.is_empty() {
            self.active = (self.active + 1) % self.fields.len();
        }
    }

    pub(crate) fn prev_field(&mut self) {
        if !self.fields.is_empty() {
            self.active = if self.active == 0 {
                self.fields.len() - 1
            } else {
                self.active - 1
            };
        }
    }

    pub(crate) fn select_field(&mut self, idx: usize) {
        self.active = idx.min(self.fields.len().saturating_sub(1));
    }

    pub(crate) fn push_char(&mut self, c: char) {
        if let Some(field) = self.active_field_mut() {
            field.value.push(c);
        }
    }

    pub(crate) fn pop_char(&mut self) {
        if let Some(field) = self.active_field_mut() {
            field.value.pop();
        }
    }

    pub(crate) fn value(&self) -> String {
        self.kind.fields_value(&self.fields)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PendingAction {
    Network(NetworkAction),
    Settings(SettingsAction),
    SettingsCommand {
        label: String,
        args: Vec<String>,
        redact_output: bool,
    },
    ToggleDangerousConfirmations,
    SubscriptionRemove(i32),
    Traffic(TrafficAction),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingConfirmation {
    pub title: String,
    pub message: String,
    pub action: PendingAction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SudoTarget {
    Network,
    Settings(SettingsAction),
    SettingsCommand { redact_output: bool },
    Traffic,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SudoPrompt {
    pub label: String,
    pub args: Vec<String>,
    pub(crate) target: SudoTarget,
    pub password: String,
}
