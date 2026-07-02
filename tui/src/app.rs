use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Row, Table, TableState};
use ratatui::Frame;
use std::collections::HashMap;
use std::sync::mpsc;

use crate::action_registry::{self, ActionDanger, ActionExecutor};
use crate::api::{
    ApiClient, Connection, ProfileEntry, ProxyInfo, TrafficPoint, TrafficStatus, TrafficTopRow,
};
use crate::background::{BackgroundEffect, NoopBackground};
use crate::config::Config;
use crate::event::DataEvent;
use crate::i18n::{
    tr, tr_action_button, tr_action_desc, tr_action_label, tr_settings_prompt_field_hint,
    tr_settings_prompt_field_label, tr_subscription_field_hint, tr_subscription_field_label, Msg,
    SettingsPromptFieldMsg, SubscriptionFieldMsg,
};
use crate::mouse::{HitboxAction, HitboxRegistry, NetworkAction, SettingsAction, TrafficAction};
use crate::settings::{next_refresh_interval_secs, UiSettings};
use crate::theme::{
    apply_theme_key, next_theme_key, normalize_theme_key, theme_label, CLASH_THEME,
};
use crate::widgets::sparkline::TrafficHistory;
use crate::widgets::tab_bar::Tab;
use crate::window::WindowState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevelFilter {
    Debug,
    Info,
    Warning,
    Error,
}

impl LogLevelFilter {
    fn api_level(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Debug => "Debug",
            Self::Info => "Info",
            Self::Warning => "Warning",
            Self::Error => "Error",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Debug => Self::Info,
            Self::Info => Self::Warning,
            Self::Warning => Self::Error,
            Self::Error => Self::Debug,
        }
    }

    fn min_rank(self) -> u8 {
        match self {
            Self::Debug => 0,
            Self::Info => 1,
            Self::Warning => 2,
            Self::Error => 3,
        }
    }

    fn matches_line(self, line: &str) -> bool {
        log_line_rank(line) >= self.min_rank()
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
    fn msg(self) -> SubscriptionFieldMsg {
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

    fn initial_value(self, profile: &ProfileEntry) -> String {
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

    fn args(self, id: i32, value: String) -> Vec<String> {
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

    fn status_target(self, profile_id: i32) -> String {
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
    fn label(self) -> &'static str {
        match self {
            Self::LastHour => "1h",
            Self::SixHours => "6h",
            Self::Day => "24h",
            Self::Week => "7d",
        }
    }

    fn range_arg(self) -> &'static str {
        match self {
            Self::LastHour => "1h",
            Self::SixHours => "6h",
            Self::Day => "24h",
            Self::Week => "168h",
        }
    }

    fn step_arg(self) -> &'static str {
        match self {
            Self::LastHour => "10s",
            Self::SixHours | Self::Day | Self::Week => "1m",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::LastHour => Self::SixHours,
            Self::SixHours => Self::Day,
            Self::Day => Self::Week,
            Self::Week => Self::LastHour,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::LastHour => Self::Week,
            Self::SixHours => Self::LastHour,
            Self::Day => Self::SixHours,
            Self::Week => Self::Day,
        }
    }

    fn setting_key(self) -> &'static str {
        match self {
            Self::LastHour => "1h",
            Self::SixHours => "6h",
            Self::Day => "24h",
            Self::Week => "7d",
        }
    }

    fn from_setting_key(raw: &str) -> Self {
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
    fn label(self) -> &'static str {
        match self {
            Self::Line => "Line",
            Self::Bar => "Bar",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Line => Self::Bar,
            Self::Bar => Self::Line,
        }
    }

    fn setting_key(self) -> &'static str {
        match self {
            Self::Line => "line",
            Self::Bar => "bar",
        }
    }

    fn from_setting_key(raw: &str) -> Self {
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
    fn label(self) -> &'static str {
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

    fn arg(self) -> &'static str {
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

    fn next(self) -> Self {
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

    fn setting_key(self) -> &'static str {
        self.arg()
    }

    fn from_setting_key(raw: &str) -> Self {
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
    fn all() -> &'static [Self] {
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

    fn msg(self) -> SubscriptionFieldMsg {
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
    fn new() -> Self {
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

    fn active_field(&self) -> SubscriptionAddField {
        let fields = SubscriptionAddField::all();
        fields[self.active.min(fields.len().saturating_sub(1))]
    }

    fn active_value_mut(&mut self) -> &mut String {
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

    fn set_active(&mut self, idx: usize) {
        self.active = idx.min(SubscriptionAddField::all().len().saturating_sub(1));
    }

    fn next_field(&mut self) {
        self.active = (self.active + 1) % SubscriptionAddField::all().len();
    }

    fn prev_field(&mut self) {
        let len = SubscriptionAddField::all().len();
        self.active = if self.active == 0 {
            len.saturating_sub(1)
        } else {
            self.active - 1
        };
    }

    fn push_char(&mut self, c: char) {
        self.active_value_mut().push(c);
    }

    fn pop_char(&mut self) {
        self.active_value_mut().pop();
    }

    fn args(&self) -> Result<Vec<String>, String> {
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
    fn from_profile(profile: &ProfileEntry) -> Self {
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

    fn active_field(&self) -> SubscriptionAddField {
        let fields = SubscriptionAddField::all();
        fields[self.active.min(fields.len().saturating_sub(1))]
    }

    fn active_value_mut(&mut self) -> &mut String {
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

    fn set_active(&mut self, idx: usize) {
        self.active = idx.min(SubscriptionAddField::all().len().saturating_sub(1));
    }

    fn next_field(&mut self) {
        self.active = (self.active + 1) % SubscriptionAddField::all().len();
    }

    fn prev_field(&mut self) {
        let len = SubscriptionAddField::all().len();
        self.active = if self.active == 0 {
            len.saturating_sub(1)
        } else {
            self.active - 1
        };
    }

    fn push_char(&mut self, c: char) {
        self.active_value_mut().push(c);
    }

    fn pop_char(&mut self) {
        self.active_value_mut().pop();
    }

    fn commands(&self) -> Result<Vec<Vec<String>>, String> {
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

fn contains_tag(tags: &[String], tag: &str) -> bool {
    tags.iter()
        .any(|existing| existing.eq_ignore_ascii_case(tag))
}

fn push_optional_arg(args: &mut Vec<String>, flag: &str, value: &str) {
    let value = value.trim();
    if value.is_empty() {
        return;
    }
    args.push(flag.into());
    args.push(value.into());
}

fn normalize_add_form_enum(raw: &str, label: &str, allowed: &[&str]) -> Result<String, String> {
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

fn split_add_form_tags(raw: &str) -> Vec<String> {
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
    fn new(field: SettingsPromptFieldMsg, value: &'static str, mask: bool) -> Self {
        Self {
            field,
            value: value.into(),
            mask,
        }
    }

    fn display_value(&self) -> String {
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
    fn label(self) -> &'static str {
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

    fn hint(self) -> &'static str {
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

    fn action_id(self) -> &'static str {
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

    fn fields(self) -> Vec<SettingsPromptField> {
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

    fn fields_value(self, fields: &[SettingsPromptField]) -> String {
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

    fn args(self, value: String) -> Result<Vec<String>, String> {
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

    fn command_preview(self, args: &[String]) -> String {
        match self {
            Self::SecretSet => "clashctl secret ********".into(),
            Self::ConfigSetApi => format!("clashctl {}", redact_secret_args(args).join(" ")),
            _ => format!("clashctl {}", args.join(" ")),
        }
    }
}

fn parse_config_set_ports_args(value: &str) -> Result<Vec<String>, String> {
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

fn parse_config_set_api_args(value: &str) -> Result<Vec<String>, String> {
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

fn parse_traffic_prune_retention_args(value: &str) -> Result<Vec<String>, String> {
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

fn parse_geodata_update_version_args(value: &str) -> Result<Vec<String>, String> {
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

fn normalize_traffic_retention_duration(raw: &str) -> Result<String, String> {
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

fn validate_non_empty_prompt_value(label: &str, raw: &str) -> Result<String, String> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(format!("{} cannot be empty", label));
    }
    Ok(value.into())
}

fn parse_prompt_bool(raw: &str, label: &str) -> Result<bool, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(format!("{} must be true/false or on/off", label)),
    }
}

fn redact_secret_args(args: &[String]) -> Vec<String> {
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

fn validate_prompt_port(raw: &str) -> Result<String, String> {
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
    fn new(kind: SettingsPromptKind) -> Self {
        Self {
            kind,
            fields: kind.fields(),
            active: 0,
        }
    }

    fn active_field(&self) -> Option<&SettingsPromptField> {
        self.fields.get(self.active)
    }

    fn active_field_mut(&mut self) -> Option<&mut SettingsPromptField> {
        self.fields.get_mut(self.active)
    }

    fn next_field(&mut self) {
        if !self.fields.is_empty() {
            self.active = (self.active + 1) % self.fields.len();
        }
    }

    fn prev_field(&mut self) {
        if !self.fields.is_empty() {
            self.active = if self.active == 0 {
                self.fields.len() - 1
            } else {
                self.active - 1
            };
        }
    }

    fn select_field(&mut self, idx: usize) {
        self.active = idx.min(self.fields.len().saturating_sub(1));
    }

    fn push_char(&mut self, c: char) {
        if let Some(field) = self.active_field_mut() {
            field.value.push(c);
        }
    }

    fn pop_char(&mut self) {
        if let Some(field) = self.active_field_mut() {
            field.value.pop();
        }
    }

    fn value(&self) -> String {
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

pub struct App {
    pub tab: Tab,
    pub should_quit: bool,
    pub show_help: bool,
    pub error_msg: Option<String>,
    pub status_msg: Option<String>,
    pub tick_count: u64,
    pub last_refresh: i64,

    pub version: String,
    pub mode: String,
    pub upload_total: u64,
    pub download_total: u64,
    pub prev_upload: u64,
    pub prev_download: u64,
    pub upload_rate: f64,
    pub download_rate: f64,
    pub start_time: i64,
    pub os_info: String,
    pub arch_info: String,

    pub proxies: HashMap<String, ProxyInfo>,
    pub proxy_groups: Vec<(String, String)>,
    pub selected_proxy_idx: usize,
    pub node_picker_open: bool,
    pub selected_node_idx: usize,
    pub proxy_table_state: TableState,
    pub delays: HashMap<String, u64>,

    pub connections: Vec<Connection>,
    pub connections_active: usize,
    pub connections_total: usize,
    pub connections_table_state: TableState,
    pub connections_selected: usize,

    pub logs: Vec<String>,
    pub log_scroll: usize,
    pub log_paused: bool,
    pub log_level: LogLevelFilter,

    pub profiles: Vec<ProfileEntry>,
    pub active_profile_id: i32,
    pub selected_sub_idx: usize,
    pub subscription_prompt: Option<SubscriptionPrompt>,
    pub subscription_add_form: Option<SubscriptionAddForm>,
    pub subscription_edit_form: Option<SubscriptionEditForm>,
    pub subscription_output: Vec<String>,
    pub pending_confirmation: Option<PendingConfirmation>,
    pub network_output: Vec<String>,
    pub settings_output: Vec<String>,
    pub settings_prompt: Option<SettingsPrompt>,
    pub tun_enabled: bool,
    pub traffic_points: Vec<TrafficPoint>,
    pub traffic_top: Vec<TrafficTopRow>,
    pub traffic_status: TrafficStatus,
    pub traffic_selected_idx: usize,
    pub traffic_error: Option<String>,
    pub traffic_output: Vec<String>,
    pub traffic_range: TrafficRange,
    pub traffic_chart: TrafficChartKind,
    pub traffic_dimension: TrafficDimension,
    pub traffic_filter_key: Option<String>,
    pub traffic_window_offset: usize,
    pub traffic_locked_bucket: Option<usize>,
    pub hitboxes: HitboxRegistry,

    pub command_palette_open: bool,
    pub command_query: String,
    pub command_selected_idx: usize,
    pub search_active: bool,
    pub search_query: String,
    pub sort_mode: bool,        // false=by name, true=by delay
    pub proxy_mode_str: String, // Rule, Global, Direct
    pub window: WindowState,
    pub background: Box<dyn BackgroundEffect>,
    pub ui_settings: UiSettings,

    pub api: ApiClient,
    #[allow(dead_code)]
    pub config: Config,
    pub rt: tokio::runtime::Handle,
    pub data_tx: mpsc::Sender<DataEvent>,

    pub traffic_history: TrafficHistory,
}

impl App {
    pub fn new(
        config: Config,
        rt: tokio::runtime::Handle,
        data_tx: mpsc::Sender<DataEvent>,
    ) -> Self {
        let api = ApiClient::new(config.api_url.clone(), config.api_key.clone());
        let mut ui_settings = UiSettings::load();
        ui_settings.theme = normalize_theme_key(&ui_settings.theme).into();
        apply_theme_key(&ui_settings.theme);
        let profiles_meta = crate::api::read_profiles();
        let initial_tab = initial_tab_for_profiles(
            profiles_meta.profiles.len(),
            ui_settings.default_page.as_str(),
        );
        let traffic_range = TrafficRange::from_setting_key(&ui_settings.traffic_default_range);
        let traffic_chart = TrafficChartKind::from_setting_key(&ui_settings.traffic_default_chart);
        let traffic_dimension =
            TrafficDimension::from_setting_key(&ui_settings.traffic_default_dimension);
        Self {
            tab: initial_tab,
            should_quit: false,
            show_help: false,
            error_msg: None,
            status_msg: None,
            tick_count: 0,
            last_refresh: 0,
            version: String::new(),
            mode: "Rule".into(),
            upload_total: 0,
            download_total: 0,
            prev_upload: 0,
            prev_download: 0,
            upload_rate: 0.0,
            download_rate: 0.0,
            start_time: chrono::Utc::now().timestamp(),
            os_info: read_os_info(),
            arch_info: std::env::consts::ARCH.to_string(),
            proxies: HashMap::new(),
            proxy_groups: Vec::new(),
            selected_proxy_idx: 0,
            node_picker_open: false,
            selected_node_idx: 0,
            proxy_table_state: TableState::default(),
            delays: HashMap::new(),
            connections: Vec::new(),
            connections_active: 0,
            connections_total: 0,
            connections_table_state: TableState::default(),
            connections_selected: 0,
            logs: Vec::new(),
            log_scroll: 0,
            log_paused: false,
            log_level: LogLevelFilter::Info,
            profiles: profiles_meta.profiles,
            active_profile_id: profiles_meta.active_id,
            selected_sub_idx: 0,
            subscription_prompt: None,
            subscription_add_form: None,
            subscription_edit_form: None,
            subscription_output: Vec::new(),
            pending_confirmation: None,
            network_output: Vec::new(),
            settings_output: Vec::new(),
            settings_prompt: None,
            tun_enabled: crate::api::read_tun_status(),
            traffic_points: Vec::new(),
            traffic_top: Vec::new(),
            traffic_status: TrafficStatus::default(),
            traffic_selected_idx: 0,
            traffic_error: None,
            traffic_output: Vec::new(),
            traffic_range,
            traffic_chart,
            traffic_dimension,
            traffic_filter_key: None,
            traffic_window_offset: 0,
            traffic_locked_bucket: None,
            hitboxes: HitboxRegistry::default(),
            command_palette_open: false,
            command_query: String::new(),
            command_selected_idx: 0,
            search_active: false,
            search_query: String::new(),
            sort_mode: false,
            proxy_mode_str: "Rule".into(),
            window: WindowState::load(),
            background: Box::new(NoopBackground),
            ui_settings,
            api,
            config,
            rt,
            data_tx,
            traffic_history: TrafficHistory::new(60),
        }
    }

    pub fn on_tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);
        let now = chrono::Utc::now().timestamp();
        let refresh_interval = self.ui_settings.effective_refresh_interval_secs() as i64;
        if now - self.last_refresh >= refresh_interval {
            self.last_refresh = now;
            self.refresh_data();
        }
    }

    pub fn refresh_data(&mut self) {
        self.fetch_proxies();
        self.fetch_connections();
        self.fetch_version();
        self.refresh_subscriptions();
        self.tun_enabled = crate::api::read_tun_status();
        if self.tick_count.is_multiple_of(3) && !self.proxy_groups.is_empty() {
            self.test_selected_delay();
        }
        if self.tab == Tab::Logs && !self.log_paused {
            self.fetch_logs();
        }
        if self.tab == Tab::Traffic && self.tick_count.is_multiple_of(3) {
            self.fetch_traffic();
        }
    }

    fn fetch_proxies(&mut self) {
        let api = self.api.clone();
        let tx = self.data_tx.clone();
        self.rt.spawn(async move {
            let result = api.get_proxies().await;
            let _ = tx.send(DataEvent::Proxies(result));
        });
    }

    fn fetch_connections(&mut self) {
        let api = self.api.clone();
        let tx = self.data_tx.clone();
        self.rt.spawn(async move {
            let result = api.get_connections().await;
            let _ = tx.send(DataEvent::Connections(result));
        });
    }

    fn fetch_version(&mut self) {
        let api = self.api.clone();
        let tx = self.data_tx.clone();
        self.rt.spawn(async move {
            let result = api.get_version().await;
            let _ = tx.send(DataEvent::Version(result));
        });
    }

    fn fetch_logs(&mut self) {
        let api = self.api.clone();
        let tx = self.data_tx.clone();
        let level = self.log_level.api_level().to_string();
        self.rt.spawn(async move {
            let result = api.get_logs(&level).await;
            let _ = tx.send(DataEvent::Logs(result));
        });
    }

    fn fetch_traffic(&mut self) {
        let tx = self.data_tx.clone();
        let range = self.traffic_range.range_arg().to_string();
        let step = self.traffic_range.step_arg().to_string();
        let by = self.traffic_dimension.arg().to_string();
        let key = self.traffic_filter_key.clone();
        self.rt.spawn(async move {
            let result =
                crate::api::read_traffic_snapshot(&range, &step, &by, key.as_deref()).await;
            let _ = tx.send(DataEvent::Traffic(result));
        });
    }

    pub fn apply_data_event(&mut self, event: DataEvent) {
        match event {
            DataEvent::Proxies(Ok(resp)) => {
                self.proxies = resp.proxies;
                self.proxy_groups.clear();
                for (name, info) in &self.proxies {
                    if info.proxy_type == "Selector"
                        || info.proxy_type == "Fallback"
                        || info.proxy_type == "URLTest"
                    {
                        let current = info.now.clone().unwrap_or_default();
                        self.proxy_groups.push((name.clone(), current));
                    }
                }
                self.clamp_proxy_selection();
            }
            DataEvent::Proxies(Err(e)) => {
                if self.error_msg.is_none() {
                    self.error_msg = Some(e);
                }
            }
            DataEvent::Connections(Ok(resp)) => {
                self.prev_upload = self.upload_total;
                self.prev_download = self.download_total;
                self.upload_total = resp.upload_total;
                self.download_total = resp.download_total;
                self.upload_rate = (self.upload_total.saturating_sub(self.prev_upload)) as f64;
                self.download_rate =
                    (self.download_total.saturating_sub(self.prev_download)) as f64;
                let conn_len = resp.connections.len();
                self.connections = resp.connections;
                self.connections_active = self.connections.len();
                self.connections_total = conn_len;
                self.traffic_history
                    .push(resp.upload_total as f64, resp.download_total as f64);
            }
            DataEvent::Connections(Err(_)) => {}
            DataEvent::Version(Ok(info)) => {
                self.version = info.version.unwrap_or_default();
                self.mode = mode_display(&info.mode);
                self.proxy_mode_str = self.mode.clone();
            }
            DataEvent::Version(Err(_)) => {}
            DataEvent::Logs(Ok(entries)) => {
                if self.log_paused {
                    return;
                }
                for e in entries {
                    let line = format!(
                        "{} {}",
                        e.level.to_uppercase(),
                        &e.payload[..e.payload.len().min(120)]
                    );
                    self.logs.push(line);
                }
                if self.logs.len() > 500 {
                    self.logs.drain(0..self.logs.len() - 500);
                }
                self.log_scroll = self.visible_logs().len().saturating_sub(1);
            }
            DataEvent::Logs(Err(_)) => {}
            DataEvent::Delay(name, delay) => {
                self.delays.insert(name, delay);
            }
            DataEvent::SwitchResult(Ok(())) => {
                self.error_msg = None;
                self.status_msg = Some("proxy switched".into());
                self.refresh_data();
            }
            DataEvent::SwitchResult(Err(e)) => {
                self.error_msg = Some(format!("Switch failed: {}", e));
            }
            DataEvent::ModeResult(Ok(mode)) => {
                self.mode = mode.clone();
                self.proxy_mode_str = mode;
                self.error_msg = None;
                self.status_msg = Some("mode switched".into());
                self.refresh_data();
            }
            DataEvent::ModeResult(Err(e)) => {
                self.error_msg = Some(format!("Mode switch failed: {}", e));
            }
            DataEvent::SubscriptionResult(Ok(msg)) => {
                self.error_msg = None;
                self.status_msg = Some(msg);
                self.subscription_output.clear();
                self.refresh_subscriptions();
                self.refresh_data();
            }
            DataEvent::SubscriptionResult(Err(e)) => {
                self.error_msg = Some(format!("Subscription action failed: {}", e));
                self.subscription_output = command_output_lines(&e);
            }
            DataEvent::SubscriptionOutputResult(label, Ok(output)) => {
                self.error_msg = None;
                self.status_msg = Some(format!("subscription action completed: {}", label));
                self.subscription_output = command_output_lines(&output);
            }
            DataEvent::SubscriptionOutputResult(label, Err(e)) => {
                self.error_msg = Some(format!("Subscription action failed: {}: {}", label, e));
                self.subscription_output = command_output_lines(&e);
            }
            DataEvent::NetworkResult(label, Ok(output)) => {
                self.error_msg = None;
                self.status_msg = Some(format!("network action completed: {}", label));
                self.network_output = command_output_lines(&output);
                self.tun_enabled = crate::api::read_tun_status();
                self.refresh_data();
            }
            DataEvent::NetworkResult(label, Err(e)) => {
                self.error_msg = Some(format!("Network action failed: {}: {}", label, e));
                self.network_output = command_output_lines(&e);
                self.tun_enabled = crate::api::read_tun_status();
            }
            DataEvent::SettingsResult(action, Ok(output)) => {
                self.error_msg = None;
                let label = action.label();
                self.status_msg = Some(format!("settings action completed: {}", label));
                let output = if action.redact_output() {
                    redact_sensitive_output(&output)
                } else {
                    output
                };
                self.settings_output = command_output_lines(&output);
                self.refresh_data();
            }
            DataEvent::SettingsResult(action, Err(e)) => {
                let label = action.label();
                self.error_msg = Some(format!("Settings action failed: {}: {}", label, e));
                self.settings_output = command_output_lines(&redact_sensitive_output(&e));
            }
            DataEvent::SettingsCommandResult(label, redact_output, Ok(output)) => {
                self.error_msg = None;
                self.status_msg = Some(format!("settings action completed: {}", label));
                let output = if redact_output {
                    redact_sensitive_output(&output)
                } else {
                    output
                };
                self.settings_output = command_output_lines(&output);
                self.refresh_data();
            }
            DataEvent::SettingsCommandResult(label, _redact_output, Err(e)) => {
                self.error_msg = Some(format!("Settings action failed: {}: {}", label, e));
                self.settings_output = command_output_lines(&redact_sensitive_output(&e));
            }
            DataEvent::Traffic(Ok(snapshot)) => {
                self.traffic_points = snapshot.history;
                self.traffic_top = snapshot.top;
                self.traffic_status = snapshot.status;
                self.traffic_error = None;
                self.clamp_traffic_selection();
                self.clamp_traffic_window();
            }
            DataEvent::Traffic(Err(e)) => {
                self.traffic_error = Some(e);
            }
            DataEvent::TrafficExport(Ok(path)) => {
                self.error_msg = None;
                self.status_msg = Some(format!("traffic exported: {}", path));
                self.traffic_output = vec![format!("exported: {}", path)];
            }
            DataEvent::TrafficExport(Err(e)) => {
                self.error_msg = Some(format!("Traffic export failed: {}", e));
                self.traffic_output = command_output_lines(&e);
            }
            DataEvent::TrafficActionResult(label, Ok(output)) => {
                self.error_msg = None;
                self.status_msg = Some(format!("traffic action completed: {}", label));
                self.traffic_output = command_output_lines(&output);
                self.refresh_traffic();
            }
            DataEvent::TrafficActionResult(label, Err(e)) => {
                self.error_msg = Some(format!("Traffic action failed: {}: {}", label, e));
                self.traffic_output = command_output_lines(&e);
            }
        }
    }

    pub fn next_tab(&mut self) {
        self.tab = self.tab.next();
        self.show_help = false;
        self.node_picker_open = false;
        self.reset_selection();
        if self.tab == Tab::Traffic {
            self.fetch_traffic();
        }
    }
    pub fn prev_tab(&mut self) {
        self.tab = self.tab.prev();
        self.show_help = false;
        self.node_picker_open = false;
        self.reset_selection();
        if self.tab == Tab::Traffic {
            self.fetch_traffic();
        }
    }

    fn reset_selection(&mut self) {
        self.selected_proxy_idx = 0;
        self.selected_node_idx = 0;
        self.connections_selected = 0;
        self.selected_sub_idx = 0;
        self.traffic_selected_idx = 0;
    }

    pub fn clamp_proxy_selection(&mut self) {
        let visible_len = self.visible_proxy_groups().len();
        if visible_len == 0 {
            self.selected_proxy_idx = 0;
            self.node_picker_open = false;
        } else if self.selected_proxy_idx >= visible_len {
            self.selected_proxy_idx = visible_len - 1;
        }
        self.clamp_node_selection();
    }

    pub fn visible_proxy_groups(&self) -> Vec<(String, String)> {
        let query = self.search_query.to_lowercase();
        let mut groups: Vec<(String, String)> = self
            .proxy_groups
            .iter()
            .filter(|(name, current)| {
                query.is_empty()
                    || name.to_lowercase().contains(&query)
                    || current.to_lowercase().contains(&query)
            })
            .cloned()
            .collect();

        if self.sort_mode {
            groups.sort_by(|a, b| {
                let da = self.delays.get(&a.0).copied().unwrap_or(u64::MAX);
                let db = self.delays.get(&b.0).copied().unwrap_or(u64::MAX);
                da.cmp(&db).then_with(|| a.0.cmp(&b.0))
            });
        } else {
            groups.sort_by(|a, b| a.0.cmp(&b.0));
        }
        groups
    }

    fn selected_proxy_group(&self) -> Option<(String, String)> {
        let groups = self.visible_proxy_groups();
        if groups.is_empty() {
            None
        } else {
            groups
                .get(self.selected_proxy_idx.min(groups.len() - 1))
                .cloned()
        }
    }

    fn selected_proxy_group_name(&self) -> Option<String> {
        self.selected_proxy_group().map(|(name, _)| name)
    }

    fn selected_proxy_nodes(&self) -> Vec<String> {
        let Some(group_name) = self.selected_proxy_group_name() else {
            return vec![];
        };
        self.proxies
            .get(&group_name)
            .and_then(|info| info.all.clone())
            .unwrap_or_default()
    }

    fn selected_proxy_current_node(&self) -> String {
        let Some(group_name) = self.selected_proxy_group_name() else {
            return String::new();
        };
        self.proxies
            .get(&group_name)
            .and_then(|info| info.now.clone())
            .unwrap_or_default()
    }

    fn clamp_node_selection(&mut self) {
        let nodes = self.selected_proxy_nodes();
        if nodes.is_empty() {
            self.selected_node_idx = 0;
            return;
        }
        if self.selected_node_idx >= nodes.len() {
            self.selected_node_idx = nodes.len() - 1;
        }
    }

    fn clamp_subscription_selection(&mut self) {
        if self.profiles.is_empty() {
            self.selected_sub_idx = 0;
        } else if self.selected_sub_idx >= self.profiles.len() {
            self.selected_sub_idx = self.profiles.len() - 1;
        }
    }

    fn clamp_traffic_selection(&mut self) {
        if self.traffic_top.is_empty() {
            self.traffic_selected_idx = 0;
        } else if self.traffic_selected_idx >= self.traffic_top.len() {
            self.traffic_selected_idx = self.traffic_top.len() - 1;
        }
    }

    fn clamp_traffic_window(&mut self) {
        if self.traffic_points.is_empty() {
            self.traffic_window_offset = 0;
            self.traffic_locked_bucket = None;
            return;
        }
        let max_idx = self.traffic_points.len() - 1;
        self.traffic_window_offset = self.traffic_window_offset.min(max_idx);
        if let Some(idx) = self.traffic_locked_bucket {
            self.traffic_locked_bucket = Some(idx.min(max_idx));
        }
    }

    fn selected_subscription_id(&self) -> Option<i32> {
        self.profiles.get(self.selected_sub_idx).map(|p| p.id)
    }

    fn selected_subscription(&self) -> Option<&ProfileEntry> {
        self.profiles.get(self.selected_sub_idx)
    }

    pub fn begin_subscription_edit(&mut self, field: SubscriptionEditField) {
        if self.tab != Tab::Subscriptions {
            return;
        }
        let Some(profile) = self.selected_subscription() else {
            self.status_msg = Some("no subscription selected".into());
            return;
        };
        let profile_id = profile.id;
        let value = field.initial_value(profile);
        self.subscription_add_form = None;
        self.subscription_edit_form = None;
        self.subscription_prompt = Some(SubscriptionPrompt {
            field,
            profile_id,
            value,
        });
        self.error_msg = None;
        self.status_msg = Some(format!(
            "editing subscription {}",
            self.subscription_edit_field_label(field)
        ));
    }

    pub fn begin_subscription_profile_edit(&mut self) {
        if self.tab != Tab::Subscriptions {
            return;
        }
        let Some(profile) = self.selected_subscription() else {
            self.status_msg = Some("no subscription selected".into());
            return;
        };
        let profile_id = profile.id;
        let form = SubscriptionEditForm::from_profile(profile);
        self.subscription_prompt = None;
        self.subscription_add_form = None;
        self.subscription_edit_form = Some(form);
        self.error_msg = None;
        self.status_msg = Some(format!("editing subscription {}", profile_id));
    }

    pub fn begin_subscription_add(&mut self) {
        if self.tab != Tab::Subscriptions {
            return;
        }
        self.subscription_prompt = None;
        self.subscription_edit_form = None;
        self.subscription_add_form = Some(SubscriptionAddForm::new());
        self.error_msg = None;
        self.status_msg = Some("adding subscription".into());
    }

    pub fn begin_subscription_import(&mut self) {
        if self.tab != Tab::Subscriptions {
            return;
        }
        self.subscription_add_form = None;
        self.subscription_edit_form = None;
        self.subscription_prompt = Some(SubscriptionPrompt {
            field: SubscriptionEditField::ImportDirectory,
            profile_id: 0,
            value: String::new(),
        });
        self.error_msg = None;
        self.status_msg = Some("importing subscriptions".into());
    }

    pub fn cancel_subscription_prompt(&mut self) {
        if self.subscription_prompt.take().is_some()
            || self.subscription_add_form.take().is_some()
            || self.subscription_edit_form.take().is_some()
        {
            self.status_msg = Some("subscription edit cancelled".into());
        }
    }

    pub fn push_subscription_prompt_char(&mut self, c: char) {
        if let Some(form) = self.subscription_add_form.as_mut() {
            form.push_char(c);
            return;
        }
        if let Some(form) = self.subscription_edit_form.as_mut() {
            form.push_char(c);
            return;
        }
        if let Some(prompt) = self.subscription_prompt.as_mut() {
            prompt.value.push(c);
        }
    }

    pub fn pop_subscription_prompt_char(&mut self) {
        if let Some(form) = self.subscription_add_form.as_mut() {
            form.pop_char();
            return;
        }
        if let Some(form) = self.subscription_edit_form.as_mut() {
            form.pop_char();
            return;
        }
        if let Some(prompt) = self.subscription_prompt.as_mut() {
            prompt.value.pop();
        }
    }

    pub fn subscription_input_active(&self) -> bool {
        self.subscription_prompt.is_some()
            || self.subscription_add_form.is_some()
            || self.subscription_edit_form.is_some()
    }

    pub fn next_subscription_form_field(&mut self) {
        if let Some(form) = self.subscription_add_form.as_mut() {
            form.next_field();
        } else if let Some(form) = self.subscription_edit_form.as_mut() {
            form.next_field();
        }
    }

    pub fn prev_subscription_form_field(&mut self) {
        if let Some(form) = self.subscription_add_form.as_mut() {
            form.prev_field();
        } else if let Some(form) = self.subscription_edit_form.as_mut() {
            form.prev_field();
        }
    }

    pub fn select_subscription_add_field(&mut self, idx: usize) {
        if let Some(form) = self.subscription_add_form.as_mut() {
            form.set_active(idx);
        } else if let Some(form) = self.subscription_edit_form.as_mut() {
            form.set_active(idx);
        }
    }

    pub fn submit_subscription_prompt(&mut self) {
        if let Some(form) = self.subscription_add_form.take() {
            let args = match form.args() {
                Ok(args) => args,
                Err(e) => {
                    self.error_msg = Some(e);
                    self.subscription_add_form = Some(form);
                    return;
                }
            };
            self.run_subscription_args("Add subscription".into(), "new subscription".into(), args);
            return;
        }
        if let Some(form) = self.subscription_edit_form.take() {
            let profile_id = form.profile_id;
            let commands = match form.commands() {
                Ok(commands) => commands,
                Err(e) => {
                    self.error_msg = Some(e);
                    self.subscription_edit_form = Some(form);
                    return;
                }
            };
            if commands.is_empty() {
                self.status_msg = Some(format!("subscription {} unchanged", profile_id));
                return;
            }
            self.run_subscription_command_sequence(
                "Edit subscription".into(),
                format!("subscription {}", profile_id),
                commands,
            );
            return;
        }

        let Some(prompt) = self.subscription_prompt.take() else {
            return;
        };
        let value = prompt.value.trim().to_string();
        if value.is_empty() {
            self.error_msg = Some(format!(
                "{} cannot be empty",
                self.subscription_edit_field_label(prompt.field)
            ));
            self.subscription_prompt = Some(prompt);
            return;
        }

        let args = prompt.field.args(prompt.profile_id, value);
        let field_label = self.subscription_edit_field_label(prompt.field).to_string();
        let profile_id = prompt.profile_id;
        let target = prompt.field.status_target(profile_id);
        self.run_subscription_args(field_label, target, args);
    }

    fn run_subscription_args(&mut self, field_label: String, target: String, args: Vec<String>) {
        self.run_subscription_command_sequence(field_label, target, vec![args]);
    }

    fn run_subscription_command_sequence(
        &mut self,
        field_label: String,
        target: String,
        commands: Vec<Vec<String>>,
    ) {
        let tx = self.data_tx.clone();
        self.status_msg = Some(format!("running {} for {}", field_label, target));
        self.rt.spawn(async move {
            let mut outputs = Vec::new();
            let mut failed = None;
            for args in commands {
                match crate::api::run_clashctl(&args).await {
                    Ok(stdout) => {
                        if !stdout.is_empty() {
                            outputs.push(stdout);
                        }
                    }
                    Err(e) => {
                        failed = Some(format!("clashctl {}: {}", args.join(" "), e));
                        break;
                    }
                }
            }
            let result = if let Some(e) = failed {
                Err(e)
            } else if outputs.is_empty() {
                Ok(format!("{} completed for {}", field_label, target))
            } else {
                Ok(outputs.join("\n"))
            };
            let _ = tx.send(DataEvent::SubscriptionResult(result));
        });
    }

    pub fn select_down(&mut self) {
        if self.node_picker_open {
            self.select_node_down();
            return;
        }
        match self.tab {
            Tab::Proxies => {
                let len = self.visible_proxy_groups().len();
                if len == 0 {
                    return;
                }
                self.selected_proxy_idx = (self.selected_proxy_idx + 1) % len;
            }
            Tab::Connections => {
                if self.connections.is_empty() {
                    return;
                }
                self.connections_selected =
                    (self.connections_selected + 1) % self.connections.len();
            }
            Tab::Subscriptions => {
                if self.profiles.is_empty() {
                    return;
                }
                self.selected_sub_idx = (self.selected_sub_idx + 1) % self.profiles.len();
            }
            Tab::Traffic => {
                if self.traffic_top.is_empty() {
                    return;
                }
                self.traffic_selected_idx =
                    (self.traffic_selected_idx + 1) % self.traffic_top.len();
            }
            _ => {}
        }
    }

    pub fn select_up(&mut self) {
        if self.node_picker_open {
            self.select_node_up();
            return;
        }
        match self.tab {
            Tab::Proxies => {
                let len = self.visible_proxy_groups().len();
                if len == 0 {
                    return;
                }
                self.selected_proxy_idx = if self.selected_proxy_idx == 0 {
                    len.saturating_sub(1)
                } else {
                    self.selected_proxy_idx - 1
                };
            }
            Tab::Connections => {
                if self.connections.is_empty() {
                    return;
                }
                self.connections_selected = if self.connections_selected == 0 {
                    self.connections.len().saturating_sub(1)
                } else {
                    self.connections_selected - 1
                };
            }
            Tab::Subscriptions => {
                if self.profiles.is_empty() {
                    return;
                }
                self.selected_sub_idx = if self.selected_sub_idx == 0 {
                    self.profiles.len().saturating_sub(1)
                } else {
                    self.selected_sub_idx - 1
                };
            }
            Tab::Traffic => {
                if self.traffic_top.is_empty() {
                    return;
                }
                self.traffic_selected_idx = if self.traffic_selected_idx == 0 {
                    self.traffic_top.len().saturating_sub(1)
                } else {
                    self.traffic_selected_idx - 1
                };
            }
            _ => {}
        }
    }

    pub fn test_selected_delay(&mut self) {
        let api = self.api.clone();
        let tx = self.data_tx.clone();
        if self.tab == Tab::Proxies {
            let Some((name, _)) = self.selected_proxy_group() else {
                return;
            };
            self.rt.spawn(async move {
                if let Ok(delay) = api.test_delay(&name).await {
                    let _ = tx.send(DataEvent::Delay(name, delay));
                }
            });
        }
    }

    pub fn switch_selected(&mut self) {
        let api = self.api.clone();
        let tx = self.data_tx.clone();
        if self.tab == Tab::Proxies {
            let Some((group_name, _)) = self.selected_proxy_group() else {
                return;
            };
            if let Some(info) = self.proxies.get(&group_name) {
                if let Some(ref all) = info.all {
                    if all.is_empty() {
                        return;
                    }
                    let all = all.clone();
                    let current = info.now.clone().unwrap_or_default();
                    let api2 = api.clone();
                    let tx2 = tx.clone();
                    self.rt.spawn(async move {
                        let current_idx = all.iter().position(|node| node == &current).unwrap_or(0);
                        let target = &all[(current_idx + 1) % all.len()];
                        let result = api2.switch_proxy(&group_name, target).await;
                        let _ = tx2.send(DataEvent::SwitchResult(result));
                    });
                }
            }
        }
    }

    pub fn open_node_picker(&mut self) {
        if self.tab != Tab::Proxies {
            return;
        }
        let nodes = self.selected_proxy_nodes();
        if nodes.is_empty() {
            self.status_msg = Some("selected group has no nodes".into());
            return;
        }
        let current = self.selected_proxy_current_node();
        self.selected_node_idx = nodes.iter().position(|node| node == &current).unwrap_or(0);
        self.node_picker_open = true;
        self.status_msg = Some("node picker opened".into());
    }

    pub fn close_node_picker(&mut self) {
        if self.node_picker_open {
            self.node_picker_open = false;
            self.status_msg = Some("node picker closed".into());
        }
    }

    pub fn select_node_down(&mut self) {
        let len = self.selected_proxy_nodes().len();
        if len == 0 {
            self.selected_node_idx = 0;
            return;
        }
        self.selected_node_idx = (self.selected_node_idx + 1) % len;
    }

    pub fn select_node_up(&mut self) {
        let len = self.selected_proxy_nodes().len();
        if len == 0 {
            self.selected_node_idx = 0;
            return;
        }
        self.selected_node_idx = if self.selected_node_idx == 0 {
            len - 1
        } else {
            self.selected_node_idx - 1
        };
    }

    pub fn switch_selected_node(&mut self) {
        let Some(group_name) = self.selected_proxy_group_name() else {
            return;
        };
        let nodes = self.selected_proxy_nodes();
        if nodes.is_empty() {
            return;
        }
        let target = nodes[self.selected_node_idx.min(nodes.len() - 1)].clone();
        let api = self.api.clone();
        let tx = self.data_tx.clone();
        self.rt.spawn(async move {
            let result = api.switch_proxy(&group_name, &target).await;
            let _ = tx.send(DataEvent::SwitchResult(result));
        });
        self.node_picker_open = false;
    }

    pub fn close_selected_connection(&mut self) {
        if self.tab != Tab::Connections || self.connections.is_empty() {
            return;
        }
        let api = self.api.clone();
        let conn_id = self.connections[self.connections_selected].id.clone();
        self.rt.spawn(async move {
            let _ = api.close_connection(&conn_id).await;
        });
    }

    pub fn close_all_connections(&mut self) {
        if self.tab != Tab::Connections {
            return;
        }
        let api = self.api.clone();
        self.rt.spawn(async move {
            let _ = api.close_all_connections().await;
        });
    }

    pub fn on_shutdown(&mut self) {
        self.window.save();
        if let Err(e) = self.ui_settings.save() {
            self.error_msg = Some(format!("save settings failed: {}", e));
        }
    }

    fn t(&self, msg: Msg) -> &'static str {
        tr(self.ui_settings.language, msg)
    }

    fn action_label(&self, spec: &action_registry::ActionSpec) -> &'static str {
        tr_action_label(self.ui_settings.language, spec.id, spec.label)
    }

    fn action_desc(&self, spec: &action_registry::ActionSpec) -> &'static str {
        tr_action_desc(self.ui_settings.language, spec.id, spec.description)
    }

    fn action_button(&self, spec: &action_registry::ActionSpec) -> &'static str {
        tr_action_button(self.ui_settings.language, spec.id, spec.mouse)
    }

    fn settings_prompt_label(&self, kind: SettingsPromptKind) -> &'static str {
        action_registry::action_by_id(kind.action_id())
            .map(|spec| self.action_label(spec))
            .unwrap_or_else(|| kind.label())
    }

    fn settings_prompt_hint(&self, kind: SettingsPromptKind) -> &'static str {
        action_registry::action_by_id(kind.action_id())
            .map(|spec| self.action_desc(spec))
            .unwrap_or_else(|| kind.hint())
    }

    fn settings_prompt_field_label(&self, field: SettingsPromptFieldMsg) -> &'static str {
        tr_settings_prompt_field_label(self.ui_settings.language, field)
    }

    fn settings_prompt_field_hint(&self, field: SettingsPromptFieldMsg) -> &'static str {
        tr_settings_prompt_field_hint(self.ui_settings.language, field)
    }

    fn subscription_add_field_label(&self, field: SubscriptionAddField) -> &'static str {
        tr_subscription_field_label(self.ui_settings.language, field.msg())
    }

    fn subscription_add_field_hint(&self, field: SubscriptionAddField) -> &'static str {
        tr_subscription_field_hint(self.ui_settings.language, field.msg())
    }

    fn subscription_edit_field_label(&self, field: SubscriptionEditField) -> &'static str {
        tr_subscription_field_label(self.ui_settings.language, field.msg())
    }

    fn subscription_edit_field_hint(&self, field: SubscriptionEditField) -> &'static str {
        tr_subscription_field_hint(self.ui_settings.language, field.msg())
    }

    fn action_danger_label(&self, danger: ActionDanger) -> &'static str {
        match danger {
            ActionDanger::Safe => self.t(Msg::CommonSafe),
            ActionDanger::Confirm => self.t(Msg::RiskConfirm),
            ActionDanger::Sensitive => self.t(Msg::RiskSensitive),
            ActionDanger::Dangerous => self.t(Msg::RiskDangerous),
        }
    }

    pub fn toggle_language(&mut self) {
        self.ui_settings.language = self.ui_settings.language.next();
        self.save_ui_settings(format!("language: {}", self.ui_settings.language.label()));
    }

    pub fn cycle_default_page(&mut self) {
        self.ui_settings.default_page = next_default_page(&self.ui_settings.default_page);
        self.save_ui_settings(format!("default page: {}", self.default_page_label()));
    }

    pub fn cycle_theme_preference(&mut self) {
        self.ui_settings.theme = next_theme_key(&self.ui_settings.theme).into();
        apply_theme_key(&self.ui_settings.theme);
        self.save_ui_settings(format!("theme: {}", theme_label(&self.ui_settings.theme)));
    }

    pub fn cycle_refresh_interval(&mut self) {
        self.ui_settings.refresh_interval_secs =
            next_refresh_interval_secs(self.ui_settings.refresh_interval_secs);
        self.save_ui_settings(format!(
            "refresh interval: {}",
            self.ui_settings.refresh_interval_label()
        ));
    }

    pub fn cycle_default_traffic_range(&mut self) {
        self.traffic_range =
            TrafficRange::from_setting_key(&self.ui_settings.traffic_default_range).next();
        self.ui_settings.traffic_default_range = self.traffic_range.setting_key().into();
        self.reset_traffic_view_window();
        self.save_ui_settings(format!(
            "default traffic range: {}",
            self.traffic_range.label()
        ));
    }

    pub fn cycle_default_traffic_chart(&mut self) {
        self.traffic_chart =
            TrafficChartKind::from_setting_key(&self.ui_settings.traffic_default_chart).next();
        self.ui_settings.traffic_default_chart = self.traffic_chart.setting_key().into();
        self.save_ui_settings(format!(
            "default traffic chart: {}",
            self.traffic_chart.label()
        ));
    }

    pub fn cycle_default_traffic_dimension(&mut self) {
        self.traffic_dimension =
            TrafficDimension::from_setting_key(&self.ui_settings.traffic_default_dimension).next();
        self.ui_settings.traffic_default_dimension = self.traffic_dimension.setting_key().into();
        self.traffic_filter_key = None;
        self.traffic_selected_idx = 0;
        self.reset_traffic_view_window();
        self.save_ui_settings(format!(
            "default traffic dimension: {}",
            self.traffic_dimension.label()
        ));
    }

    pub fn toggle_mouse_preference(&mut self) {
        self.ui_settings.mouse_enabled = !self.ui_settings.mouse_enabled;
        let state = if self.ui_settings.mouse_enabled {
            "enabled"
        } else {
            "disabled"
        };
        self.save_ui_settings(format!("mouse capture: {state} after restart"));
    }

    pub fn toggle_dangerous_confirmations(&mut self) {
        if self.ui_settings.confirm_dangerous_actions {
            self.pending_confirmation = Some(PendingConfirmation {
                title: "Confirm settings change".into(),
                message: "Disable confirmation prompts for dangerous actions?".into(),
                action: PendingAction::ToggleDangerousConfirmations,
            });
            self.status_msg = Some("confirm action with Enter/y, cancel with Esc/n".into());
            return;
        }
        self.apply_dangerous_confirmations_toggle();
    }

    fn apply_dangerous_confirmations_toggle(&mut self) {
        self.ui_settings.confirm_dangerous_actions = !self.ui_settings.confirm_dangerous_actions;
        let state = if self.ui_settings.confirm_dangerous_actions {
            "on"
        } else {
            "off"
        };
        self.save_ui_settings(format!("dangerous confirmations: {state}"));
    }

    fn save_ui_settings(&mut self, success_msg: String) {
        match self.ui_settings.save() {
            Ok(()) => {
                self.error_msg = None;
                self.status_msg = Some(success_msg);
            }
            Err(e) => {
                self.error_msg = Some(format!("save settings failed: {}", e));
            }
        }
    }

    fn default_page_label(&self) -> String {
        if is_auto_default_page(&self.ui_settings.default_page) {
            return "auto".into();
        }
        Tab::from_setting_key(&self.ui_settings.default_page)
            .map(|tab| self.t(tab.msg()).to_string())
            .unwrap_or_else(|| self.ui_settings.default_page.clone())
    }

    pub fn command_palette_active(&self) -> bool {
        self.command_palette_open
    }

    pub fn open_command_palette(&mut self) {
        if self.pending_confirmation.is_some()
            || self.settings_prompt.is_some()
            || self.subscription_input_active()
        {
            self.status_msg = Some("close the current dialog before opening commands".into());
            return;
        }
        self.command_palette_open = true;
        self.command_query.clear();
        self.command_selected_idx = 0;
        self.search_active = false;
        self.show_help = false;
        self.error_msg = None;
    }

    pub fn close_command_palette(&mut self) {
        self.command_palette_open = false;
        self.command_query.clear();
        self.command_selected_idx = 0;
    }

    pub fn push_command_palette_char(&mut self, c: char) {
        self.command_query.push(c);
        self.command_selected_idx = 0;
    }

    pub fn pop_command_palette_char(&mut self) {
        self.command_query.pop();
        self.command_selected_idx = 0;
    }

    pub fn command_palette_select_down(&mut self) {
        let len = self.command_palette_matches().len();
        if len > 0 {
            self.command_selected_idx = (self.command_selected_idx + 1).min(len - 1);
        }
    }

    pub fn command_palette_select_up(&mut self) {
        self.command_selected_idx = self.command_selected_idx.saturating_sub(1);
    }

    pub fn submit_command_palette(&mut self) {
        let matches = self.command_palette_matches();
        let Some(spec) = matches
            .get(
                self.command_selected_idx
                    .min(matches.len().saturating_sub(1)),
            )
            .copied()
        else {
            self.error_msg = Some("no matching command".into());
            return;
        };
        self.close_command_palette();
        self.execute_action_spec(spec);
    }

    fn command_palette_matches(&self) -> Vec<&'static action_registry::ActionSpec> {
        let query = self.command_query.trim().to_lowercase();
        action_registry::all_actions()
            .iter()
            .filter(|spec| {
                if query.is_empty() {
                    return true;
                }
                let page = self.t(spec.page.msg()).to_lowercase();
                let label = self.action_label(spec).to_lowercase();
                let desc = self.action_desc(spec).to_lowercase();
                let button = self.action_button(spec).to_lowercase();
                let haystack = format!(
                    "{} {} {} {} {} {} {} {} {}",
                    spec.id,
                    spec.label,
                    label,
                    button,
                    spec.shortcut,
                    page,
                    spec.description,
                    desc,
                    spec.executor.command()
                )
                .to_lowercase();
                haystack.contains(&query)
            })
            .collect()
    }

    fn execute_action_spec(&mut self, spec: &action_registry::ActionSpec) {
        if spec.id.starts_with("nav.") {
            self.tab = spec.page;
            self.show_help = false;
            self.node_picker_open = false;
            self.reset_selection();
            if self.tab == Tab::Subscriptions {
                self.refresh_subscriptions();
            }
            if self.tab == Tab::Traffic {
                self.refresh_traffic();
            }
            self.status_msg = Some(format!("page: {}", self.t(self.tab.msg())));
            return;
        }
        if self.tab != spec.page {
            self.tab = spec.page;
            self.show_help = false;
            self.node_picker_open = false;
            self.reset_selection();
            if self.tab == Tab::Subscriptions {
                self.refresh_subscriptions();
            }
            if self.tab == Tab::Traffic {
                self.refresh_traffic();
            }
        }
        if let Some(action) = hitbox_for_action_spec(spec) {
            self.dispatch_hitbox_action(action);
            return;
        }
        match spec.id {
            "sub.use" => self.use_selected_subscription(),
            "sub.update" => self.update_selected_subscription(),
            "sub.edit" => self.begin_subscription_profile_edit(),
            "sub.remove" => self.remove_selected_subscription(),
            "proxy.mode.cycle" => self.cycle_proxy_mode(),
            "proxy.node.switch" => self.switch_selected(),
            "proxy.delay.selected" => self.test_selected_delay(),
            "conn.close.selected" => self.close_selected_connection(),
            "conn.close.all" => self.close_all_connections(),
            "logs.pause" => self.toggle_log_pause(),
            "logs.filter" => self.cycle_log_level(),
            _ => {
                self.status_msg = Some(format!("command not implemented yet: {}", spec.id));
            }
        }
    }

    pub fn handle_mouse_event(&mut self, kind: crossterm::event::MouseEventKind, x: u16, y: u16) {
        let action = self.hitboxes.action_at(x, y);
        match kind {
            crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                self.handle_mouse_click(action)
            }
            crossterm::event::MouseEventKind::ScrollDown => self.handle_mouse_scroll(action, 3),
            crossterm::event::MouseEventKind::ScrollUp => self.handle_mouse_scroll(action, -3),
            _ => {}
        }
    }

    fn handle_mouse_click(&mut self, action: Option<HitboxAction>) {
        if self.pending_confirmation.is_some()
            && !matches!(
                action,
                Some(HitboxAction::ConfirmPendingAction) | Some(HitboxAction::CancelPendingAction)
            )
        {
            return;
        }
        if self.settings_prompt.is_some()
            && !matches!(
                action,
                Some(HitboxAction::FocusSettingsPromptValue)
                    | Some(HitboxAction::SelectSettingsPromptField(_))
                    | Some(HitboxAction::SubmitSettingsPrompt)
                    | Some(HitboxAction::CancelSettingsPrompt)
            )
        {
            return;
        }
        if self.subscription_input_active()
            && !matches!(
                action,
                Some(HitboxAction::SubmitSubscriptionPrompt)
                    | Some(HitboxAction::CancelSubscriptionPrompt)
                    | Some(HitboxAction::SelectSubscriptionAddField(_))
            )
        {
            return;
        }
        if self.node_picker_open
            && !matches!(
                action,
                Some(HitboxAction::SelectProxyNode(_))
                    | Some(HitboxAction::SwitchSelectedProxyNode)
                    | Some(HitboxAction::TestSelectedProxyDelay)
                    | Some(HitboxAction::CloseNodePicker)
            )
        {
            return;
        }
        if let Some(action) = action {
            self.dispatch_hitbox_action(action);
        }
    }

    fn dispatch_hitbox_action(&mut self, action: HitboxAction) {
        match action {
            HitboxAction::SwitchTab(tab) => {
                self.tab = tab;
                self.show_help = false;
                self.node_picker_open = false;
                self.reset_selection();
                if self.tab == Tab::Subscriptions {
                    self.refresh_subscriptions();
                }
                if self.tab == Tab::Traffic {
                    self.refresh_traffic();
                }
            }
            HitboxAction::RunNetwork(action) => self.run_network_action(action),
            HitboxAction::RunSettings(action) => self.run_settings_action(action),
            HitboxAction::BeginSecretSet => self.begin_secret_set(),
            HitboxAction::BeginConfigSetPorts => {
                self.begin_settings_prompt(SettingsPromptKind::ConfigSetPorts);
            }
            HitboxAction::BeginConfigSetApi => {
                self.begin_settings_prompt(SettingsPromptKind::ConfigSetApi);
            }
            HitboxAction::BeginConfigSetDns => {
                self.begin_settings_prompt(SettingsPromptKind::ConfigSetDnsMode);
            }
            HitboxAction::BeginConfigSetLan => {
                self.begin_settings_prompt(SettingsPromptKind::ConfigSetLan);
            }
            HitboxAction::BeginTrafficPruneRetention => {
                self.begin_settings_prompt(SettingsPromptKind::TrafficPruneRetention);
            }
            HitboxAction::BeginGeodataUpdateVersion => {
                self.begin_settings_prompt(SettingsPromptKind::GeodataUpdateVersion);
            }
            HitboxAction::RunTraffic(action) => self.run_traffic_action(action),
            HitboxAction::SelectProxy(idx) => {
                self.tab = Tab::Proxies;
                self.selected_proxy_idx =
                    idx.min(self.visible_proxy_groups().len().saturating_sub(1));
                self.clamp_proxy_selection();
            }
            HitboxAction::SelectProxyNode(idx) => {
                self.selected_node_idx =
                    idx.min(self.selected_proxy_nodes().len().saturating_sub(1));
                self.clamp_node_selection();
            }
            HitboxAction::SelectSubscription(idx) => {
                self.selected_sub_idx = idx.min(self.profiles.len().saturating_sub(1));
                self.clamp_subscription_selection();
            }
            HitboxAction::BeginSubscriptionAdd => self.begin_subscription_add(),
            HitboxAction::BeginSubscriptionImport => self.begin_subscription_import(),
            HitboxAction::UseSubscription => self.use_selected_subscription(),
            HitboxAction::UpdateSubscription => self.update_selected_subscription(),
            HitboxAction::RemoveSubscription => self.remove_selected_subscription(),
            HitboxAction::ShowSubscriptionLog => self.show_subscription_log(),
            HitboxAction::EditSubscriptionName => {
                self.begin_subscription_profile_edit();
            }
            HitboxAction::EditSubscriptionInterval => {
                self.begin_subscription_edit(SubscriptionEditField::Interval);
            }
            HitboxAction::EditSubscriptionUrl => {
                self.begin_subscription_edit(SubscriptionEditField::Url);
            }
            HitboxAction::EditSubscriptionUserAgent => {
                self.begin_subscription_edit(SubscriptionEditField::UserAgent);
            }
            HitboxAction::EditSubscriptionUpdateProxy => {
                self.begin_subscription_edit(SubscriptionEditField::UpdateProxy);
            }
            HitboxAction::EditSubscriptionConvertMode => {
                self.begin_subscription_edit(SubscriptionEditField::ConvertMode);
            }
            HitboxAction::EditSubscriptionAddTag => {
                self.begin_subscription_edit(SubscriptionEditField::AddTag);
            }
            HitboxAction::EditSubscriptionRemoveTag => {
                self.begin_subscription_edit(SubscriptionEditField::RemoveTag);
            }
            HitboxAction::SelectSubscriptionAddField(idx) => {
                self.select_subscription_add_field(idx);
            }
            HitboxAction::SelectConnection(idx) => {
                self.connections_selected = idx.min(self.connections.len().saturating_sub(1));
            }
            HitboxAction::SelectTrafficRow(idx) => {
                self.traffic_selected_idx = idx.min(self.traffic_top.len().saturating_sub(1));
                self.clamp_traffic_selection();
            }
            HitboxAction::SelectTrafficBucket(idx) => self.lock_traffic_bucket(idx),
            HitboxAction::ScrollTrafficChart => self.clear_traffic_bucket_lock(),
            HitboxAction::NextTrafficRange => self.next_traffic_range(),
            HitboxAction::ToggleTrafficChart => self.toggle_traffic_chart(),
            HitboxAction::NextTrafficDimension => self.next_traffic_dimension(),
            HitboxAction::ExportTraffic => self.export_traffic_csv(),
            HitboxAction::SubmitSubscriptionPrompt => self.submit_subscription_prompt(),
            HitboxAction::CancelSubscriptionPrompt => self.cancel_subscription_prompt(),
            HitboxAction::ConfirmPendingAction => self.confirm_pending_action(),
            HitboxAction::CancelPendingAction => self.cancel_pending_action(),
            HitboxAction::FocusSettingsPromptValue => self.focus_settings_prompt_value(),
            HitboxAction::SelectSettingsPromptField(idx) => self.select_settings_prompt_field(idx),
            HitboxAction::SubmitSettingsPrompt => self.submit_settings_prompt(),
            HitboxAction::CancelSettingsPrompt => self.cancel_settings_prompt(),
            HitboxAction::CycleUiLanguage => self.toggle_language(),
            HitboxAction::CycleThemePreference => self.cycle_theme_preference(),
            HitboxAction::CycleDefaultPage => self.cycle_default_page(),
            HitboxAction::CycleRefreshInterval => self.cycle_refresh_interval(),
            HitboxAction::ToggleMousePreference => self.toggle_mouse_preference(),
            HitboxAction::ToggleDangerousConfirmations => self.toggle_dangerous_confirmations(),
            HitboxAction::CycleDefaultTrafficRange => self.cycle_default_traffic_range(),
            HitboxAction::CycleDefaultTrafficChart => self.cycle_default_traffic_chart(),
            HitboxAction::CycleDefaultTrafficDimension => self.cycle_default_traffic_dimension(),
            HitboxAction::CycleProxyMode => self.cycle_proxy_mode(),
            HitboxAction::ToggleProxySort => self.toggle_sort(),
            HitboxAction::ToggleNodePicker => {
                if self.node_picker_open {
                    self.close_node_picker();
                } else {
                    self.open_node_picker();
                }
            }
            HitboxAction::CloseNodePicker => self.close_node_picker(),
            HitboxAction::SwitchSelectedProxyNode => {
                if self.node_picker_open {
                    self.switch_selected_node();
                } else {
                    self.switch_selected();
                }
            }
            HitboxAction::TestSelectedProxyDelay => self.test_selected_delay(),
            HitboxAction::TestAllProxyDelays => self.test_all_delays(),
            HitboxAction::CloseSelectedConnection => self.close_selected_connection(),
            HitboxAction::CloseAllConnections => self.close_all_connections(),
            HitboxAction::ToggleLogPause => self.toggle_log_pause(),
            HitboxAction::CycleLogFilter => self.cycle_log_level(),
            HitboxAction::ClearLogs => self.clear_logs(),
            HitboxAction::ScrollLogs
            | HitboxAction::ScrollProxyNodes
            | HitboxAction::ScrollTrafficRows => {}
        }
    }

    fn handle_mouse_scroll(&mut self, action: Option<HitboxAction>, amount: i32) {
        match action {
            Some(HitboxAction::ScrollLogs) => {
                if amount > 0 {
                    self.scroll_logs_down(amount as usize);
                } else {
                    self.scroll_logs_up(amount.unsigned_abs() as usize);
                }
            }
            Some(HitboxAction::ScrollTrafficRows) | Some(HitboxAction::SelectTrafficRow(_)) => {
                if amount > 0 {
                    for _ in 0..amount {
                        self.select_down();
                    }
                } else {
                    for _ in 0..amount.unsigned_abs() {
                        self.select_up();
                    }
                }
            }
            Some(HitboxAction::ScrollTrafficChart) | Some(HitboxAction::SelectTrafficBucket(_)) => {
                self.pan_traffic_window(amount);
            }
            Some(HitboxAction::ScrollProxyNodes) | Some(HitboxAction::SelectProxyNode(_)) => {
                if amount > 0 {
                    for _ in 0..amount {
                        self.select_node_down();
                    }
                } else {
                    for _ in 0..amount.unsigned_abs() {
                        self.select_node_up();
                    }
                }
            }
            _ => {}
        }
    }

    fn lock_traffic_bucket(&mut self, idx: usize) {
        if self.traffic_points.is_empty() {
            return;
        }
        let idx = idx.min(self.traffic_points.len().saturating_sub(1));
        self.traffic_locked_bucket = Some(idx);
        if let Some(point) = self.traffic_points.get(idx) {
            self.status_msg = Some(format!(
                "traffic bucket locked: {}",
                point.ts.format("%H:%M")
            ));
        }
    }

    fn clear_traffic_bucket_lock(&mut self) {
        if self.traffic_locked_bucket.take().is_some() {
            self.status_msg = Some("traffic bucket lock cleared".into());
        }
    }

    fn pan_traffic_window(&mut self, amount: i32) {
        if self.traffic_points.is_empty() {
            self.traffic_window_offset = 0;
            return;
        }
        if amount > 0 {
            self.traffic_window_offset = self
                .traffic_window_offset
                .saturating_add(amount as usize)
                .min(self.traffic_points.len().saturating_sub(1));
        } else {
            self.traffic_window_offset = self
                .traffic_window_offset
                .saturating_sub(amount.unsigned_abs() as usize);
        }
        self.status_msg = Some(if self.traffic_window_offset == 0 {
            "traffic window: latest".into()
        } else {
            format!("traffic window: -{} buckets", self.traffic_window_offset)
        });
    }

    fn reset_traffic_view_window(&mut self) {
        self.traffic_window_offset = 0;
        self.traffic_locked_bucket = None;
    }

    pub fn refresh_subscriptions(&mut self) {
        let meta = crate::api::read_profiles();
        self.profiles = meta.profiles;
        self.active_profile_id = meta.active_id;
        self.clamp_subscription_selection();
    }

    pub fn refresh_traffic(&mut self) {
        self.fetch_traffic();
    }

    pub fn run_network_action(&mut self, action: NetworkAction) {
        if self.tab != Tab::Network {
            return;
        }
        if self.should_confirm() && action.requires_confirmation() {
            self.pending_confirmation = Some(PendingConfirmation {
                title: format!("Confirm {}", action.label()),
                message: format!("Run `clashctl {}`?", action.args().join(" ")),
                action: PendingAction::Network(action),
            });
            self.status_msg = Some("confirm action with Enter/y, cancel with Esc/n".into());
            return;
        }
        self.execute_network_action(action);
    }

    fn execute_network_action(&mut self, action: NetworkAction) {
        let label = action.label().to_string();
        let args = action.args();
        let tx = self.data_tx.clone();
        self.status_msg = Some(format!("running network action: {}", label));
        self.rt.spawn(async move {
            let result = crate::api::run_clashctl(&args).await;
            let _ = tx.send(DataEvent::NetworkResult(label, result));
        });
    }

    pub fn run_settings_action(&mut self, action: SettingsAction) {
        if self.tab != Tab::Settings {
            return;
        }
        if self.should_confirm() && action.requires_confirmation() {
            self.pending_confirmation = Some(PendingConfirmation {
                title: format!("Confirm {}", action.label()),
                message: format!("Run `clashctl {}`?", action.args().join(" ")),
                action: PendingAction::Settings(action),
            });
            self.status_msg = Some("confirm action with Enter/y, cancel with Esc/n".into());
            return;
        }
        self.execute_settings_action(action);
    }

    pub fn begin_settings_prompt(&mut self, kind: SettingsPromptKind) {
        if self.tab != Tab::Settings {
            return;
        }
        self.settings_prompt = Some(SettingsPrompt::new(kind));
        self.error_msg = None;
        self.status_msg = Some(format!("editing {}", kind.label()));
    }

    pub fn begin_secret_set(&mut self) {
        self.begin_settings_prompt(SettingsPromptKind::SecretSet);
    }

    pub fn cancel_settings_prompt(&mut self) {
        if self.settings_prompt.take().is_some() {
            self.status_msg = Some("settings edit cancelled".into());
        }
    }

    pub fn focus_settings_prompt_value(&mut self) {
        if let Some(prompt) = self.settings_prompt.as_ref() {
            self.error_msg = None;
            let field = prompt
                .active_field()
                .map(|field| self.settings_prompt_field_label(field.field))
                .unwrap_or("value");
            self.status_msg = Some(format!(
                "editing {} · {}",
                self.settings_prompt_label(prompt.kind),
                field
            ));
        }
    }

    pub fn next_settings_prompt_field(&mut self) {
        if let Some(prompt) = self.settings_prompt.as_mut() {
            prompt.next_field();
            self.focus_settings_prompt_value();
        }
    }

    pub fn prev_settings_prompt_field(&mut self) {
        if let Some(prompt) = self.settings_prompt.as_mut() {
            prompt.prev_field();
            self.focus_settings_prompt_value();
        }
    }

    pub fn select_settings_prompt_field(&mut self, idx: usize) {
        if let Some(prompt) = self.settings_prompt.as_mut() {
            prompt.select_field(idx);
            self.focus_settings_prompt_value();
        }
    }

    pub fn push_settings_prompt_char(&mut self, c: char) {
        if let Some(prompt) = self.settings_prompt.as_mut() {
            prompt.push_char(c);
        }
    }

    pub fn pop_settings_prompt_char(&mut self) {
        if let Some(prompt) = self.settings_prompt.as_mut() {
            prompt.pop_char();
        }
    }

    pub fn submit_settings_prompt(&mut self) {
        let Some(prompt) = self.settings_prompt.take() else {
            return;
        };
        let value = prompt.value().trim().to_string();
        if value.is_empty() {
            self.error_msg = Some(format!(
                "{} cannot be empty",
                self.settings_prompt_label(prompt.kind)
            ));
            self.settings_prompt = Some(prompt);
            return;
        }

        let label = prompt.kind.label().to_string();
        let args = match prompt.kind.args(value) {
            Ok(args) => args,
            Err(e) => {
                self.error_msg = Some(e);
                self.settings_prompt = Some(prompt);
                return;
            }
        };
        if self.should_confirm() {
            let preview = prompt.kind.command_preview(&args);
            self.pending_confirmation = Some(PendingConfirmation {
                title: format!("Confirm {}", self.settings_prompt_label(prompt.kind)),
                message: format!("Run `{}`?", preview),
                action: PendingAction::SettingsCommand {
                    label,
                    args,
                    redact_output: true,
                },
            });
            self.status_msg = Some("confirm action with Enter/y, cancel with Esc/n".into());
            return;
        }
        self.execute_settings_command(label, args, true);
    }

    fn execute_settings_action(&mut self, action: SettingsAction) {
        let args = action.args();
        let tx = self.data_tx.clone();
        self.status_msg = Some(format!("running settings action: {}", action.label()));
        self.rt.spawn(async move {
            let result = crate::api::run_clashctl(&args).await;
            let _ = tx.send(DataEvent::SettingsResult(action, result));
        });
    }

    fn execute_settings_command(&mut self, label: String, args: Vec<String>, redact_output: bool) {
        let tx = self.data_tx.clone();
        self.status_msg = Some(format!("running settings action: {}", label));
        self.rt.spawn(async move {
            let result = crate::api::run_clashctl(&args).await;
            let _ = tx.send(DataEvent::SettingsCommandResult(
                label,
                redact_output,
                result,
            ));
        });
    }

    fn should_confirm(&self) -> bool {
        self.ui_settings.confirm_dangerous_actions
    }

    pub fn confirm_pending_action(&mut self) {
        let Some(pending) = self.pending_confirmation.take() else {
            return;
        };
        match pending.action {
            PendingAction::Network(action) => self.execute_network_action(action),
            PendingAction::Settings(action) => self.execute_settings_action(action),
            PendingAction::SettingsCommand {
                label,
                args,
                redact_output,
            } => self.execute_settings_command(label, args, redact_output),
            PendingAction::ToggleDangerousConfirmations => {
                self.apply_dangerous_confirmations_toggle();
            }
            PendingAction::SubscriptionRemove(id) => self.execute_subscription_remove(id),
            PendingAction::Traffic(action) => self.execute_traffic_action(action),
        }
    }

    pub fn cancel_pending_action(&mut self) {
        if self.pending_confirmation.take().is_some() {
            self.status_msg = Some("action cancelled".into());
        }
    }

    pub fn next_traffic_range(&mut self) {
        if self.tab != Tab::Traffic {
            return;
        }
        self.traffic_range = self.traffic_range.next();
        self.traffic_filter_key = None;
        self.reset_traffic_view_window();
        self.status_msg = Some(format!("traffic range: {}", self.traffic_range.label()));
        self.fetch_traffic();
    }

    pub fn prev_traffic_range(&mut self) {
        if self.tab != Tab::Traffic {
            return;
        }
        self.traffic_range = self.traffic_range.prev();
        self.traffic_filter_key = None;
        self.reset_traffic_view_window();
        self.status_msg = Some(format!("traffic range: {}", self.traffic_range.label()));
        self.fetch_traffic();
    }

    pub fn toggle_traffic_chart(&mut self) {
        if self.tab != Tab::Traffic {
            return;
        }
        self.traffic_chart = self.traffic_chart.next();
        self.status_msg = Some(format!("traffic chart: {}", self.traffic_chart.label()));
    }

    pub fn next_traffic_dimension(&mut self) {
        if self.tab != Tab::Traffic {
            return;
        }
        self.traffic_dimension = self.traffic_dimension.next();
        self.traffic_filter_key = None;
        self.traffic_selected_idx = 0;
        self.reset_traffic_view_window();
        self.status_msg = Some(format!(
            "traffic dimension: {}",
            self.traffic_dimension.label()
        ));
        self.fetch_traffic();
    }

    pub fn toggle_traffic_filter(&mut self) {
        if self.tab != Tab::Traffic {
            return;
        }
        if self.traffic_filter_key.is_some() {
            self.traffic_filter_key = None;
            self.reset_traffic_view_window();
            self.status_msg = Some("traffic filter cleared".into());
            self.fetch_traffic();
            return;
        }
        let Some(row) = self.traffic_top.get(self.traffic_selected_idx) else {
            return;
        };
        let key = row.key.clone();
        self.traffic_filter_key = Some(key.clone());
        self.reset_traffic_view_window();
        self.status_msg = Some(format!("traffic filter: {}", trunc_str(&key, 48)));
        self.fetch_traffic();
    }

    pub fn export_traffic_csv(&mut self) {
        if self.tab != Tab::Traffic {
            return;
        }
        let range = self.traffic_range.range_arg().to_string();
        let by = self.traffic_dimension.arg().to_string();
        let tx = self.data_tx.clone();
        self.status_msg = Some(format!(
            "exporting traffic: {} / {}",
            self.traffic_range.label(),
            self.traffic_dimension.label()
        ));
        self.rt.spawn(async move {
            let result = crate::api::export_traffic_csv(&range, &by).await;
            let _ = tx.send(DataEvent::TrafficExport(result));
        });
    }

    pub fn run_traffic_action(&mut self, action: TrafficAction) {
        if self.tab != Tab::Traffic {
            return;
        }
        if self.should_confirm() && action.requires_confirmation() {
            self.pending_confirmation = Some(PendingConfirmation {
                title: format!("Confirm {}", action.label()),
                message: format!("Run `clashctl {}`?", action.args().join(" ")),
                action: PendingAction::Traffic(action),
            });
            self.status_msg = Some("confirm action with Enter/y, cancel with Esc/n".into());
            return;
        }
        self.execute_traffic_action(action);
    }

    fn execute_traffic_action(&mut self, action: TrafficAction) {
        let label = action.label().to_string();
        let args = action.args();
        let tx = self.data_tx.clone();
        self.status_msg = Some(format!("running traffic action: {}", label));
        self.rt.spawn(async move {
            let result = crate::api::run_clashctl(&args).await;
            let _ = tx.send(DataEvent::TrafficActionResult(label, result));
        });
    }

    pub fn use_selected_subscription(&mut self) {
        let Some(id) = self.selected_subscription_id() else {
            return;
        };
        let tx = self.data_tx.clone();
        self.rt.spawn(async move {
            let result = crate::api::run_clashctl_sub("use", id).await;
            let _ = tx.send(DataEvent::SubscriptionResult(result));
        });
    }

    pub fn update_selected_subscription(&mut self) {
        let Some(id) = self.selected_subscription_id() else {
            return;
        };
        let tx = self.data_tx.clone();
        self.rt.spawn(async move {
            let result = crate::api::run_clashctl_sub("update", id).await;
            let _ = tx.send(DataEvent::SubscriptionResult(result));
        });
    }

    pub fn remove_selected_subscription(&mut self) {
        if self.tab != Tab::Subscriptions {
            return;
        }
        let Some(id) = self.selected_subscription_id() else {
            self.status_msg = Some("no subscription selected".into());
            return;
        };
        if self.should_confirm() {
            self.pending_confirmation = Some(PendingConfirmation {
                title: format!("Confirm remove subscription {}", id),
                message: format!("Run `clashctl sub remove {}`?", id),
                action: PendingAction::SubscriptionRemove(id),
            });
            self.status_msg = Some("confirm action with Enter/y, cancel with Esc/n".into());
            return;
        }
        self.execute_subscription_remove(id);
    }

    fn execute_subscription_remove(&mut self, id: i32) {
        let tx = self.data_tx.clone();
        self.status_msg = Some(format!("removing subscription {}", id));
        self.rt.spawn(async move {
            let args = vec!["sub".into(), "remove".into(), id.to_string()];
            let result = crate::api::run_clashctl(&args).await.map(|stdout| {
                if stdout.is_empty() {
                    format!("subscription removed: [{}]", id)
                } else {
                    stdout.lines().last().unwrap_or("").to_string()
                }
            });
            let _ = tx.send(DataEvent::SubscriptionResult(result));
        });
    }

    pub fn show_subscription_log(&mut self) {
        if self.tab != Tab::Subscriptions {
            return;
        }
        let tx = self.data_tx.clone();
        self.status_msg = Some("loading subscription log".into());
        self.rt.spawn(async move {
            let args = vec!["sub".into(), "log".into()];
            let result = crate::api::run_clashctl(&args).await;
            let _ = tx.send(DataEvent::SubscriptionOutputResult("log".into(), result));
        });
    }

    pub fn toggle_sort(&mut self) {
        self.sort_mode = !self.sort_mode;
        self.clamp_proxy_selection();
    }

    pub fn cycle_proxy_mode(&mut self) {
        let next = next_mode(&self.proxy_mode_str);
        let api_mode = next.to_lowercase();
        let api = self.api.clone();
        let tx = self.data_tx.clone();
        self.rt.spawn(async move {
            let result = api.set_mode(&api_mode).await.map(|_| next);
            let _ = tx.send(DataEvent::ModeResult(result));
        });
    }

    pub fn test_all_delays(&mut self) {
        let api = self.api.clone();
        let tx = self.data_tx.clone();
        for (name, _) in self.visible_proxy_groups() {
            let api = api.clone();
            let tx = tx.clone();
            self.rt.spawn(async move {
                if let Ok(delay) = api.test_delay(&name).await {
                    let _ = tx.send(DataEvent::Delay(name, delay));
                }
            });
        }
    }

    pub fn toggle_log_pause(&mut self) {
        self.log_paused = !self.log_paused;
        self.status_msg = Some(if self.log_paused {
            "logs paused".into()
        } else {
            "logs resumed".into()
        });
        if !self.log_paused && self.tab == Tab::Logs {
            self.fetch_logs();
        }
    }

    pub fn cycle_log_level(&mut self) {
        self.log_level = self.log_level.next();
        self.clamp_log_scroll();
        self.status_msg = Some(format!("log level: {}", self.log_level.label()));
        if self.tab == Tab::Logs && !self.log_paused {
            self.fetch_logs();
        }
    }

    pub fn clear_logs(&mut self) {
        self.logs.clear();
        self.log_scroll = 0;
        self.status_msg = Some("logs cleared".into());
    }

    pub fn scroll_logs_down(&mut self, amount: usize) {
        self.log_scroll = self.log_scroll.saturating_add(amount);
        self.clamp_log_scroll();
    }

    pub fn scroll_logs_up(&mut self, amount: usize) {
        self.log_scroll = self.log_scroll.saturating_sub(amount);
    }

    fn clamp_log_scroll(&mut self) {
        let len = self.visible_logs().len();
        if len == 0 {
            self.log_scroll = 0;
        } else if self.log_scroll >= len {
            self.log_scroll = len - 1;
        }
    }

    fn visible_logs(&self) -> Vec<&String> {
        let query = self.search_query.to_lowercase();
        self.logs
            .iter()
            .filter(|line| self.log_level.matches_line(line))
            .filter(|line| query.is_empty() || line.to_lowercase().contains(&query))
            .collect()
    }
}

pub fn render(frame: &mut Frame, app: &mut App) {
    app.hitboxes.clear();
    let term = frame.area();

    fill_area(frame, term, CLASH_THEME.bg_outer);
    let win = app.window.compute(term);
    app.background.update(term, win, app.tick_count);
    app.background.render(term, frame.buffer_mut());

    fill_area(frame, win, CLASH_THEME.bg);
    let win_border = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(CLASH_THEME.border).bg(CLASH_THEME.bg));
    frame.render_widget(win_border, win);

    let inner = win.inner(Margin::new(1, 0));
    let title = format!(" clash-tui · {} ", app.t(app.tab.msg()));
    let title_span = Span::styled(
        title.clone(),
        Style::default().fg(CLASH_THEME.primary).bold(),
    );
    let decor = "─".repeat(inner.width.saturating_sub(title.len() as u16) as usize);
    let title_line = Line::from(vec![
        title_span,
        Span::styled(decor, Style::default().fg(CLASH_THEME.muted)),
    ]);
    frame.render_widget(
        Paragraph::new(title_line).style(Style::default().bg(CLASH_THEME.bg)),
        Rect::new(win.x + 1, win.y, win.width.saturating_sub(2), 1),
    );

    let chunks = if inner.height >= 6 {
        Layout::vertical([
            Constraint::Length(2),
            Constraint::Min(4),
            Constraint::Length(1),
        ])
        .split(inner)
    } else {
        Layout::vertical([
            Constraint::Length(0),
            Constraint::Min(2),
            Constraint::Length(1),
        ])
        .split(inner)
    };

    fill_area(frame, chunks[0], CLASH_THEME.bg);
    crate::widgets::tab_bar::render_tab_bar(frame, chunks[0], app.tab, app.ui_settings.language);
    register_tab_hitboxes(chunks[0], app);

    let content_area = chunks[1];
    fill_area(frame, content_area, CLASH_THEME.bg);

    if app.show_help {
        render_help(frame, content_area, app);
    } else {
        match app.tab {
            Tab::Network => render_network(frame, content_area, app),
            Tab::Proxies => render_proxies(frame, content_area, app),
            Tab::Subscriptions => render_subscriptions(frame, content_area, app),
            Tab::Connections => render_connections(frame, content_area, app),
            Tab::Traffic => render_traffic(frame, content_area, app),
            Tab::Logs => render_logs(frame, content_area, app),
            Tab::Settings => render_settings(frame, content_area, app),
            Tab::Help => render_help(frame, content_area, app),
        }
        if app.node_picker_open {
            render_node_picker(frame, content_area, app);
        }
        if app.subscription_prompt.is_some() {
            render_subscription_prompt(frame, content_area, app);
        }
        if app.settings_prompt.is_some() {
            render_settings_prompt(frame, content_area, app);
        }
        if app.pending_confirmation.is_some() {
            render_confirmation_prompt(frame, content_area, app);
        }
        if app.command_palette_open {
            render_command_palette(frame, content_area, app);
        }
    }

    render_status_bar(frame, chunks[2], app);
}

fn render_network(frame: &mut Frame, area: Rect, app: &mut App) {
    if area.height < 12 {
        return;
    }

    let rows = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(6),
        Constraint::Length(5),
        Constraint::Min(5),
    ])
    .split(area);

    // Row 0: Status bar
    let status_dot = if !app.version.is_empty() {
        "●"
    } else {
        "○"
    };
    let status_color = if !app.version.is_empty() {
        CLASH_THEME.accent
    } else {
        CLASH_THEME.muted
    };
    let ver_display = if app.version.is_empty() {
        "Mihomo —".to_string()
    } else {
        format!("Mihomo {}", app.version)
    };
    let uptime = if !app.version.is_empty() {
        let secs = (chrono::Utc::now().timestamp() - app.start_time) as u64;
        if secs >= 3600 {
            format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
        } else {
            format!("{}m", secs / 60)
        }
    } else {
        "—".into()
    };
    let status_lines = vec![Line::from(vec![
        Span::styled(
            format!(" {} ", status_dot),
            Style::default().fg(status_color).bold(),
        ),
        Span::styled(
            format!("{}    {}  ", ver_display, app.mode),
            CLASH_THEME.text,
        ),
        Span::styled("TUN ", CLASH_THEME.muted),
        Span::styled(
            if app.tun_enabled {
                "Enabled"
            } else {
                "Disabled"
            },
            if app.tun_enabled {
                CLASH_THEME.accent
            } else {
                CLASH_THEME.muted
            },
        ),
        Span::styled(format!("    Uptime: {}", uptime), CLASH_THEME.text),
    ])];
    crate::widgets::card::Card::new("Kernel").render(frame, rows[0], status_lines);

    // Row 1: Traffic + Current Proxy cards
    let mid_top =
        Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(rows[1]);

    let traffic_lines = vec![
        Line::from(vec![Span::styled(
            format!(
                "  ↑ {}/s  ↓ {}/s",
                format_bytes(app.upload_rate as u64),
                format_bytes(app.download_rate as u64)
            ),
            CLASH_THEME.text,
        )]),
        Line::from(vec![Span::styled(
            format!(
                "  Total  ↑ {}  ↓ {}",
                format_bytes(app.upload_total),
                format_bytes(app.download_total)
            ),
            CLASH_THEME.muted,
        )]),
    ];
    crate::widgets::card::Card::new(app.t(Msg::PageTraffic)).render(
        frame,
        mid_top[0],
        traffic_lines,
    );

    let mut proxy_lines = vec![Line::from(Span::styled(
        format!("  {}", app.t(Msg::NetworkNoProxySelected)),
        CLASH_THEME.muted,
    ))];
    if let Some((name, current)) = app.selected_proxy_group() {
        let delay = app
            .delays
            .get(&name)
            .map(|d| format!("{}ms", d))
            .unwrap_or_else(|| "—".into());
        proxy_lines = vec![
            Line::from(vec![
                Span::styled(format!("  {} → ", name), CLASH_THEME.muted),
                Span::styled(current, CLASH_THEME.accent),
            ]),
            Line::from(vec![
                Span::styled("  Delay: ", CLASH_THEME.muted),
                Span::styled(delay, CLASH_THEME.text),
            ]),
        ];
    }
    crate::widgets::card::Card::new(app.t(Msg::NetworkCurrentProxy)).render(
        frame,
        mid_top[1],
        proxy_lines,
    );

    // Row 2: Network actions
    render_network_actions(frame, rows[2], app);

    // Row 3: Connections, system info, and last command output
    let mid_bot =
        Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(rows[3]);

    let left_rows = Layout::vertical([Constraint::Length(3), Constraint::Min(3)]).split(mid_bot[0]);
    let conn_lines = vec![Line::from(vec![Span::styled(
        format!(
            "  Active: {}   Total: {}",
            app.connections_active, app.connections_total
        ),
        CLASH_THEME.text,
    )])];
    crate::widgets::card::Card::new(app.t(Msg::PageConnections)).render(
        frame,
        left_rows[0],
        conn_lines,
    );

    let sys_lines = vec![Line::from(vec![
        Span::styled(format!("  OS: {}  ", app.os_info), CLASH_THEME.text),
        Span::styled(format!("Arch: {}", app.arch_info), CLASH_THEME.text),
    ])];
    crate::widgets::card::Card::new(app.t(Msg::NetworkSystemInfo)).render(
        frame,
        left_rows[1],
        sys_lines,
    );

    let output_lines = if app.network_output.is_empty() {
        vec![
            Line::from(format!("  {}", app.t(Msg::NetworkLastCommandOutput))),
            Line::from(format!("  {}", app.t(Msg::NetworkShellProxyPrinted))),
        ]
    } else {
        app.network_output
            .iter()
            .map(|line| Line::from(Span::styled(format!("  {}", line), CLASH_THEME.text)))
            .collect()
    };
    crate::widgets::card::Card::new(app.t(Msg::NetworkCommandOutput)).render(
        frame,
        mid_bot[1],
        output_lines,
    );
}

fn render_network_actions(frame: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLASH_THEME.border))
        .title(Span::styled(
            format!(" {} ", app.t(Msg::NetworkActions)),
            Style::default().fg(CLASH_THEME.primary).bold(),
        ))
        .title_bottom(Span::styled(
            format!(" {} ", app.t(Msg::NetworkActionsHint)),
            Style::default().fg(CLASH_THEME.muted),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut x = inner.x;
    let mut y = inner.y;
    for spec in action_registry::network_action_specs() {
        let ActionExecutor::Network(action) = spec.executor else {
            continue;
        };
        let label = format!("[{}]", app.action_button(spec));
        let width = label.chars().count() as u16 + 1;
        if x.saturating_add(width) > inner.x.saturating_add(inner.width) {
            x = inner.x;
            y = y.saturating_add(1);
        }
        if y >= inner.y.saturating_add(inner.height) {
            break;
        }
        let area = Rect::new(
            x,
            y,
            width.min(inner.x.saturating_add(inner.width).saturating_sub(x)),
            1,
        );
        app.hitboxes
            .register(area, HitboxAction::RunNetwork(action));
        let fg = match spec.danger {
            ActionDanger::Dangerous => CLASH_THEME.danger,
            ActionDanger::Confirm | ActionDanger::Sensitive => CLASH_THEME.warning,
            ActionDanger::Safe => CLASH_THEME.text,
        };
        frame.render_widget(
            Paragraph::new(label).style(Style::default().fg(fg).bg(CLASH_THEME.surface)),
            area,
        );
        x = x.saturating_add(width);
    }
}

fn action_specs_line<'a, I>(app: &App, specs: I) -> Line<'static>
where
    I: IntoIterator<Item = &'a action_registry::ActionSpec>,
{
    let mut spans = vec![Span::styled("  ", CLASH_THEME.text)];
    for spec in specs {
        spans.push(Span::styled(
            format!("[{}] ", app.action_button(spec)),
            Style::default().fg(action_fg(spec.danger)),
        ));
    }
    Line::from(spans)
}

fn register_action_specs_hitboxes<'a, I>(app: &mut App, area: Rect, specs: I)
where
    I: IntoIterator<Item = &'a action_registry::ActionSpec>,
{
    if area.width < 6 || area.height == 0 {
        return;
    }
    let y = area.y + area.height.saturating_sub(2);
    let mut x = area.x + 3;
    for spec in specs {
        let Some(action) = hitbox_for_action_spec(spec) else {
            continue;
        };
        let width = action_button_width(app.action_button(spec));
        register_clamped_hitbox(app, x, y, width, area, action);
        x = x.saturating_add(width + 1);
    }
}

fn register_tab_hitboxes(area: Rect, app: &mut App) {
    let mut x = area.x;
    for tab in Tab::all() {
        let width = (app.t(tab.msg()).chars().count() as u16).saturating_add(2);
        if x >= area.x.saturating_add(area.width) {
            break;
        }
        let available = area.x.saturating_add(area.width).saturating_sub(x);
        app.hitboxes.register(
            Rect::new(x, area.y, width.min(available), area.height.max(1)),
            HitboxAction::SwitchTab(*tab),
        );
        x = x.saturating_add(width);
    }
}

fn register_table_row_hitboxes<F>(app: &mut App, inner: Rect, count: usize, mut action: F)
where
    F: FnMut(usize) -> HitboxAction,
{
    if inner.height <= 1 || inner.width == 0 {
        return;
    }
    let visible = count.min(inner.height.saturating_sub(1) as usize);
    for idx in 0..visible {
        app.hitboxes.register(
            Rect::new(inner.x, inner.y + 1 + idx as u16, inner.width, 1),
            action(idx),
        );
    }
}

fn render_proxies(frame: &mut Frame, area: Rect, app: &mut App) {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let groups = app.visible_proxy_groups();
    for (name, _) in &groups {
        let Some(info) = app.proxies.get(name) else {
            continue;
        };
        let current = info.now.as_deref().unwrap_or("—");
        let delay = app
            .delays
            .get(name)
            .map(|d| format!("{}ms", d))
            .unwrap_or_else(|| "—".into());
        let all_count = info.all.as_ref().map(|a| a.len()).unwrap_or(0);
        rows.push(vec![
            name.clone(),
            current.to_string(),
            delay,
            format!("{} {}", all_count, app.t(Msg::ProxyNodesUnit)),
        ]);
    }

    fill_area(frame, area, CLASH_THEME.surface);

    let (table_area, actions_area) = if area.height >= 8 {
        let chunks = Layout::vertical([Constraint::Min(5), Constraint::Length(3)]).split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    if rows.is_empty() {
        let msg =
            Paragraph::new(app.t(Msg::ProxyNoGroups)).style(Style::default().fg(CLASH_THEME.muted));
        frame.render_widget(msg, table_area);
        if let Some(actions_area) = actions_area {
            render_proxy_actions(frame, actions_area, app);
        }
        return;
    }

    let header = Row::new(vec![
        app.t(Msg::ProxyGroup),
        app.t(Msg::ProxyCurrent),
        app.t(Msg::ProxyDelay),
        app.t(Msg::ProxyNodes),
    ])
    .style(Style::default().fg(CLASH_THEME.muted));
    let data_rows: Vec<Row> = rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let style = if i == app.selected_proxy_idx {
                Style::default()
                    .fg(CLASH_THEME.text)
                    .bg(CLASH_THEME.primary)
            } else {
                Style::default()
                    .fg(CLASH_THEME.text)
                    .bg(CLASH_THEME.surface)
            };
            Row::new(row.clone()).style(style)
        })
        .collect();

    let widths = [
        Constraint::Ratio(1, 4),
        Constraint::Ratio(1, 4),
        Constraint::Ratio(1, 4),
        Constraint::Ratio(1, 4),
    ];
    let table = Table::new(data_rows, widths)
        .header(header)
        .style(Style::default().bg(CLASH_THEME.surface));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLASH_THEME.border))
        .title(Span::styled(
            format!(" {} ", app.t(Msg::PageProxies)),
            Style::default().fg(CLASH_THEME.primary).bold(),
        ))
        .title_bottom(Span::styled(
            format!(" {} ", app.t(Msg::ProxyTableHint)),
            Style::default().fg(CLASH_THEME.muted),
        ));
    let inner = block.inner(table_area);
    frame.render_widget(block, table_area);
    register_table_row_hitboxes(app, inner, rows.len(), HitboxAction::SelectProxy);
    frame.render_stateful_widget(table, inner, &mut app.proxy_table_state.clone());

    if let Some(actions_area) = actions_area {
        render_proxy_actions(frame, actions_area, app);
    }
}

fn render_proxy_actions(frame: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLASH_THEME.border))
        .title(Span::styled(
            format!(" {} ", app.t(Msg::ProxyActions)),
            Style::default().fg(CLASH_THEME.primary).bold(),
        ))
        .title_bottom(Span::styled(
            format!(" {} ", app.t(Msg::ProxyActionsHint)),
            Style::default().fg(CLASH_THEME.muted),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(
        Paragraph::new(action_specs_line(
            app,
            action_registry::proxy_action_specs(),
        ))
        .style(Style::default().bg(CLASH_THEME.surface)),
        inner,
    );
    register_action_specs_hitboxes(app, area, action_registry::proxy_action_specs());
}

fn render_node_picker(frame: &mut Frame, area: Rect, app: &mut App) {
    let popup = centered_rect(area, 72, 76);
    fill_area(frame, popup, CLASH_THEME.bg);

    let group = app
        .selected_proxy_group_name()
        .unwrap_or_else(|| app.t(Msg::ProxyUnknownGroup).into());
    let current = app.selected_proxy_current_node();
    let nodes = app.selected_proxy_nodes();
    let has_actions = popup.height >= 8;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLASH_THEME.primary))
        .title(Span::styled(
            format!(" {} · {} ", app.t(Msg::ProxyNodes), group),
            Style::default().fg(CLASH_THEME.primary).bold(),
        ))
        .title_bottom(Span::styled(
            format!(" {} ", app.t(Msg::ProxyNodePickerHint)),
            Style::default().fg(CLASH_THEME.muted),
        ));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let (table_area, action_area) = if has_actions {
        let chunks = Layout::vertical([Constraint::Min(3), Constraint::Length(3)]).split(inner);
        (chunks[0], Some(chunks[1]))
    } else {
        (inner, None)
    };
    let visible_capacity = table_area.height.saturating_sub(1) as usize;
    let start = if visible_capacity == 0 {
        0
    } else {
        app.selected_node_idx
            .saturating_add(1)
            .saturating_sub(visible_capacity)
    };

    let rows: Vec<Row> = nodes
        .iter()
        .enumerate()
        .skip(start)
        .take(visible_capacity)
        .map(|(idx, node)| {
            let marker = if node == &current { "●" } else { " " };
            let delay = app
                .delays
                .get(node)
                .map(|d| format!("{}ms", d))
                .unwrap_or_else(|| "—".into());
            let style = if idx == app.selected_node_idx {
                Style::default()
                    .fg(CLASH_THEME.text)
                    .bg(CLASH_THEME.primary)
            } else if node == &current {
                Style::default().fg(CLASH_THEME.accent).bg(CLASH_THEME.bg)
            } else {
                Style::default().fg(CLASH_THEME.text).bg(CLASH_THEME.bg)
            };
            Row::new(vec![marker.to_string(), node.clone(), delay]).style(style)
        })
        .collect();

    let header = Row::new(vec!["", app.t(Msg::ProxyNode), app.t(Msg::ProxyDelay)])
        .style(Style::default().fg(CLASH_THEME.muted));
    let widths = [
        Constraint::Length(2),
        Constraint::Ratio(1, 1),
        Constraint::Length(10),
    ];
    let table = Table::new(rows, widths)
        .header(header)
        .style(Style::default().bg(CLASH_THEME.bg));
    if nodes.is_empty() {
        frame.render_widget(
            Paragraph::new(app.t(Msg::ProxyNoNodes))
                .style(Style::default().fg(CLASH_THEME.muted).bg(CLASH_THEME.bg)),
            table_area,
        );
    } else {
        app.hitboxes
            .register(table_area, HitboxAction::ScrollProxyNodes);
        let row_count = visible_capacity.min(nodes.len().saturating_sub(start));
        for row_idx in 0..row_count {
            app.hitboxes.register(
                Rect::new(
                    table_area.x,
                    table_area.y + 1 + row_idx as u16,
                    table_area.width,
                    1,
                ),
                HitboxAction::SelectProxyNode(start + row_idx),
            );
        }
        frame.render_widget(table, table_area);
    }
    if let Some(action_area) = action_area {
        frame.render_widget(
            Paragraph::new(action_specs_line(
                app,
                action_registry::node_picker_action_specs(),
            ))
            .style(Style::default().bg(CLASH_THEME.bg)),
            action_area,
        );
        register_action_specs_hitboxes(
            app,
            action_area,
            action_registry::node_picker_action_specs(),
        );
    }
}

fn centered_rect(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(area);
    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(vertical[1])[1]
}

fn render_subscriptions(frame: &mut Frame, area: Rect, app: &mut App) {
    fill_area(frame, area, CLASH_THEME.surface);

    let (table_area, output_area) = if area.height >= 14 {
        let rows = Layout::vertical([Constraint::Min(8), Constraint::Length(5)]).split(area);
        (rows[0], Some(rows[1]))
    } else {
        (area, None)
    };

    let header = Row::new(vec![
        app.t(Msg::SubscriptionsId),
        app.t(Msg::SubscriptionsName),
        app.t(Msg::SubscriptionsInterval),
        app.t(Msg::SubscriptionsNext),
        app.t(Msg::SubscriptionsStatus),
        app.t(Msg::SubscriptionsUrl),
    ])
    .style(Style::default().fg(CLASH_THEME.muted));
    let data_rows: Vec<Row> = app
        .profiles
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let active_marker = if p.id == app.active_profile_id {
                "● "
            } else {
                "  "
            };
            let name = if p.name.is_empty() { &p.url } else { &p.name };
            let interval = profile_interval_label(p);
            let next = if p.next_update.is_empty() {
                "—".to_string()
            } else {
                p.next_update.chars().take(16).collect()
            };
            let status = if !p.last_error.is_empty() {
                app.t(Msg::SubscriptionsStatusError)
            } else if p.id == app.active_profile_id {
                app.t(Msg::SubscriptionsStatusActive)
            } else {
                app.t(Msg::SubscriptionsStatusReady)
            };
            let style = if i == app.selected_sub_idx {
                Style::default()
                    .fg(CLASH_THEME.text)
                    .bg(CLASH_THEME.primary)
            } else if p.id == app.active_profile_id {
                Style::default()
                    .fg(CLASH_THEME.accent)
                    .bg(CLASH_THEME.surface)
            } else {
                Style::default()
                    .fg(CLASH_THEME.text)
                    .bg(CLASH_THEME.surface)
            };
            Row::new(vec![
                format!("{}{}", active_marker, p.id),
                name.chars().take(30).collect(),
                interval,
                next,
                status.to_string(),
                p.url.chars().take(50).collect(),
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Length(6),
        Constraint::Length(28),
        Constraint::Length(10),
        Constraint::Length(16),
        Constraint::Length(8),
        Constraint::Ratio(1, 1),
    ];
    let row_count = data_rows.len();
    let table = Table::new(data_rows, widths)
        .header(header)
        .style(Style::default().bg(CLASH_THEME.surface));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLASH_THEME.border))
        .title(Span::styled(
            format!(" {} ", app.t(Msg::PageSubscriptions)),
            Style::default().fg(CLASH_THEME.primary).bold(),
        ))
        .title_bottom(Span::styled(
            format!(" {} ", subscription_actions_line_text(app)),
            Style::default().fg(CLASH_THEME.muted),
        ));
    let inner = block.inner(table_area);
    frame.render_widget(block, table_area);
    register_table_row_hitboxes(app, inner, row_count, HitboxAction::SelectSubscription);
    register_subscription_action_hitboxes(app, table_area);
    if app.profiles.is_empty() {
        frame.render_widget(
            Paragraph::new(app.t(Msg::SubscriptionsNoProfiles)).style(
                Style::default()
                    .fg(CLASH_THEME.muted)
                    .bg(CLASH_THEME.surface),
            ),
            inner,
        );
    } else {
        frame.render_widget(table, inner);
    }
    if let Some(output_area) = output_area {
        render_subscription_output(frame, output_area, app);
    }
}

fn register_subscription_action_hitboxes(app: &mut App, area: Rect) {
    if area.height < 2 || area.width < 12 {
        return;
    }
    let y = area.y + area.height.saturating_sub(1);
    let mut x = area.x + 2;
    for (label, action) in subscription_action_buttons(app) {
        let width = action_button_width(&label);
        register_clamped_hitbox(app, x, y, width, area, action);
        x = x.saturating_add(width + 1);
    }
}

fn subscription_actions_line_text(app: &App) -> String {
    subscription_action_buttons(app)
        .into_iter()
        .map(|(label, _)| format!("[{}]", label))
        .collect::<Vec<_>>()
        .join(" ")
}

fn subscription_action_buttons(app: &App) -> Vec<(String, HitboxAction)> {
    let registry_button = |id: &str, action| {
        let label = action_registry::action_by_id(id)
            .map(|spec| app.action_button(spec))
            .unwrap_or("action")
            .to_string();
        (label, action)
    };
    vec![
        registry_button("sub.add", HitboxAction::BeginSubscriptionAdd),
        registry_button("sub.import", HitboxAction::BeginSubscriptionImport),
        registry_button("sub.log", HitboxAction::ShowSubscriptionLog),
        registry_button("sub.remove", HitboxAction::RemoveSubscription),
        registry_button("sub.use", HitboxAction::UseSubscription),
        registry_button("sub.update", HitboxAction::UpdateSubscription),
        registry_button("sub.edit", HitboxAction::EditSubscriptionName),
        (
            app.t(Msg::SubscriptionsQuickUrl).to_string(),
            HitboxAction::EditSubscriptionUrl,
        ),
        (
            app.t(Msg::SubscriptionsQuickInterval).to_string(),
            HitboxAction::EditSubscriptionInterval,
        ),
        (
            app.t(Msg::SubscriptionsQuickUserAgent).to_string(),
            HitboxAction::EditSubscriptionUserAgent,
        ),
        (
            app.t(Msg::SubscriptionsQuickUpdateProxy).to_string(),
            HitboxAction::EditSubscriptionUpdateProxy,
        ),
        (
            app.t(Msg::SubscriptionsQuickConvert).to_string(),
            HitboxAction::EditSubscriptionConvertMode,
        ),
        (
            app.t(Msg::SubscriptionsQuickAddTag).to_string(),
            HitboxAction::EditSubscriptionAddTag,
        ),
        (
            app.t(Msg::SubscriptionsQuickRemoveTag).to_string(),
            HitboxAction::EditSubscriptionRemoveTag,
        ),
    ]
}

fn render_subscription_output(frame: &mut Frame, area: Rect, app: &App) {
    let lines = if app.subscription_output.is_empty() {
        vec![Line::from(Span::styled(
            format!("  {}", app.t(Msg::SubscriptionsOutputPlaceholder)),
            CLASH_THEME.muted,
        ))]
    } else {
        app.subscription_output
            .iter()
            .map(|line| Line::from(Span::styled(format!("  {}", line), CLASH_THEME.text)))
            .collect()
    };
    crate::widgets::card::Card::new(app.t(Msg::NetworkCommandOutput)).render(frame, area, lines);
}

fn render_subscription_prompt(frame: &mut Frame, area: Rect, app: &mut App) {
    if app.subscription_add_form.is_some() {
        render_subscription_add_form(frame, area, app);
        return;
    }
    if app.subscription_edit_form.is_some() {
        render_subscription_edit_form(frame, area, app);
        return;
    }
    let Some(prompt) = app.subscription_prompt.as_ref() else {
        return;
    };
    let popup = centered_rect(area, 72, 32);
    fill_area(frame, popup, CLASH_THEME.bg);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLASH_THEME.primary))
        .title(Span::styled(
            if prompt.field == SubscriptionEditField::ImportDirectory {
                format!(" {} ", app.subscription_edit_field_label(prompt.field))
            } else {
                format!(
                    " {} {} · [{}] ",
                    app.t(Msg::SubscriptionsEditTitle),
                    app.subscription_edit_field_label(prompt.field),
                    prompt.profile_id
                )
            },
            Style::default().fg(CLASH_THEME.primary).bold(),
        ))
        .title_bottom(Span::styled(
            format!(" {} ", app.t(Msg::SubscriptionsQuickPromptControlsHint)),
            Style::default().fg(CLASH_THEME.muted),
        ));
    let inner = block.inner(popup).inner(Margin {
        vertical: 1,
        horizontal: 2,
    });

    let value = if prompt.value.is_empty() {
        format!("<{}>", app.t(Msg::CommonEmpty))
    } else {
        prompt.value.clone()
    };
    let lines = vec![
        Line::from(Span::styled(
            app.subscription_edit_field_hint(prompt.field),
            CLASH_THEME.muted,
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!("{}: ", app.t(Msg::CommonValue)),
                CLASH_THEME.primary,
            ),
            Span::styled(value, CLASH_THEME.text),
            Span::styled("█", CLASH_THEME.primary),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("[{}]", app.t(Msg::CommonSave)), CLASH_THEME.primary),
            Span::styled("   ", CLASH_THEME.text),
            Span::styled(
                format!("[{}]", app.t(Msg::CommonCancel)),
                CLASH_THEME.warning,
            ),
        ]),
    ];
    frame.render_widget(block, popup);
    app.hitboxes.register(
        Rect::new(inner.x, inner.y.saturating_add(2), inner.width, 1),
        HitboxAction::FocusSettingsPromptValue,
    );
    let button_y = inner.y.saturating_add(4);
    let save_width = action_button_width(app.t(Msg::CommonSave));
    let cancel_width = action_button_width(app.t(Msg::CommonCancel));
    app.hitboxes.register(
        Rect::new(inner.x, button_y, save_width, 1),
        HitboxAction::SubmitSubscriptionPrompt,
    );
    app.hitboxes.register(
        Rect::new(
            inner.x.saturating_add(save_width).saturating_add(3),
            button_y,
            cancel_width,
            1,
        ),
        HitboxAction::CancelSubscriptionPrompt,
    );
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().fg(CLASH_THEME.text).bg(CLASH_THEME.bg)),
        inner,
    );
}

fn render_subscription_edit_form(frame: &mut Frame, area: Rect, app: &mut App) {
    let Some(form) = app.subscription_edit_form.as_ref() else {
        return;
    };
    let popup = centered_rect(area, 82, 56);
    fill_area(frame, popup, CLASH_THEME.bg);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLASH_THEME.primary))
        .title(Span::styled(
            format!(
                " {} {} ",
                app.t(Msg::SubscriptionsEditTitle),
                form.profile_id
            ),
            Style::default().fg(CLASH_THEME.primary).bold(),
        ))
        .title_bottom(Span::styled(
            format!(" {} ", app.t(Msg::SubscriptionsFormControlsHint)),
            Style::default().fg(CLASH_THEME.muted),
        ));
    let inner = block.inner(popup).inner(Margin {
        vertical: 1,
        horizontal: 2,
    });

    let mut lines = vec![
        Line::from(Span::styled(
            app.subscription_add_field_hint(form.active_field()),
            CLASH_THEME.muted,
        )),
        Line::from(""),
    ];
    for (idx, field) in SubscriptionAddField::all().iter().enumerate() {
        let active = idx == form.active;
        let label_style = if active {
            Style::default().fg(CLASH_THEME.bg).bg(CLASH_THEME.primary)
        } else {
            Style::default().fg(CLASH_THEME.primary).bg(CLASH_THEME.bg)
        };
        let value_style = if active {
            Style::default()
                .fg(CLASH_THEME.text)
                .bg(CLASH_THEME.primary)
        } else {
            Style::default().fg(CLASH_THEME.text).bg(CLASH_THEME.bg)
        };
        let value = edit_form_display_value(app, form, *field, active);
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:<14}", app.subscription_add_field_label(*field)),
                label_style,
            ),
            Span::styled(" ", value_style),
            Span::styled(value, value_style),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(format!("[{}]", app.t(Msg::CommonSave)), CLASH_THEME.primary),
        Span::styled("   ", CLASH_THEME.text),
        Span::styled(
            format!("[{}]", app.t(Msg::CommonCancel)),
            CLASH_THEME.warning,
        ),
    ]));

    frame.render_widget(block, popup);
    register_subscription_form_hitboxes(app, inner);
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().fg(CLASH_THEME.text).bg(CLASH_THEME.bg)),
        inner,
    );
}

fn render_subscription_add_form(frame: &mut Frame, area: Rect, app: &mut App) {
    let Some(form) = app.subscription_add_form.as_ref() else {
        return;
    };
    let popup = centered_rect(area, 82, 56);
    fill_area(frame, popup, CLASH_THEME.bg);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLASH_THEME.primary))
        .title(Span::styled(
            format!(" {} ", app.t(Msg::SubscriptionsAddTitle)),
            Style::default().fg(CLASH_THEME.primary).bold(),
        ))
        .title_bottom(Span::styled(
            format!(" {} ", app.t(Msg::SubscriptionsFormControlsHint)),
            Style::default().fg(CLASH_THEME.muted),
        ));
    let inner = block.inner(popup).inner(Margin {
        vertical: 1,
        horizontal: 2,
    });

    let mut lines = vec![
        Line::from(Span::styled(
            app.subscription_add_field_hint(form.active_field()),
            CLASH_THEME.muted,
        )),
        Line::from(""),
    ];
    for (idx, field) in SubscriptionAddField::all().iter().enumerate() {
        let active = idx == form.active;
        let label_style = if active {
            Style::default().fg(CLASH_THEME.bg).bg(CLASH_THEME.primary)
        } else {
            Style::default().fg(CLASH_THEME.primary).bg(CLASH_THEME.bg)
        };
        let value_style = if active {
            Style::default()
                .fg(CLASH_THEME.text)
                .bg(CLASH_THEME.primary)
        } else {
            Style::default().fg(CLASH_THEME.text).bg(CLASH_THEME.bg)
        };
        let value = add_form_display_value(app, form, *field, active);
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:<14}", app.subscription_add_field_label(*field)),
                label_style,
            ),
            Span::styled(" ", value_style),
            Span::styled(value, value_style),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(format!("[{}]", app.t(Msg::CommonSave)), CLASH_THEME.primary),
        Span::styled("   ", CLASH_THEME.text),
        Span::styled(
            format!("[{}]", app.t(Msg::CommonCancel)),
            CLASH_THEME.warning,
        ),
    ]));

    frame.render_widget(block, popup);
    register_subscription_form_hitboxes(app, inner);
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().fg(CLASH_THEME.text).bg(CLASH_THEME.bg)),
        inner,
    );
}

fn register_subscription_form_hitboxes(app: &mut App, inner: Rect) {
    let field_y = inner.y.saturating_add(2);
    for (idx, _) in SubscriptionAddField::all().iter().enumerate() {
        if field_y.saturating_add(idx as u16) < inner.y.saturating_add(inner.height) {
            app.hitboxes.register(
                Rect::new(inner.x, field_y + idx as u16, inner.width, 1),
                HitboxAction::SelectSubscriptionAddField(idx),
            );
        }
    }
    let button_y = field_y
        .saturating_add(SubscriptionAddField::all().len() as u16)
        .saturating_add(1);
    let save_width = action_button_width(app.t(Msg::CommonSave));
    let cancel_width = action_button_width(app.t(Msg::CommonCancel));
    app.hitboxes.register(
        Rect::new(inner.x, button_y, save_width, 1),
        HitboxAction::SubmitSubscriptionPrompt,
    );
    app.hitboxes.register(
        Rect::new(
            inner.x.saturating_add(save_width).saturating_add(3),
            button_y,
            cancel_width,
            1,
        ),
        HitboxAction::CancelSubscriptionPrompt,
    );
}

fn add_form_display_value(
    app: &App,
    form: &SubscriptionAddForm,
    field: SubscriptionAddField,
    active: bool,
) -> String {
    let raw = match field {
        SubscriptionAddField::Source => &form.source,
        SubscriptionAddField::Name => &form.name,
        SubscriptionAddField::Interval => &form.interval,
        SubscriptionAddField::UpdateProxy => &form.update_proxy,
        SubscriptionAddField::UserAgent => &form.user_agent,
        SubscriptionAddField::ConvertMode => &form.convert_mode,
        SubscriptionAddField::Tags => &form.tags,
    };
    let mut value = if raw.is_empty() {
        match field {
            SubscriptionAddField::Source => format!("<{}>", app.t(Msg::CommonRequired)),
            SubscriptionAddField::Name
            | SubscriptionAddField::Interval
            | SubscriptionAddField::UserAgent
            | SubscriptionAddField::Tags => format!("<{}>", app.t(Msg::CommonOptional)),
            SubscriptionAddField::UpdateProxy | SubscriptionAddField::ConvertMode => {
                format!("<{}>", app.t(Msg::CommonDefault))
            }
        }
    } else {
        trunc_str(raw, 76)
    };
    if active {
        value.push('█');
    }
    value
}

fn edit_form_display_value(
    app: &App,
    form: &SubscriptionEditForm,
    field: SubscriptionAddField,
    active: bool,
) -> String {
    let raw = match field {
        SubscriptionAddField::Source => &form.url,
        SubscriptionAddField::Name => &form.name,
        SubscriptionAddField::Interval => &form.interval,
        SubscriptionAddField::UpdateProxy => &form.update_proxy,
        SubscriptionAddField::UserAgent => &form.user_agent,
        SubscriptionAddField::ConvertMode => &form.convert_mode,
        SubscriptionAddField::Tags => &form.tags,
    };
    let mut value = if raw.is_empty() {
        match field {
            SubscriptionAddField::Source => format!("<{}>", app.t(Msg::CommonRequired)),
            SubscriptionAddField::Name
            | SubscriptionAddField::UserAgent
            | SubscriptionAddField::Tags => format!("<{}>", app.t(Msg::CommonEmpty)),
            SubscriptionAddField::Interval
            | SubscriptionAddField::UpdateProxy
            | SubscriptionAddField::ConvertMode => format!("<{}>", app.t(Msg::CommonRequired)),
        }
    } else {
        trunc_str(raw, 76)
    };
    if active {
        value.push('█');
    }
    value
}

fn render_settings_prompt(frame: &mut Frame, area: Rect, app: &mut App) {
    let Some(prompt) = app.settings_prompt.clone() else {
        return;
    };
    let popup = centered_rect(area, 68, 44);
    fill_area(frame, popup, CLASH_THEME.bg);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLASH_THEME.warning))
        .title(Span::styled(
            format!(" {} ", app.settings_prompt_label(prompt.kind)),
            Style::default().fg(CLASH_THEME.warning).bold(),
        ))
        .title_bottom(Span::styled(
            format!(" {} ", app.t(Msg::SettingsPromptControlsHint)),
            Style::default().fg(CLASH_THEME.muted),
        ));
    let inner = block.inner(popup).inner(Margin {
        vertical: 1,
        horizontal: 2,
    });

    let active_hint = prompt
        .active_field()
        .map(|field| app.settings_prompt_field_hint(field.field))
        .unwrap_or_else(|| app.settings_prompt_hint(prompt.kind));
    let mut lines = vec![
        Line::from(Span::styled(
            app.settings_prompt_hint(prompt.kind),
            CLASH_THEME.muted,
        )),
        Line::from(Span::styled(active_hint, CLASH_THEME.muted)),
        Line::from(""),
    ];
    for (idx, field) in prompt.fields.iter().enumerate() {
        let active = idx == prompt.active;
        let marker = if active { ">" } else { " " };
        let mut displayed = field.display_value();
        if active {
            displayed.push('█');
        }
        lines.push(Line::from(vec![
            Span::styled(
                format!(
                    "{} {:<14} ",
                    marker,
                    app.settings_prompt_field_label(field.field)
                ),
                if active {
                    CLASH_THEME.warning
                } else {
                    CLASH_THEME.muted
                },
            ),
            Span::styled(displayed, CLASH_THEME.text),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(
            format!("[{}]", app.t(Msg::CommonContinue)),
            CLASH_THEME.warning,
        ),
        Span::styled("   ", CLASH_THEME.text),
        Span::styled(
            format!("[{}]", app.t(Msg::CommonCancel)),
            CLASH_THEME.primary,
        ),
    ]));
    frame.render_widget(block, popup);
    for idx in 0..prompt.fields.len() {
        app.hitboxes.register(
            Rect::new(
                inner.x,
                inner.y.saturating_add(3 + idx as u16),
                inner.width,
                1,
            ),
            HitboxAction::SelectSettingsPromptField(idx),
        );
    }
    app.hitboxes.register(
        Rect::new(
            inner.x,
            inner.y.saturating_add(4 + prompt.fields.len() as u16),
            action_button_width(app.t(Msg::CommonContinue)),
            1,
        ),
        HitboxAction::SubmitSettingsPrompt,
    );
    app.hitboxes.register(
        Rect::new(
            inner
                .x
                .saturating_add(action_button_width(app.t(Msg::CommonContinue)).saturating_add(3)),
            inner.y.saturating_add(4 + prompt.fields.len() as u16),
            action_button_width(app.t(Msg::CommonCancel)),
            1,
        ),
        HitboxAction::CancelSettingsPrompt,
    );
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().fg(CLASH_THEME.text).bg(CLASH_THEME.bg)),
        inner,
    );
}

fn render_confirmation_prompt(frame: &mut Frame, area: Rect, app: &mut App) {
    let Some(prompt) = app.pending_confirmation.clone() else {
        return;
    };
    let popup = centered_rect(area, 64, 28);
    fill_area(frame, popup, CLASH_THEME.bg);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLASH_THEME.warning))
        .title(Span::styled(
            format!(" {} ", prompt.title),
            Style::default().fg(CLASH_THEME.warning).bold(),
        ))
        .title_bottom(Span::styled(
            format!(" {} ", app.t(Msg::ConfirmHint)),
            Style::default().fg(CLASH_THEME.muted),
        ));
    let inner = block.inner(popup).inner(Margin {
        vertical: 1,
        horizontal: 2,
    });

    let lines = vec![
        Line::from(Span::styled(
            app.t(Msg::ConfirmSystemStateWarning),
            CLASH_THEME.muted,
        )),
        Line::from(""),
        Line::from(Span::styled(prompt.message, CLASH_THEME.text)),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!("[{}]", app.t(Msg::CommonConfirm)),
                CLASH_THEME.warning,
            ),
            Span::styled("   ", CLASH_THEME.text),
            Span::styled(
                format!("[{}]", app.t(Msg::CommonCancel)),
                CLASH_THEME.primary,
            ),
        ]),
    ];
    frame.render_widget(block, popup);
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().fg(CLASH_THEME.text).bg(CLASH_THEME.bg)),
        inner,
    );
    let confirm_width = action_button_width(app.t(Msg::CommonConfirm));
    let cancel_width = action_button_width(app.t(Msg::CommonCancel));
    app.hitboxes.register(
        Rect::new(inner.x, inner.y + 4, confirm_width, 1),
        HitboxAction::ConfirmPendingAction,
    );
    app.hitboxes.register(
        Rect::new(
            inner.x.saturating_add(confirm_width).saturating_add(3),
            inner.y + 4,
            cancel_width,
            1,
        ),
        HitboxAction::CancelPendingAction,
    );
}

fn render_traffic(frame: &mut Frame, area: Rect, app: &mut App) {
    fill_area(frame, area, CLASH_THEME.surface);

    let rows = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(10),
        Constraint::Length(9),
    ])
    .split(area);

    let (down_total, up_total, down_rate, up_rate) = traffic_summary(&app.traffic_points);
    let filter = app
        .traffic_filter_key
        .as_deref()
        .map(|key| trunc_str(key, 32))
        .unwrap_or_else(|| app.t(Msg::TrafficAll).into());
    let summary = vec![Line::from(vec![
        Span::styled(
            format!(
                "  {} [{}]  {} [{}]  {} [{}]  {} [{}]  ",
                app.t(Msg::TrafficRange),
                app.traffic_range.label(),
                app.t(Msg::TrafficChart),
                app.traffic_chart.label(),
                app.t(Msg::TrafficBy),
                app.traffic_dimension.label(),
                app.t(Msg::TrafficFilter),
                filter
            ),
            CLASH_THEME.muted,
        ),
        Span::styled(
            format!(
                "{} {}  {} {}  {} ↓ {}/s ↑ {}/s",
                app.t(Msg::TrafficDown),
                format_bytes(down_total),
                app.t(Msg::TrafficUp),
                format_bytes(up_total),
                app.t(Msg::TrafficPeak),
                format_bytes(down_rate),
                format_bytes(up_rate)
            ),
            CLASH_THEME.text,
        ),
    ])];
    crate::widgets::card::Card::new(app.t(Msg::PageTraffic)).render(frame, rows[0], summary);
    register_traffic_control_hitboxes(app, rows[0]);

    let main =
        Layout::horizontal([Constraint::Ratio(2, 3), Constraint::Ratio(1, 3)]).split(rows[1]);
    render_traffic_chart(frame, main[0], app);
    render_traffic_top(frame, main[1], app);
    render_traffic_detail(frame, rows[2], app);
}

fn register_traffic_control_hitboxes(app: &mut App, area: Rect) {
    if area.height < 3 || area.width < 12 {
        return;
    }
    let y = area.y + 1;
    let x = area.x + 2;
    register_clamped_hitbox(app, x, y, 14, area, HitboxAction::NextTrafficRange);
    register_clamped_hitbox(app, x + 15, y, 14, area, HitboxAction::ToggleTrafficChart);
    register_clamped_hitbox(app, x + 30, y, 12, area, HitboxAction::NextTrafficDimension);
}

fn register_clamped_hitbox(
    app: &mut App,
    x: u16,
    y: u16,
    width: u16,
    area: Rect,
    action: HitboxAction,
) {
    if x >= area.x.saturating_add(area.width) || y >= area.y.saturating_add(area.height) {
        return;
    }
    let available = area.x.saturating_add(area.width).saturating_sub(x);
    app.hitboxes
        .register(Rect::new(x, y, width.min(available), 1), action);
}

fn action_fg(danger: ActionDanger) -> Color {
    match danger {
        ActionDanger::Dangerous => CLASH_THEME.danger,
        ActionDanger::Confirm | ActionDanger::Sensitive => CLASH_THEME.warning,
        ActionDanger::Safe => CLASH_THEME.text,
    }
}

fn action_button_width(label: &str) -> u16 {
    label.chars().count() as u16 + 2
}

fn hitbox_for_action_spec(spec: &action_registry::ActionSpec) -> Option<HitboxAction> {
    match spec.executor {
        ActionExecutor::Network(action) => Some(HitboxAction::RunNetwork(action)),
        ActionExecutor::Settings(action) => Some(HitboxAction::RunSettings(action)),
        ActionExecutor::Traffic(action) => Some(HitboxAction::RunTraffic(action)),
        ActionExecutor::Api("PATCH /configs mode") => Some(HitboxAction::CycleProxyMode),
        ActionExecutor::Api("PUT /proxies/{group}") => Some(HitboxAction::SwitchSelectedProxyNode),
        ActionExecutor::Api("GET /proxies/{name}/delay") => {
            Some(HitboxAction::TestSelectedProxyDelay)
        }
        ActionExecutor::Api("DELETE /connections/{id}") => {
            Some(HitboxAction::CloseSelectedConnection)
        }
        ActionExecutor::Api("DELETE /connections") => Some(HitboxAction::CloseAllConnections),
        ActionExecutor::Prompt("config set-port") => Some(HitboxAction::BeginConfigSetPorts),
        ActionExecutor::Prompt("config set-api") => Some(HitboxAction::BeginConfigSetApi),
        ActionExecutor::Prompt("config set-dns-mode") => Some(HitboxAction::BeginConfigSetDns),
        ActionExecutor::Prompt("config set-lan") => Some(HitboxAction::BeginConfigSetLan),
        ActionExecutor::Prompt("traffic prune retention") => {
            Some(HitboxAction::BeginTrafficPruneRetention)
        }
        ActionExecutor::Prompt("geodata update version") => {
            Some(HitboxAction::BeginGeodataUpdateVersion)
        }
        ActionExecutor::Prompt("secret set") => Some(HitboxAction::BeginSecretSet),
        ActionExecutor::Prompt("sub add") => Some(HitboxAction::BeginSubscriptionAdd),
        ActionExecutor::Prompt("sub import") => Some(HitboxAction::BeginSubscriptionImport),
        ActionExecutor::Internal("toggle proxy sort") => Some(HitboxAction::ToggleProxySort),
        ActionExecutor::Internal("toggle node picker") => Some(HitboxAction::ToggleNodePicker),
        ActionExecutor::Internal("close node picker") => Some(HitboxAction::CloseNodePicker),
        ActionExecutor::Internal("test all proxy delays") => Some(HitboxAction::TestAllProxyDelays),
        ActionExecutor::Internal("cycle ui language") => Some(HitboxAction::CycleUiLanguage),
        ActionExecutor::Internal("cycle theme preference") => {
            Some(HitboxAction::CycleThemePreference)
        }
        ActionExecutor::Internal("cycle default page") => Some(HitboxAction::CycleDefaultPage),
        ActionExecutor::Internal("cycle refresh interval") => {
            Some(HitboxAction::CycleRefreshInterval)
        }
        ActionExecutor::Internal("toggle mouse preference") => {
            Some(HitboxAction::ToggleMousePreference)
        }
        ActionExecutor::Internal("toggle dangerous confirmations") => {
            Some(HitboxAction::ToggleDangerousConfirmations)
        }
        ActionExecutor::Internal("cycle default traffic range") => {
            Some(HitboxAction::CycleDefaultTrafficRange)
        }
        ActionExecutor::Internal("cycle default traffic chart") => {
            Some(HitboxAction::CycleDefaultTrafficChart)
        }
        ActionExecutor::Internal("cycle default traffic dimension") => {
            Some(HitboxAction::CycleDefaultTrafficDimension)
        }
        ActionExecutor::Internal("toggle log pause") => Some(HitboxAction::ToggleLogPause),
        ActionExecutor::Internal("cycle log level") => Some(HitboxAction::CycleLogFilter),
        ActionExecutor::Internal("clear local logs") => Some(HitboxAction::ClearLogs),
        ActionExecutor::Internal("clashctl traffic export --format csv") => {
            Some(HitboxAction::ExportTraffic)
        }
        ActionExecutor::Internal("clashctl sub log") => Some(HitboxAction::ShowSubscriptionLog),
        _ => None,
    }
}

fn render_traffic_chart(frame: &mut Frame, area: Rect, app: &mut App) {
    let filter = app
        .traffic_filter_key
        .as_deref()
        .map(|key| trunc_str(key, 28))
        .unwrap_or_else(|| app.t(Msg::TrafficAllTraffic).into());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLASH_THEME.border))
        .title(Span::styled(
            format!(
                " {} · {} · {} · {} · {} ",
                app.t(Msg::TrafficHistory),
                app.traffic_range.label(),
                app.traffic_range.step_arg(),
                app.traffic_chart.label(),
                filter
            ),
            Style::default().fg(CLASH_THEME.primary).bold(),
        ))
        .title_bottom(Span::styled(
            format!(" {} ", app.t(Msg::TrafficHistoryHint)),
            Style::default().fg(CLASH_THEME.muted),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    app.hitboxes
        .register(inner, HitboxAction::ScrollTrafficChart);

    if app.traffic_points.is_empty() {
        let msg = app
            .traffic_error
            .as_deref()
            .unwrap_or(app.t(Msg::TrafficNoHistory));
        frame.render_widget(
            Paragraph::new(msg).style(
                Style::default()
                    .fg(CLASH_THEME.muted)
                    .bg(CLASH_THEME.surface),
            ),
            inner,
        );
        return;
    }

    if app.traffic_chart == TrafficChartKind::Line {
        render_traffic_line_chart(frame, inner, app);
        return;
    }

    let max_lines = inner.height.max(1) as usize;
    let (start, end) = traffic_window_bounds(
        app.traffic_points.len(),
        max_lines,
        app.traffic_window_offset,
    );
    register_traffic_bar_bucket_hitboxes(app, inner, start, end.saturating_sub(start));
    let visible = &app.traffic_points[start..end];
    let max_down = visible
        .iter()
        .map(|p| p.download_delta.max(p.down_bps))
        .max()
        .unwrap_or(1)
        .max(1);
    let max_up = visible
        .iter()
        .map(|p| p.upload_delta.max(p.up_bps))
        .max()
        .unwrap_or(1)
        .max(1);
    let bar_width = inner.width.saturating_sub(26).max(8) as usize;
    let mut lines = Vec::new();
    for point in visible {
        let down_bar = scaled_bar(
            point.download_delta.max(point.down_bps),
            max_down,
            bar_width,
        );
        let up_bar = scaled_bar(point.upload_delta.max(point.up_bps), max_up, bar_width / 3);
        lines.push(Line::from(vec![
            Span::styled(format!("{} ", point.ts.format("%H:%M")), CLASH_THEME.muted),
            Span::styled(down_bar, CLASH_THEME.primary),
            Span::styled(" ", CLASH_THEME.text),
            Span::styled(up_bar, CLASH_THEME.accent),
            Span::styled(
                format!(" ↓{}", format_bytes(point.download_delta)),
                CLASH_THEME.text,
            ),
        ]));
    }
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(CLASH_THEME.surface)),
        inner,
    );
}

fn render_traffic_line_chart(frame: &mut Frame, area: Rect, app: &mut App) {
    if area.width < 16 || area.height < 4 {
        return;
    }
    let chart_width = area.width.saturating_sub(11).max(8) as usize;
    let chart_height = area.height.saturating_sub(2).max(2) as usize;
    let (start, end) = traffic_window_bounds(
        app.traffic_points.len(),
        chart_width,
        app.traffic_window_offset,
    );
    register_traffic_line_bucket_hitboxes(
        app,
        area,
        start,
        end.saturating_sub(start),
        chart_width,
        10,
    );
    let visible = &app.traffic_points[start..end];
    let lines = traffic_line_chart_lines(app, visible, chart_width, chart_height);
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(CLASH_THEME.surface)),
        area,
    );
}

fn register_traffic_line_bucket_hitboxes(
    app: &mut App,
    area: Rect,
    start: usize,
    count: usize,
    plot_width: usize,
    plot_x_offset: u16,
) {
    if count == 0 || area.width == 0 || area.height == 0 {
        return;
    }
    for idx in 0..count {
        let x_offset = if count <= 1 || plot_width <= 1 {
            0
        } else {
            idx * (plot_width - 1) / (count - 1)
        };
        let x = area
            .x
            .saturating_add(plot_x_offset)
            .saturating_add(x_offset as u16);
        if x >= area.x.saturating_add(area.width) {
            continue;
        }
        app.hitboxes.register(
            Rect::new(x, area.y, 1, area.height),
            HitboxAction::SelectTrafficBucket(start + idx),
        );
    }
}

fn register_traffic_bar_bucket_hitboxes(app: &mut App, area: Rect, start: usize, count: usize) {
    if count == 0 || area.width == 0 || area.height == 0 {
        return;
    }
    for idx in 0..count.min(area.height as usize) {
        app.hitboxes.register(
            Rect::new(area.x, area.y.saturating_add(idx as u16), area.width, 1),
            HitboxAction::SelectTrafficBucket(start + idx),
        );
    }
}

fn traffic_window_bounds(total: usize, capacity: usize, offset: usize) -> (usize, usize) {
    if total == 0 {
        return (0, 0);
    }
    let capacity = capacity.max(1).min(total);
    let offset = offset.min(total.saturating_sub(capacity));
    let end = total.saturating_sub(offset);
    let start = end.saturating_sub(capacity);
    (start, end)
}

fn traffic_line_chart_lines(
    app: &App,
    points: &[TrafficPoint],
    width: usize,
    height: usize,
) -> Vec<Line<'static>> {
    let width = width.max(1);
    let height = height.max(1);
    let down: Vec<u64> = points
        .iter()
        .map(|p| p.download_delta.max(p.down_bps))
        .collect();
    let up: Vec<u64> = points
        .iter()
        .map(|p| p.upload_delta.max(p.up_bps))
        .collect();
    let max_value = down
        .iter()
        .chain(up.iter())
        .copied()
        .max()
        .unwrap_or(1)
        .max(1);
    let mut canvas = vec![vec![' '; width]; height];
    plot_traffic_series(&mut canvas, &down, max_value, 'd');
    plot_traffic_series(&mut canvas, &up, max_value, 'u');

    let mut lines = vec![Line::from(vec![
        Span::styled("  d", CLASH_THEME.primary),
        Span::styled(
            format!(" {}  ", app.t(Msg::TrafficDownload)),
            CLASH_THEME.muted,
        ),
        Span::styled("u", CLASH_THEME.accent),
        Span::styled(
            format!(" {}  ", app.t(Msg::TrafficUpload)),
            CLASH_THEME.muted,
        ),
        Span::styled(
            format!("{} {}", app.t(Msg::TrafficMax), format_bytes(max_value)),
            CLASH_THEME.text,
        ),
    ])];
    for (row_idx, row) in canvas.into_iter().enumerate() {
        let label = traffic_axis_label(row_idx, height, max_value);
        let mut spans = vec![Span::styled(format!("{:>8} |", label), CLASH_THEME.muted)];
        for ch in row {
            let style = match ch {
                'd' => CLASH_THEME.primary,
                'u' => CLASH_THEME.accent,
                '*' => CLASH_THEME.warning,
                _ => CLASH_THEME.surface,
            };
            spans.push(Span::styled(ch.to_string(), style));
        }
        lines.push(Line::from(spans));
    }
    let from = points
        .first()
        .map(|p| p.ts.format("%H:%M").to_string())
        .unwrap_or_else(|| "--:--".into());
    let to = points
        .last()
        .map(|p| p.ts.format("%H:%M").to_string())
        .unwrap_or_else(|| "--:--".into());
    let gap = width.saturating_sub(from.len() + to.len()).max(1);
    lines.push(Line::from(vec![
        Span::styled("         +", CLASH_THEME.muted),
        Span::styled("-".repeat(width), CLASH_THEME.muted),
        Span::styled(" ", CLASH_THEME.muted),
        Span::styled(
            format!("{}{}{}", from, " ".repeat(gap), to),
            CLASH_THEME.muted,
        ),
    ]));
    lines
}

fn traffic_axis_label(row_idx: usize, height: usize, max_value: u64) -> String {
    if row_idx == 0 {
        trunc_str(&format_bytes(max_value), 8)
    } else if row_idx + 1 == height {
        "0".into()
    } else if row_idx == height / 2 {
        trunc_str(&format_bytes(max_value / 2), 8)
    } else {
        String::new()
    }
}

fn plot_traffic_series(canvas: &mut [Vec<char>], values: &[u64], max_value: u64, marker: char) {
    if values.is_empty() || canvas.is_empty() || canvas[0].is_empty() {
        return;
    }
    let height = canvas.len();
    let width = canvas[0].len();
    let mut previous: Option<(usize, usize)> = None;
    for (idx, value) in values.iter().enumerate() {
        let x = if values.len() <= 1 || width <= 1 {
            0
        } else {
            idx * (width - 1) / (values.len() - 1)
        };
        let y = traffic_chart_y(*value, max_value, height);
        if let Some((prev_x, prev_y)) = previous {
            draw_traffic_segment(canvas, prev_x, prev_y, x, y, marker);
        } else {
            mark_traffic_canvas(canvas, x, y, marker);
        }
        previous = Some((x, y));
    }
}

fn draw_traffic_segment(
    canvas: &mut [Vec<char>],
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
    marker: char,
) {
    if x0 == x1 {
        let start = y0.min(y1);
        let end = y0.max(y1);
        for y in start..=end {
            mark_traffic_canvas(canvas, x0, y, marker);
        }
        return;
    }
    let start = x0.min(x1);
    let end = x0.max(x1);
    let span = (x1 as isize - x0 as isize) as f64;
    for x in start..=end {
        let t = (x as isize - x0 as isize) as f64 / span;
        let y = (y0 as f64 + (y1 as f64 - y0 as f64) * t).round() as usize;
        mark_traffic_canvas(canvas, x, y, marker);
    }
}

fn traffic_chart_y(value: u64, max_value: u64, height: usize) -> usize {
    if height <= 1 {
        return 0;
    }
    let ratio = value as f64 / max_value.max(1) as f64;
    (((height - 1) as f64) * (1.0 - ratio.clamp(0.0, 1.0))).round() as usize
}

fn mark_traffic_canvas(canvas: &mut [Vec<char>], x: usize, y: usize, marker: char) {
    if y >= canvas.len() || x >= canvas[y].len() {
        return;
    }
    canvas[y][x] = match canvas[y][x] {
        ' ' => marker,
        existing if existing == marker => existing,
        _ => '*',
    };
}

fn render_traffic_top(frame: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLASH_THEME.border))
        .title(Span::styled(
            format!(
                " {} {} ",
                app.traffic_dimension.label(),
                app.t(Msg::TrafficBreakdown)
            ),
            Style::default().fg(CLASH_THEME.primary).bold(),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    app.hitboxes
        .register(inner, HitboxAction::ScrollTrafficRows);

    if app.traffic_top.is_empty() {
        frame.render_widget(
            Paragraph::new(app.t(Msg::TrafficNoBreakdown))
                .style(Style::default().fg(CLASH_THEME.muted)),
            inner,
        );
        return;
    }

    let rows: Vec<Row> = app
        .traffic_top
        .iter()
        .enumerate()
        .map(|(idx, row)| {
            let style = if idx == app.traffic_selected_idx {
                Style::default()
                    .fg(CLASH_THEME.text)
                    .bg(CLASH_THEME.primary)
            } else {
                Style::default()
                    .fg(CLASH_THEME.text)
                    .bg(CLASH_THEME.surface)
            };
            Row::new(vec![trunc_str(&row.key, 34), format_bytes(row.total_delta)]).style(style)
        })
        .collect();
    let table = Table::new(rows, [Constraint::Ratio(2, 3), Constraint::Ratio(1, 3)])
        .header(
            Row::new(vec![
                app.traffic_dimension.label(),
                app.t(Msg::TrafficTotal),
            ])
            .style(Style::default().fg(CLASH_THEME.muted)),
        )
        .style(Style::default().bg(CLASH_THEME.surface));
    register_table_row_hitboxes(
        app,
        inner,
        app.traffic_top.len(),
        HitboxAction::SelectTrafficRow,
    );
    frame.render_widget(table, inner);
}

fn render_traffic_detail(frame: &mut Frame, area: Rect, app: &mut App) {
    let selected = app.traffic_top.get(app.traffic_selected_idx);
    let lines = if let Some(row) = selected {
        let mut lines = vec![
            Line::from(vec![
                Span::styled(
                    format!(
                        "  {} {}: ",
                        app.t(Msg::TrafficSelected),
                        app.traffic_dimension.label()
                    ),
                    CLASH_THEME.muted,
                ),
                Span::styled(row.key.clone(), CLASH_THEME.text),
            ]),
            Line::from(vec![
                Span::styled(
                    format!("  {} ", app.t(Msg::TrafficDownload)),
                    CLASH_THEME.muted,
                ),
                Span::styled(format_bytes(row.download_delta), CLASH_THEME.primary),
                Span::styled(
                    format!("  {} ", app.t(Msg::TrafficUpload)),
                    CLASH_THEME.muted,
                ),
                Span::styled(format_bytes(row.upload_delta), CLASH_THEME.accent),
                Span::styled(
                    format!("  {} ", app.t(Msg::TrafficTotal)),
                    CLASH_THEME.muted,
                ),
                Span::styled(format_bytes(row.total_delta), CLASH_THEME.text),
            ]),
        ];
        lines.extend(traffic_status_lines(app));
        lines.extend(traffic_locked_bucket_lines(app));
        lines.push(traffic_action_line(app));
        lines.extend(traffic_output_lines(app));
        lines
    } else {
        let mut lines = vec![Line::from(Span::styled(
            format!("  {}", app.t(Msg::TrafficDataSource)),
            CLASH_THEME.muted,
        ))];
        lines.extend(traffic_status_lines(app));
        lines.extend(traffic_locked_bucket_lines(app));
        lines.push(traffic_action_line(app));
        lines.extend(traffic_output_lines(app));
        lines
    };
    if area.width > 16 && area.height > 2 {
        register_traffic_action_hitboxes(app, area);
    }
    crate::widgets::card::Card::new(app.t(Msg::TrafficDetail)).render(frame, area, lines);
}

fn traffic_action_line(app: &App) -> Line<'static> {
    let mut spans = vec![Span::styled("  ", CLASH_THEME.text)];
    let mut shortcuts = Vec::new();
    for spec in action_registry::traffic_action_specs() {
        let label = app.action_button(spec);
        spans.push(Span::styled(
            format!("[{}] ", label),
            Style::default().fg(action_fg(spec.danger)),
        ));
        shortcuts.push(spec.shortcut);
    }
    spans.push(Span::styled(
        format!("  {}", shortcuts.join("/")),
        CLASH_THEME.muted,
    ));
    Line::from(spans)
}

fn traffic_locked_bucket_lines(app: &App) -> Vec<Line<'static>> {
    let Some(idx) = app.traffic_locked_bucket else {
        return Vec::new();
    };
    let Some(point) = app.traffic_points.get(idx) else {
        return Vec::new();
    };
    vec![Line::from(vec![
        Span::styled(
            format!("  {} ", app.t(Msg::TrafficBucket)),
            CLASH_THEME.muted,
        ),
        Span::styled(point.ts.format("%H:%M:%S").to_string(), CLASH_THEME.text),
        Span::styled(format!("  {} ", app.t(Msg::TrafficDown)), CLASH_THEME.muted),
        Span::styled(format_bytes(point.download_delta), CLASH_THEME.primary),
        Span::styled(format!("  {} ", app.t(Msg::TrafficUp)), CLASH_THEME.muted),
        Span::styled(format_bytes(point.upload_delta), CLASH_THEME.accent),
        Span::styled(format!("  {} ", app.t(Msg::TrafficRate)), CLASH_THEME.muted),
        Span::styled(
            format!(
                "↓{}/s ↑{}/s  conn {}",
                format_bytes(point.down_bps),
                format_bytes(point.up_bps),
                point.connections
            ),
            CLASH_THEME.text,
        ),
    ])]
}

fn register_traffic_action_hitboxes(app: &mut App, area: Rect) {
    let y = area.y + area.height.saturating_sub(2);
    let mut x = area.x + 2;
    for spec in action_registry::traffic_action_specs() {
        let Some(action) = hitbox_for_action_spec(spec) else {
            continue;
        };
        let width = action_button_width(app.action_button(spec));
        register_clamped_hitbox(app, x, y, width, area, action);
        x = x.saturating_add(width + 1);
    }
}

fn traffic_output_lines(app: &App) -> Vec<Line<'static>> {
    app.traffic_output
        .iter()
        .take(2)
        .map(|line| {
            Line::from(Span::styled(
                format!("  {}", trunc_str(line, 92)),
                CLASH_THEME.muted,
            ))
        })
        .collect()
}

fn traffic_status_lines(app: &App) -> Vec<Line<'static>> {
    let status = &app.traffic_status;
    let last = if status.last_sample.is_empty() {
        app.t(Msg::TrafficNever).to_string()
    } else {
        status.last_sample.clone()
    };
    let collector_interval = if status.collector_interval.is_empty() {
        "-"
    } else {
        status.collector_interval.as_str()
    };
    let collector = if status.collector_running {
        format!(
            "{} pid={} interval={} log={}",
            app.t(Msg::TrafficRunning),
            status.collector_pid,
            collector_interval,
            trunc_str(&status.collector_log, 42)
        )
    } else if status.collector_stale {
        format!(
            "{} pid={} log={}",
            app.t(Msg::TrafficStale),
            status.collector_pid,
            trunc_str(&status.collector_log, 42)
        )
    } else {
        format!(
            "{} log={}",
            app.t(Msg::TrafficStopped),
            trunc_str(&status.collector_log, 42)
        )
    };
    vec![
        Line::from(vec![
            Span::styled(
                format!("  {} ", app.t(Msg::TrafficStore)),
                CLASH_THEME.muted,
            ),
            Span::styled(trunc_str(&status.store_dir, 42), CLASH_THEME.text),
        ]),
        Line::from(vec![
            Span::styled(
                format!("  {} ", app.t(Msg::TrafficSamples)),
                CLASH_THEME.muted,
            ),
            Span::styled(
                format!(
                    "raw {}  10s {}  1m {}  {} {}  {} {}",
                    status.raw_samples,
                    status.rollup_10s,
                    status.rollup_1m,
                    app.t(Msg::TrafficTracked),
                    status.tracked_connections,
                    app.t(Msg::TrafficLast),
                    last
                ),
                CLASH_THEME.text,
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!("  {} ", app.t(Msg::TrafficCollector)),
                CLASH_THEME.muted,
            ),
            Span::styled(collector, CLASH_THEME.text),
            if status.collector_status_read_error.is_empty() {
                Span::raw("")
            } else {
                Span::styled(
                    format!(
                        "  {} {}",
                        app.t(Msg::TrafficStatusError),
                        trunc_str(&status.collector_status_read_error, 32)
                    ),
                    CLASH_THEME.warning,
                )
            },
        ]),
    ]
}

fn render_connections(frame: &mut Frame, area: Rect, app: &mut App) {
    fill_area(frame, area, CLASH_THEME.surface);

    let close_selected = action_registry::action_by_id("conn.close.selected")
        .map(|spec| app.action_button(spec))
        .unwrap_or("close selected");
    let close_all = action_registry::action_by_id("conn.close.all")
        .map(|spec| app.action_button(spec))
        .unwrap_or("close all");
    let header_text = format!(
        " {}: {} | c:{}  C:{} ",
        app.t(Msg::ConnectionsActive),
        app.connections.len(),
        close_selected,
        close_all
    );
    let (table_area, actions_area) = if area.height >= 8 {
        let chunks = Layout::vertical([Constraint::Min(5), Constraint::Length(3)]).split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    let mut rows: Vec<Vec<String>> = Vec::new();
    for conn in &app.connections {
        let host = conn
            .metadata
            .as_ref()
            .and_then(|m| m.host.as_ref())
            .cloned()
            .unwrap_or_default();
        let network = conn
            .metadata
            .as_ref()
            .and_then(|m| m.network.as_ref())
            .cloned()
            .unwrap_or_default();
        let chain = conn.chains.first().cloned().unwrap_or_default();
        rows.push(vec![host, network, chain]);
    }

    if rows.is_empty() {
        let msg = Paragraph::new(app.t(Msg::ConnectionsNoActive))
            .style(Style::default().fg(CLASH_THEME.muted));
        frame.render_widget(msg, table_area);
        if let Some(actions_area) = actions_area {
            render_connection_actions(frame, actions_area, app);
        }
        return;
    }

    let header = Row::new(vec![
        app.t(Msg::ConnectionsHost),
        app.t(Msg::ConnectionsType),
        app.t(Msg::ConnectionsChain),
    ])
    .style(Style::default().fg(CLASH_THEME.muted));
    let data_rows: Vec<Row> = rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let style = if i == app.connections_selected {
                Style::default()
                    .fg(CLASH_THEME.text)
                    .bg(CLASH_THEME.primary)
            } else {
                Style::default()
                    .fg(CLASH_THEME.text)
                    .bg(CLASH_THEME.surface)
            };
            Row::new(row.clone()).style(style)
        })
        .collect();

    let widths = [
        Constraint::Ratio(2, 5),
        Constraint::Length(10),
        Constraint::Ratio(2, 5),
    ];
    let table = Table::new(data_rows, widths)
        .header(header)
        .style(Style::default().bg(CLASH_THEME.surface));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLASH_THEME.border))
        .title(Span::styled(
            format!(" {} ", app.t(Msg::PageConnections)),
            Style::default().fg(CLASH_THEME.primary).bold(),
        ))
        .title_bottom(Span::styled(
            format!(" {} ", header_text),
            Style::default().fg(CLASH_THEME.muted),
        ));
    let inner = block.inner(table_area);
    frame.render_widget(block, table_area);
    register_table_row_hitboxes(app, inner, rows.len(), HitboxAction::SelectConnection);
    frame.render_stateful_widget(table, inner, &mut app.connections_table_state.clone());

    if let Some(actions_area) = actions_area {
        render_connection_actions(frame, actions_area, app);
    }
}

fn render_connection_actions(frame: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLASH_THEME.border))
        .title(Span::styled(
            format!(" {} ", app.t(Msg::ConnectionsActions)),
            Style::default().fg(CLASH_THEME.primary).bold(),
        ))
        .title_bottom(Span::styled(
            format!(" {} ", app.t(Msg::ConnectionsActionsHint)),
            Style::default().fg(CLASH_THEME.muted),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(
        Paragraph::new(action_specs_line(
            app,
            action_registry::connection_action_specs(),
        ))
        .style(Style::default().bg(CLASH_THEME.surface)),
        inner,
    );
    register_action_specs_hitboxes(app, area, action_registry::connection_action_specs());
}

fn render_logs(frame: &mut Frame, area: Rect, app: &mut App) {
    fill_area(frame, area, CLASH_THEME.surface);

    let (log_area, actions_area) = if area.height >= 8 {
        let chunks = Layout::vertical([Constraint::Min(5), Constraint::Length(3)]).split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };
    let max_lines = log_area.height.saturating_sub(2) as usize;
    let visible_logs = app.visible_logs();
    let start = if visible_logs.len() > max_lines {
        (app.log_scroll.min(visible_logs.len().saturating_sub(1)))
            .saturating_sub(max_lines.saturating_sub(1))
    } else {
        0
    };
    let end = (start + max_lines).min(visible_logs.len());

    let lines: Vec<Line> = if visible_logs.is_empty() {
        vec![Line::from(Span::styled(
            app.t(Msg::LogsEmpty),
            CLASH_THEME.muted,
        ))]
    } else {
        visible_logs[start..end]
            .iter()
            .map(|l| {
                let color = if log_line_rank(l) >= 3 {
                    CLASH_THEME.danger
                } else if log_line_rank(l) >= 2 {
                    CLASH_THEME.warning
                } else if log_line_rank(l) == 0 {
                    CLASH_THEME.muted
                } else {
                    CLASH_THEME.text
                };
                Line::from(Span::styled((*l).clone(), Style::default().fg(color)))
            })
            .collect()
    };

    let pause_str = if app.log_paused { "⏸" } else { "▶" };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLASH_THEME.border))
        .title(Span::styled(
            format!(" {} ", app.t(Msg::PageLogs)),
            Style::default().fg(CLASH_THEME.primary).bold(),
        ))
        .title_bottom(Span::styled(
            format!(
                " p:{}  f:{}+  c:clear  /:search  scroll:navigate ",
                pause_str,
                app.log_level.label()
            ),
            Style::default().fg(CLASH_THEME.muted),
        ));
    let inner = block.inner(log_area);
    frame.render_widget(block, log_area);
    app.hitboxes.register(inner, HitboxAction::ScrollLogs);
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(CLASH_THEME.surface)),
        inner,
    );
    if let Some(actions_area) = actions_area {
        render_log_actions(frame, actions_area, app);
    }
}

fn render_log_actions(frame: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLASH_THEME.border))
        .title(Span::styled(
            format!(" {} ", app.t(Msg::LogsActions)),
            Style::default().fg(CLASH_THEME.primary).bold(),
        ))
        .title_bottom(Span::styled(
            format!(" {} ", app.t(Msg::LogsActionsHint)),
            Style::default().fg(CLASH_THEME.muted),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(
        Paragraph::new(action_specs_line(app, action_registry::log_action_specs()))
            .style(Style::default().bg(CLASH_THEME.surface)),
        inner,
    );
    register_action_specs_hitboxes(app, area, action_registry::log_action_specs());
}

fn render_settings(frame: &mut Frame, area: Rect, app: &mut App) {
    fill_area(frame, area, CLASH_THEME.surface);
    let rows = Layout::vertical([
        Constraint::Length(5),
        Constraint::Length(5),
        Constraint::Length(5),
        Constraint::Min(5),
    ])
    .split(area);

    crate::widgets::card::Card::new(app.t(Msg::SettingsGeneral)).render(
        frame,
        rows[0],
        vec![
            Line::from(vec![
                Span::styled(
                    format!("  {} ", app.t(Msg::SettingsLanguage)),
                    CLASH_THEME.muted,
                ),
                Span::styled(app.ui_settings.language.label(), CLASH_THEME.text),
                Span::styled(
                    format!("  {} ", app.t(Msg::SettingsTheme)),
                    CLASH_THEME.muted,
                ),
                Span::styled(theme_label(&app.ui_settings.theme), CLASH_THEME.text),
                Span::styled(
                    format!("  {} ", app.t(Msg::SettingsRefreshInterval)),
                    CLASH_THEME.muted,
                ),
                Span::styled(app.ui_settings.refresh_interval_label(), CLASH_THEME.text),
            ]),
            Line::from(vec![
                Span::styled(
                    format!("  {} ", app.t(Msg::SettingsDefaultPage)),
                    CLASH_THEME.muted,
                ),
                Span::styled(app.default_page_label(), CLASH_THEME.text),
                Span::styled(
                    format!("  {} ", app.t(Msg::SettingsMouse)),
                    CLASH_THEME.muted,
                ),
                Span::styled(
                    if app.ui_settings.mouse_enabled {
                        app.t(Msg::CommonEnabled)
                    } else {
                        app.t(Msg::CommonDisabled)
                    },
                    CLASH_THEME.accent,
                ),
                Span::styled(
                    format!("  {} ", app.t(Msg::SettingsConfirmDanger)),
                    CLASH_THEME.muted,
                ),
                Span::styled(
                    if app.ui_settings.confirm_dangerous_actions {
                        app.t(Msg::CommonOn)
                    } else {
                        app.t(Msg::CommonOff)
                    },
                    CLASH_THEME.text,
                ),
            ]),
            action_specs_line(
                app,
                SETTINGS_GENERAL_ACTION_IDS
                    .iter()
                    .filter_map(|id| action_registry::action_by_id(id)),
            ),
        ],
    );
    render_settings_general_hitboxes(app, rows[0]);

    crate::widgets::card::Card::new(app.t(Msg::SettingsCoreApi)).render(
        frame,
        rows[1],
        vec![
            Line::from(vec![
                Span::styled(format!("  {} ", app.t(Msg::SettingsApi)), CLASH_THEME.muted),
                Span::styled(&app.config.api_url, CLASH_THEME.text),
            ]),
            Line::from(vec![
                Span::styled(
                    format!("  {} ", app.t(Msg::SettingsKernel)),
                    CLASH_THEME.muted,
                ),
                Span::styled(
                    if app.version.is_empty() {
                        app.t(Msg::SettingsNotConnected)
                    } else {
                        app.version.as_str()
                    },
                    CLASH_THEME.text,
                ),
            ]),
        ],
    );

    crate::widgets::card::Card::new(app.t(Msg::SettingsTraffic)).render(
        frame,
        rows[2],
        vec![
            Line::from(vec![
                Span::styled(
                    format!("  {} ", app.t(Msg::TrafficStore)),
                    CLASH_THEME.muted,
                ),
                Span::styled(
                    trunc_str(&app.traffic_status.store_dir, 48),
                    CLASH_THEME.text,
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    format!("  {} ", app.t(Msg::SettingsDefaultChart)),
                    CLASH_THEME.muted,
                ),
                Span::styled(
                    TrafficChartKind::from_setting_key(&app.ui_settings.traffic_default_chart)
                        .label(),
                    CLASH_THEME.text,
                ),
                Span::styled(
                    format!("  {} ", app.t(Msg::TrafficRange)),
                    CLASH_THEME.muted,
                ),
                Span::styled(
                    TrafficRange::from_setting_key(&app.ui_settings.traffic_default_range).label(),
                    CLASH_THEME.text,
                ),
                Span::styled(format!("  {} ", app.t(Msg::TrafficBy)), CLASH_THEME.muted),
                Span::styled(
                    TrafficDimension::from_setting_key(&app.ui_settings.traffic_default_dimension)
                        .label(),
                    CLASH_THEME.text,
                ),
            ]),
            action_specs_line(
                app,
                SETTINGS_TRAFFIC_ACTION_IDS
                    .iter()
                    .filter_map(|id| action_registry::action_by_id(id)),
            ),
        ],
    );
    render_settings_traffic_hitboxes(app, rows[2]);

    render_settings_actions(frame, rows[3], app);
}

const SETTINGS_GENERAL_ACTION_IDS: &[&str] = &[
    "settings.ui.language",
    "settings.ui.theme",
    "settings.ui.default_page",
    "settings.ui.refresh_interval",
    "settings.ui.mouse",
    "settings.ui.confirm",
];

const SETTINGS_TRAFFIC_ACTION_IDS: &[&str] = &[
    "settings.traffic.default_range",
    "settings.traffic.default_chart",
    "settings.traffic.default_dimension",
    "settings.traffic.prune_retention",
];

fn render_settings_general_hitboxes(app: &mut App, area: Rect) {
    render_settings_inline_action_hitboxes(app, area, SETTINGS_GENERAL_ACTION_IDS);
}

fn render_settings_traffic_hitboxes(app: &mut App, area: Rect) {
    render_settings_inline_action_hitboxes(app, area, SETTINGS_TRAFFIC_ACTION_IDS);
}

fn render_settings_inline_action_hitboxes(app: &mut App, area: Rect, action_ids: &[&str]) {
    if area.height < 5 || area.width < 12 {
        return;
    }
    let y = area.y + 3;
    let mut x = area.x + 3;
    for spec in action_ids
        .iter()
        .filter_map(|id| action_registry::action_by_id(id))
    {
        let Some(action) = hitbox_for_action_spec(spec) else {
            continue;
        };
        let width = action_button_width(app.action_button(spec));
        register_clamped_hitbox(app, x, y, width, area, action);
        x = x.saturating_add(width + 1);
    }
}

fn render_settings_actions(frame: &mut Frame, area: Rect, app: &mut App) {
    let mut lines = vec![
        settings_action_line(
            app,
            app.t(Msg::SettingsDiagnostics),
            SETTINGS_DIAGNOSTIC_ACTION_IDS,
        ),
        settings_action_line(app, app.t(Msg::SettingsConfig), SETTINGS_CONFIG_ACTION_IDS),
        settings_action_line(app, app.t(Msg::SettingsForms), SETTINGS_FORM_ACTION_IDS),
        settings_action_line(
            app,
            app.t(Msg::SettingsSecurity),
            SETTINGS_SECURITY_ACTION_IDS,
        ),
        settings_action_line(app, app.t(Msg::SettingsUpdates), SETTINGS_UPDATE_ACTION_IDS),
    ];
    if app.settings_output.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  {}", app.t(Msg::SettingsResultsPlaceholder)),
            CLASH_THEME.muted,
        )));
    } else {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  {}", app.t(Msg::SettingsLastResult)),
            CLASH_THEME.primary,
        )));
        for line in &app.settings_output {
            lines.push(Line::from(Span::styled(
                format!("  {}", line),
                CLASH_THEME.text,
            )));
        }
    }
    crate::widgets::card::Card::new(app.t(Msg::SettingsDiagnosticsUpdates))
        .render(frame, area, lines);
    register_settings_action_hitboxes(app, area);
}

const SETTINGS_DIAGNOSTIC_ACTION_IDS: &[&str] = &[
    "settings.doctor",
    "settings.config_doctor",
    "settings.proxy.test",
    "settings.version",
];
const SETTINGS_CONFIG_ACTION_IDS: &[&str] = &[
    "settings.config.view",
    "settings.config.raw",
    "settings.config.merge",
    "settings.config.autofix",
];
const SETTINGS_FORM_ACTION_IDS: &[&str] = &[
    "settings.config.set_ports",
    "settings.config.set_api",
    "settings.config.set_dns",
    "settings.config.set_lan",
];
const SETTINGS_SECURITY_ACTION_IDS: &[&str] = &[
    "settings.secret.status",
    "settings.secret.reveal",
    "settings.secret.set",
];
const SETTINGS_UPDATE_ACTION_IDS: &[&str] = &[
    "settings.geodata.update",
    "settings.geodata.version",
    "settings.api.upgrade",
    "settings.kernel.upgrade",
];

fn settings_action_line(app: &App, group: &'static str, action_ids: &[&str]) -> Line<'static> {
    let mut spans = vec![
        Span::styled(format!("  {:<11}", group), CLASH_THEME.muted),
        Span::styled(" ", CLASH_THEME.text),
    ];
    for spec in action_ids
        .iter()
        .filter_map(|id| action_registry::action_by_id(id))
    {
        let label = app.action_button(spec);
        spans.push(Span::styled(
            format!("[{}] ", label),
            Style::default().fg(action_fg(spec.danger)),
        ));
    }
    Line::from(spans)
}

fn register_settings_action_hitboxes(app: &mut App, area: Rect) {
    if area.height < 6 || area.width < 12 {
        return;
    }
    let rows: [(u16, &[&str]); 5] = [
        (1, SETTINGS_DIAGNOSTIC_ACTION_IDS),
        (2, SETTINGS_CONFIG_ACTION_IDS),
        (3, SETTINGS_FORM_ACTION_IDS),
        (4, SETTINGS_SECURITY_ACTION_IDS),
        (5, SETTINGS_UPDATE_ACTION_IDS),
    ];
    for (row_offset, action_ids) in rows {
        let y = area.y + row_offset;
        let mut x = area.x + 15;
        for spec in action_ids
            .iter()
            .filter_map(|id| action_registry::action_by_id(id))
        {
            let Some(action) = hitbox_for_action_spec(spec) else {
                continue;
            };
            let width = action_button_width(app.action_button(spec));
            register_clamped_hitbox(app, x, y, width, area, action);
            x = x.saturating_add(width + 1);
        }
    }
}

fn render_help(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLASH_THEME.primary).bg(CLASH_THEME.bg))
        .style(Style::default().bg(CLASH_THEME.bg))
        .title(Span::styled(
            format!(" {} ", app.t(Msg::HelpTitle)),
            Style::default().fg(CLASH_THEME.primary).bold(),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!("  {:<16}", app.t(Msg::HelpKey)),
                CLASH_THEME.primary,
            ),
            Span::styled(format!("{:<15}", app.t(Msg::HelpPage)), CLASH_THEME.primary),
            Span::styled(
                format!("{:<31}", app.t(Msg::HelpAction)),
                CLASH_THEME.primary,
            ),
            Span::styled(format!("{:<11}", app.t(Msg::HelpRisk)), CLASH_THEME.primary),
            Span::styled(app.t(Msg::HelpDescription), CLASH_THEME.primary),
        ]),
        Line::from(vec![
            Span::styled("  Tab/1-8        ", CLASH_THEME.primary),
            Span::styled(
                format!("{:<15}", app.t(Msg::CommonGlobal)),
                CLASH_THEME.muted,
            ),
            Span::styled(
                format!("{:<31}", app.t(Msg::HelpSwitchTabs)),
                CLASH_THEME.text,
            ),
            Span::styled(format!("{:<11}", app.t(Msg::CommonSafe)), CLASH_THEME.text),
            Span::styled(app.t(Msg::HelpSwitchTabs), CLASH_THEME.text),
        ]),
        Line::from(vec![
            Span::styled("  r/q/?           ", CLASH_THEME.primary),
            Span::styled(
                format!("{:<15}", app.t(Msg::CommonGlobal)),
                CLASH_THEME.muted,
            ),
            Span::styled(
                format!("{:<31}", app.t(Msg::HelpRefreshQuitHelp)),
                CLASH_THEME.text,
            ),
            Span::styled(format!("{:<11}", app.t(Msg::CommonSafe)), CLASH_THEME.text),
            Span::styled(app.t(Msg::HelpRefreshQuitHelp), CLASH_THEME.text),
        ]),
        Line::from(vec![
            Span::styled("  j/k/↑↓         ", CLASH_THEME.primary),
            Span::styled(
                format!("{:<15}", app.t(Msg::CommonGlobal)),
                CLASH_THEME.muted,
            ),
            Span::styled(
                format!("{:<31}", app.t(Msg::HelpNavigateLists)),
                CLASH_THEME.text,
            ),
            Span::styled(format!("{:<11}", app.t(Msg::CommonSafe)), CLASH_THEME.text),
            Span::styled(app.t(Msg::HelpNavigateLists), CLASH_THEME.text),
        ]),
    ];

    let max_registry_rows = inner.height.saturating_sub(6) as usize;
    for spec in action_registry::all_actions()
        .iter()
        .take(max_registry_rows)
    {
        lines.push(action_help_line(app, spec));
    }
    if action_registry::all_actions().len() > max_registry_rows {
        lines.push(Line::from(Span::styled(
            format!("  {}", app.t(Msg::HelpMoreActions)),
            CLASH_THEME.muted,
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!(
            "  {}: {} {}, {} {}.",
            app.t(Msg::HelpGenerated),
            action_registry::all_actions().len(),
            app.t(Msg::HelpActionsUnit),
            action_registry::cli_backed_enum_count(),
            app.t(Msg::HelpCliBackedEnumActions)
        ),
        CLASH_THEME.muted,
    )));
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().fg(CLASH_THEME.text)),
        inner,
    );
}

fn render_command_palette(frame: &mut Frame, area: Rect, app: &App) {
    let popup = centered_rect(area, 74, 58);
    fill_area(frame, popup, CLASH_THEME.bg);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLASH_THEME.primary))
        .title(Span::styled(
            format!(" {} ", app.t(Msg::CommandPaletteTitle)),
            Style::default().fg(CLASH_THEME.primary).bold(),
        ))
        .title_bottom(Span::styled(
            format!(" {} ", app.t(Msg::CommandPaletteHint)),
            Style::default().fg(CLASH_THEME.muted),
        ));
    let inner = block.inner(popup).inner(Margin {
        vertical: 1,
        horizontal: 2,
    });
    frame.render_widget(block, popup);

    let matches = app.command_palette_matches();
    let max_rows = inner.height.saturating_sub(3) as usize;
    let selected = app
        .command_selected_idx
        .min(matches.len().saturating_sub(1));
    let first = selected.saturating_sub(max_rows.saturating_sub(1));

    let mut lines = vec![Line::from(vec![
        Span::styled("> ", CLASH_THEME.primary),
        Span::styled(&app.command_query, CLASH_THEME.text),
        Span::styled(
            format!("  {}", app.t(Msg::CommandPaletteSearchHint)),
            CLASH_THEME.muted,
        ),
    ])];
    lines.push(Line::from(""));

    if matches.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("  {}", app.t(Msg::CommandPaletteNoMatch)),
            CLASH_THEME.warning,
        )));
    } else {
        for (row_idx, spec) in matches.iter().skip(first).take(max_rows).enumerate() {
            let idx = first + row_idx;
            let marker = if idx == selected { ">" } else { " " };
            let style = if idx == selected {
                Style::default().fg(CLASH_THEME.bg).bg(CLASH_THEME.primary)
            } else {
                Style::default().fg(CLASH_THEME.text)
            };
            let danger = app.action_danger_label(spec.danger);
            let summary = format!(
                "{} {:<24} {:<11} {:<9} {}",
                marker,
                trunc_str(app.action_label(spec), 23),
                trunc_str(app.t(spec.page.msg()), 10),
                danger,
                trunc_str(&spec.executor.command(), 48)
            );
            lines.push(Line::from(Span::styled(summary, style)));
        }
    }

    frame.render_widget(
        Paragraph::new(lines).style(Style::default().fg(CLASH_THEME.text)),
        inner,
    );
}

fn action_help_line(app: &App, spec: &action_registry::ActionSpec) -> Line<'static> {
    let page = app.t(spec.page.msg());
    let label = app.action_label(spec);
    let description = app.action_desc(spec);
    let danger_style = match spec.danger {
        ActionDanger::Dangerous => Style::default().fg(CLASH_THEME.danger),
        ActionDanger::Confirm | ActionDanger::Sensitive => Style::default().fg(CLASH_THEME.warning),
        ActionDanger::Safe => Style::default().fg(CLASH_THEME.text),
    };
    Line::from(vec![
        Span::styled(
            format!("  {:<14}", trunc_str(spec.shortcut, 14)),
            CLASH_THEME.primary,
        ),
        Span::styled(format!("{:<15}", trunc_str(page, 14)), CLASH_THEME.muted),
        Span::styled(format!("{:<31}", trunc_str(label, 30)), CLASH_THEME.text),
        Span::styled(
            format!("{:<11}", app.action_danger_label(spec.danger)),
            danger_style,
        ),
        Span::styled(
            trunc_str(
                &format!("{} · {}", description, spec.executor.command()),
                72,
            ),
            CLASH_THEME.text,
        ),
    ])
}

fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    fill_area(frame, area, CLASH_THEME.bg);
    let error_text = app.error_msg.as_deref().unwrap_or("");
    let search_info = if app.search_active {
        format!(" [/] {}: {} | ", app.t(Msg::StatusSearch), app.search_query)
    } else {
        String::new()
    };
    let mode_info = if app.tab == Tab::Proxies {
        format!(
            "{}: {} | {}: {} | ",
            app.t(Msg::StatusMode),
            app.proxy_mode_str,
            app.t(Msg::StatusSort),
            if app.sort_mode {
                app.t(Msg::SortDelay)
            } else {
                app.t(Msg::SortName)
            }
        )
    } else {
        String::new()
    };
    let status = format!(
        " {}{}[q] {}  [tab] {}  [r] {}  [?] {}    {}",
        search_info,
        mode_info,
        app.t(Msg::StatusQuit),
        app.t(Msg::StatusSwitch),
        app.t(Msg::StatusRefresh),
        app.t(Msg::PageHelp),
        if app.error_msg.is_some() {
            error_text
        } else {
            app.status_msg.as_deref().unwrap_or("")
        }
    );
    let color = if app.error_msg.is_some() {
        CLASH_THEME.danger
    } else {
        CLASH_THEME.muted
    };
    let line = Line::from(Span::styled(status, Style::default().fg(color)));
    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(CLASH_THEME.bg)),
        area,
    );
}

fn fill_area(frame: &mut Frame, area: Rect, bg: Color) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let line_str = " ".repeat(area.width as usize);
    for y in 0..area.height {
        frame
            .buffer_mut()
            .set_string(area.x, area.y + y, &line_str, Style::default().bg(bg));
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

fn scaled_bar(value: u64, max: u64, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let filled = ((value as f64 / max.max(1) as f64) * width as f64).round() as usize;
    let filled = filled.clamp(1, width);
    "█".repeat(filled)
}

fn traffic_summary(points: &[TrafficPoint]) -> (u64, u64, u64, u64) {
    let mut down_total = 0_u64;
    let mut up_total = 0_u64;
    let mut down_rate = 0_u64;
    let mut up_rate = 0_u64;
    for point in points {
        down_total = down_total.saturating_add(point.download_delta);
        up_total = up_total.saturating_add(point.upload_delta);
        down_rate = down_rate.max(point.down_bps);
        up_rate = up_rate.max(point.up_bps);
    }
    (down_total, up_total, down_rate, up_rate)
}

fn profile_interval_label(profile: &ProfileEntry) -> String {
    if profile.update_enabled == Some(false) {
        return "off".into();
    }
    if !profile.update_interval.is_empty() {
        return profile.update_interval.clone();
    }
    if !profile.interval.is_empty() {
        return profile.interval.clone();
    }
    if profile.url.starts_with("file://") {
        "off".into()
    } else {
        "12h".into()
    }
}

fn trunc_str(value: &str, max: usize) -> String {
    let count = value.chars().count();
    if count <= max {
        return value.to_string();
    }
    if max <= 1 {
        return "…".into();
    }
    let mut out: String = value.chars().take(max - 1).collect();
    out.push('…');
    out
}

fn read_os_info() -> String {
    if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
        for line in content.lines() {
            if let Some(rest) = line.strip_prefix("PRETTY_NAME=") {
                return rest.trim_matches('"').to_string();
            }
        }
    }
    "Linux".into()
}

fn log_line_rank(line: &str) -> u8 {
    match line
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase()
        .as_str()
    {
        "ERROR" => 3,
        "WARN" | "WARNING" => 2,
        "DEBUG" => 0,
        _ => 1,
    }
}

fn mode_display(mode: &str) -> String {
    match mode.to_ascii_lowercase().as_str() {
        "global" => "Global".into(),
        "direct" => "Direct".into(),
        _ => "Rule".into(),
    }
}

fn next_mode(mode: &str) -> String {
    match mode_display(mode).as_str() {
        "Rule" => "Global".into(),
        "Global" => "Direct".into(),
        _ => "Rule".into(),
    }
}

fn command_output_lines(output: &str) -> Vec<String> {
    let mut lines: Vec<String> = output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| trunc_str(line, 120))
        .collect();
    if lines.len() > 8 {
        lines = lines[lines.len() - 8..].to_vec();
    }
    lines
}

fn redact_sensitive_output(output: &str) -> String {
    output
        .lines()
        .map(redact_sensitive_line)
        .collect::<Vec<_>>()
        .join("\n")
}

fn redact_sensitive_line(line: &str) -> String {
    let lower = line.to_ascii_lowercase();
    if let Some(pos) = lower.find("\"secret\"") {
        if let Some(colon) = line[pos..].find(':') {
            let split = pos + colon + 1;
            return format!("{} ********", &line[..split]);
        }
    }
    if let Some(pos) = lower.find("secret:") {
        let split = pos + "secret:".len();
        return format!("{} ********", &line[..split]);
    }
    line.to_string()
}

fn initial_tab_for_profiles(profile_count: usize, default_page: &str) -> Tab {
    if !is_auto_default_page(default_page) {
        if let Some(tab) = Tab::from_setting_key(default_page) {
            return tab;
        }
    }
    if profile_count == 0 {
        Tab::Subscriptions
    } else {
        Tab::Proxies
    }
}

fn next_default_page(current: &str) -> String {
    if is_auto_default_page(current) {
        return Tab::Subscriptions.setting_key().into();
    }
    match Tab::from_setting_key(current) {
        Some(Tab::Help) | None => "auto".into(),
        Some(tab) => tab.next().setting_key().to_string(),
    }
}

fn is_auto_default_page(raw: &str) -> bool {
    raw.trim().eq_ignore_ascii_case("auto")
}

#[cfg(test)]
mod tests {
    use super::{
        action_button_width, command_output_lines, hitbox_for_action_spec,
        initial_tab_for_profiles, log_line_rank, mode_display, next_default_page, next_mode,
        parse_config_set_api_args, parse_config_set_ports_args, parse_geodata_update_version_args,
        parse_traffic_prune_retention_args, profile_interval_label, redact_sensitive_output,
        register_subscription_action_hitboxes, register_subscription_form_hitboxes,
        subscription_action_buttons, traffic_line_chart_lines, traffic_locked_bucket_lines,
        traffic_status_lines, traffic_summary, traffic_window_bounds, App, LogLevelFilter,
        SettingsPromptKind, SubscriptionAddForm, SubscriptionEditField, SubscriptionEditForm,
        SubscriptionPrompt, TrafficChartKind, TrafficDimension, TrafficRange,
    };
    use crate::action_registry;
    use crate::api::{ProfileEntry, ProxyInfo, TrafficPoint, TrafficStatus};
    use crate::config::Config;
    use crate::event::DataEvent;
    use crate::mouse::{HitboxAction, NetworkAction, SettingsAction, TrafficAction};
    use crate::settings::LanguageSetting;
    use crate::widgets::tab_bar::Tab;
    use crossterm::event::{MouseButton, MouseEventKind};
    use ratatui::layout::Rect;
    use std::sync::mpsc;

    fn test_app(rt: &tokio::runtime::Runtime) -> App {
        let (tx, _rx) = mpsc::channel();
        App::new(
            Config {
                api_url: "http://127.0.0.1:9090".into(),
                api_key: String::new(),
            },
            rt.handle().clone(),
            tx,
        )
    }

    fn selector_proxy(now: &str, all: Vec<&str>) -> ProxyInfo {
        ProxyInfo {
            proxy_type: "Selector".into(),
            now: Some(now.into()),
            all: Some(all.into_iter().map(String::from).collect()),
            history: None,
            name: None,
        }
    }

    #[test]
    fn tab_order_matches_authoritative_eight_page_model() {
        assert_eq!(
            Tab::all(),
            &[
                Tab::Subscriptions,
                Tab::Proxies,
                Tab::Connections,
                Tab::Traffic,
                Tab::Network,
                Tab::Logs,
                Tab::Settings,
                Tab::Help,
            ]
        );
        assert_eq!(Tab::Subscriptions.next(), Tab::Proxies);
        assert_eq!(Tab::Traffic.next(), Tab::Network);
        assert_eq!(Tab::Help.next(), Tab::Subscriptions);
    }

    #[test]
    fn startup_default_page_prefers_subscriptions_until_profiles_exist() {
        assert_eq!(initial_tab_for_profiles(0, "auto"), Tab::Subscriptions);
        assert_eq!(initial_tab_for_profiles(1, "auto"), Tab::Proxies);
        assert_eq!(initial_tab_for_profiles(1, "Auto"), Tab::Proxies);
        assert_eq!(initial_tab_for_profiles(0, "traffic"), Tab::Traffic);
        assert_eq!(initial_tab_for_profiles(3, "network"), Tab::Network);
        assert_eq!(initial_tab_for_profiles(3, "bogus"), Tab::Proxies);
    }

    #[test]
    fn default_page_setting_cycles_through_auto_and_tabs() {
        assert_eq!(next_default_page("auto"), "subscriptions");
        assert_eq!(next_default_page("subscriptions"), "proxies");
        assert_eq!(next_default_page("help"), "auto");
        assert_eq!(next_default_page("unknown"), "auto");
    }

    #[test]
    fn command_output_lines_trim_empty_and_keep_recent_lines() {
        let input = "\n  first  \n\nsecond\nthird\nfourth\nfifth\nsixth\nseventh\neighth\nninth\n";
        let lines = command_output_lines(input);
        assert_eq!(lines.len(), 8);
        assert_eq!(lines.first().unwrap(), "second");
        assert_eq!(lines.last().unwrap(), "ninth");
    }

    #[test]
    fn settings_output_redacts_secret_like_lines() {
        let redacted = redact_sensitive_output("secret: abc\n\"secret\": \"abc\"\napi: ok");
        assert!(redacted.contains("secret: ********"));
        assert!(redacted.contains("\"secret\": ********"));
        assert!(!redacted.contains("abc"));
        assert!(redacted.contains("api: ok"));
    }

    #[test]
    fn traffic_summary_sums_totals_and_tracks_peak_rates() {
        let points = vec![
            TrafficPoint {
                download_delta: 10,
                upload_delta: 1,
                down_bps: 100,
                up_bps: 20,
                ..TrafficPoint::default()
            },
            TrafficPoint {
                download_delta: 20,
                upload_delta: 4,
                down_bps: 80,
                up_bps: 30,
                ..TrafficPoint::default()
            },
        ];
        assert_eq!(traffic_summary(&points), (30, 5, 100, 30));
    }

    #[test]
    fn traffic_line_chart_uses_available_vertical_space() {
        let base = chrono::DateTime::parse_from_rfc3339("2026-07-02T12:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let points: Vec<TrafficPoint> = (0..20)
            .map(|idx| TrafficPoint {
                ts: base + chrono::Duration::minutes(idx),
                download_delta: (idx as u64 + 1) * 10,
                upload_delta: (20 - idx as u64) * 5,
                ..TrafficPoint::default()
            })
            .collect();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let app = test_app(&rt);
        let lines = traffic_line_chart_lines(&app, &points, 30, 12);
        assert_eq!(lines.len(), 14);
        let rendered = lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(rendered.contains("download"));
        assert!(rendered.contains("upload"));
        assert!(rendered.contains("12:00"));
        assert!(rendered.contains("12:19"));
    }

    #[test]
    fn traffic_chart_mouse_pans_and_locks_time_buckets() {
        assert_eq!(traffic_window_bounds(20, 5, 0), (15, 20));
        assert_eq!(traffic_window_bounds(20, 5, 3), (12, 17));
        assert_eq!(traffic_window_bounds(20, 5, 99), (0, 5));

        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);
        app.tab = Tab::Traffic;
        let base = chrono::DateTime::parse_from_rfc3339("2026-07-02T12:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        app.traffic_points = (0..20)
            .map(|idx| TrafficPoint {
                ts: base + chrono::Duration::minutes(idx),
                download_delta: (idx as u64 + 1) * 100,
                upload_delta: (idx as u64 + 1) * 10,
                down_bps: (idx as u64 + 1) * 1000,
                up_bps: (idx as u64 + 1) * 100,
                connections: idx as usize,
                ..TrafficPoint::default()
            })
            .collect();
        app.hitboxes
            .register(Rect::new(0, 0, 80, 12), HitboxAction::ScrollTrafficChart);
        app.hitboxes.register(
            Rect::new(20, 0, 1, 12),
            HitboxAction::SelectTrafficBucket(12),
        );

        app.handle_mouse_event(MouseEventKind::ScrollDown, 2, 2);
        assert_eq!(app.traffic_window_offset, 3);
        app.handle_mouse_event(MouseEventKind::ScrollUp, 2, 2);
        assert_eq!(app.traffic_window_offset, 0);

        app.handle_mouse_event(MouseEventKind::Down(MouseButton::Left), 20, 2);
        assert_eq!(app.traffic_locked_bucket, Some(12));
        let locked = traffic_locked_bucket_lines(&app)
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(locked.contains("12:12:00"));
        assert!(locked.contains("conn 12"));

        app.handle_mouse_event(MouseEventKind::Down(MouseButton::Left), 2, 2);
        assert_eq!(app.traffic_locked_bucket, None);
    }

    #[test]
    fn traffic_controls_map_to_cli_query_arguments() {
        assert_eq!(TrafficRange::LastHour.range_arg(), "1h");
        assert_eq!(TrafficRange::LastHour.step_arg(), "10s");
        assert_eq!(TrafficRange::Day.range_arg(), "24h");
        assert_eq!(TrafficRange::Day.step_arg(), "1m");
        assert_eq!(TrafficRange::Week.range_arg(), "168h");
        assert_eq!(TrafficDimension::Route.arg(), "route");
        assert_eq!(TrafficDimension::Node.arg(), "node");
        assert_eq!(TrafficDimension::Network.arg(), "network");
    }

    #[test]
    fn traffic_controls_cycle_predictably() {
        assert_eq!(TrafficRange::LastHour.next(), TrafficRange::SixHours);
        assert_eq!(TrafficRange::LastHour.prev(), TrafficRange::Week);
        assert_eq!(TrafficChartKind::Line.next(), TrafficChartKind::Bar);
        assert_eq!(TrafficChartKind::Bar.next(), TrafficChartKind::Line);
        assert_eq!(TrafficDimension::Route.next(), TrafficDimension::Rule);
        assert_eq!(TrafficDimension::Network.next(), TrafficDimension::Route);
    }

    #[test]
    fn traffic_default_settings_map_to_stable_keys() {
        assert_eq!(TrafficRange::from_setting_key("1h"), TrafficRange::LastHour);
        assert_eq!(TrafficRange::from_setting_key("168h"), TrafficRange::Week);
        assert_eq!(TrafficRange::from_setting_key("bogus"), TrafficRange::Day);
        assert_eq!(TrafficRange::Week.setting_key(), "7d");

        assert_eq!(
            TrafficChartKind::from_setting_key("bar"),
            TrafficChartKind::Bar
        );
        assert_eq!(
            TrafficChartKind::from_setting_key("unknown"),
            TrafficChartKind::Line
        );
        assert_eq!(TrafficChartKind::Line.setting_key(), "line");

        assert_eq!(
            TrafficDimension::from_setting_key("process"),
            TrafficDimension::Process
        );
        assert_eq!(
            TrafficDimension::from_setting_key("unknown"),
            TrafficDimension::Route
        );
        assert_eq!(TrafficDimension::Network.setting_key(), "network");
    }

    #[test]
    fn registry_driven_visible_actions_have_hitbox_dispatch() {
        let settings_ids = [
            super::SETTINGS_GENERAL_ACTION_IDS,
            super::SETTINGS_TRAFFIC_ACTION_IDS,
            super::SETTINGS_DIAGNOSTIC_ACTION_IDS,
            super::SETTINGS_CONFIG_ACTION_IDS,
            super::SETTINGS_FORM_ACTION_IDS,
            super::SETTINGS_SECURITY_ACTION_IDS,
            super::SETTINGS_UPDATE_ACTION_IDS,
        ];

        for id in settings_ids.into_iter().flatten() {
            let spec = action_registry::action_by_id(id).expect("registered settings action");
            assert!(
                hitbox_for_action_spec(spec).is_some(),
                "{id} must dispatch to a hitbox action"
            );
        }

        for spec in action_registry::traffic_action_specs() {
            assert!(
                hitbox_for_action_spec(spec).is_some(),
                "{} must dispatch to a hitbox action",
                spec.id
            );
        }
        for spec in action_registry::proxy_action_specs()
            .chain(action_registry::connection_action_specs())
            .chain(action_registry::log_action_specs())
        {
            assert!(
                hitbox_for_action_spec(spec).is_some(),
                "{} must dispatch to a hitbox action",
                spec.id
            );
        }
    }

    #[test]
    fn command_palette_filters_registry_actions() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);

        app.open_command_palette();
        for c in "traffic reset".chars() {
            app.push_command_palette_char(c);
        }

        let matches = app.command_palette_matches();
        assert!(matches.iter().any(|spec| spec.id == "traffic.reset"));
    }

    #[test]
    fn command_palette_filters_translated_action_text() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);
        app.ui_settings.language = LanguageSetting::ZhCn;

        app.open_command_palette();
        for c in "开启 TUN".chars() {
            app.push_command_palette_char(c);
        }

        let matches = app.command_palette_matches();
        assert!(matches.iter().any(|spec| spec.id == "network.tun.on"));
    }

    #[test]
    fn command_palette_submit_can_switch_pages() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);

        app.open_command_palette();
        for c in "nav.network".chars() {
            app.push_command_palette_char(c);
        }
        app.submit_command_palette();

        assert_eq!(app.tab, Tab::Network);
        assert!(!app.command_palette_active());
    }

    #[test]
    fn command_palette_submit_can_open_settings_forms() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);

        app.open_command_palette();
        for c in "settings.config.set_dns".chars() {
            app.push_command_palette_char(c);
        }
        app.submit_command_palette();

        assert_eq!(
            app.settings_prompt.as_ref().map(|prompt| prompt.kind),
            Some(SettingsPromptKind::ConfigSetDnsMode)
        );

        app.close_command_palette();
        app.cancel_settings_prompt();
        app.open_command_palette();
        for c in "settings.traffic.prune_retention".chars() {
            app.push_command_palette_char(c);
        }
        app.submit_command_palette();

        assert_eq!(
            app.settings_prompt.as_ref().map(|prompt| prompt.kind),
            Some(SettingsPromptKind::TrafficPruneRetention)
        );

        app.close_command_palette();
        app.cancel_settings_prompt();
        app.open_command_palette();
        for c in "settings.geodata.version".chars() {
            app.push_command_palette_char(c);
        }
        app.submit_command_palette();

        assert_eq!(
            app.settings_prompt.as_ref().map(|prompt| prompt.kind),
            Some(SettingsPromptKind::GeodataUpdateVersion)
        );
    }

    #[test]
    fn traffic_status_lines_include_store_counts_and_last_sample() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);
        app.traffic_status = TrafficStatus {
            store_dir: "/tmp/clashctl/traffic".into(),
            raw_samples: 3,
            rollup_10s: 2,
            rollup_1m: 1,
            last_sample: "2026-07-02 12:00:00".into(),
            tracked_connections: 4,
            collector_running: true,
            collector_pid: 1234,
            collector_interval: "1s".into(),
            collector_log: "/tmp/clashctl/traffic/collector.log".into(),
            ..TrafficStatus::default()
        };
        let rendered = traffic_status_lines(&app)
            .into_iter()
            .map(|line| {
                line.spans
                    .into_iter()
                    .map(|span| span.content.into_owned())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(rendered.contains("/tmp/clashctl/traffic"));
        assert!(rendered.contains("raw 3"));
        assert!(rendered.contains("10s 2"));
        assert!(rendered.contains("1m 1"));
        assert!(rendered.contains("tracked 4"));
        assert!(rendered.contains("2026-07-02 12:00:00"));
        assert!(rendered.contains("running pid=1234"));
        assert!(rendered.contains("interval=1s"));
    }

    #[test]
    fn traffic_export_event_reports_saved_path_or_error() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);

        app.apply_data_event(DataEvent::TrafficExport(Ok("/tmp/traffic.csv".into())));
        assert_eq!(
            app.status_msg.as_deref(),
            Some("traffic exported: /tmp/traffic.csv")
        );
        assert!(app.error_msg.is_none());

        app.apply_data_event(DataEvent::TrafficExport(Err("disk full".into())));
        assert_eq!(
            app.error_msg.as_deref(),
            Some("Traffic export failed: disk full")
        );
    }

    #[test]
    fn traffic_action_event_reports_output_or_error() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);

        app.apply_data_event(DataEvent::TrafficActionResult(
            "traffic sample".into(),
            Ok("[+] sampled".into()),
        ));
        assert_eq!(
            app.status_msg.as_deref(),
            Some("traffic action completed: traffic sample")
        );
        assert_eq!(app.traffic_output, vec!["[+] sampled"]);

        app.apply_data_event(DataEvent::TrafficActionResult(
            "traffic sample".into(),
            Err("kernel unavailable".into()),
        ));
        assert_eq!(
            app.error_msg.as_deref(),
            Some("Traffic action failed: traffic sample: kernel unavailable")
        );
        assert_eq!(app.traffic_output, vec!["kernel unavailable"]);
    }

    #[test]
    fn settings_result_event_reports_output_or_error() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);

        app.apply_data_event(DataEvent::SettingsResult(
            SettingsAction::Doctor,
            Ok("[+] ok\nsecret: clear".into()),
        ));
        assert_eq!(
            app.status_msg.as_deref(),
            Some("settings action completed: doctor")
        );
        assert!(app
            .settings_output
            .iter()
            .any(|line| line.contains("[+] ok")));
        assert!(app
            .settings_output
            .iter()
            .any(|line| line.contains("secret: ********")));

        app.apply_data_event(DataEvent::SettingsResult(
            SettingsAction::Doctor,
            Err("failed".into()),
        ));
        assert_eq!(
            app.error_msg.as_deref(),
            Some("Settings action failed: doctor: failed")
        );
    }

    #[test]
    fn settings_secret_reveal_event_allows_explicit_secret_output() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);

        app.apply_data_event(DataEvent::SettingsResult(
            SettingsAction::SecretReveal,
            Ok("current secret: visible-secret".into()),
        ));

        assert!(app
            .settings_output
            .iter()
            .any(|line| line.contains("visible-secret")));
    }

    #[test]
    fn settings_command_result_redacts_secret_set_output() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);

        app.apply_data_event(DataEvent::SettingsCommandResult(
            "Set API secret".into(),
            true,
            Ok("secret: new-secret".into()),
        ));

        assert!(app
            .settings_output
            .iter()
            .any(|line| line.contains("secret: ********")));
        assert!(!app
            .settings_output
            .iter()
            .any(|line| line.contains("new-secret")));
    }

    #[test]
    fn subscription_output_event_keeps_multiline_output() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);

        app.apply_data_event(DataEvent::SubscriptionOutputResult(
            "log".into(),
            Ok("line one\nline two".into()),
        ));

        assert_eq!(
            app.status_msg.as_deref(),
            Some("subscription action completed: log")
        );
        assert_eq!(app.subscription_output, vec!["line one", "line two"]);
    }

    #[test]
    fn dangerous_network_action_opens_confirmation() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);
        app.tab = Tab::Network;

        app.run_network_action(NetworkAction::Stop);

        let pending = app.pending_confirmation.as_ref().unwrap();
        assert!(pending.title.contains("stop"));
        assert!(pending.message.contains("clashctl stop"));

        app.cancel_pending_action();
        assert!(app.pending_confirmation.is_none());
        assert_eq!(app.status_msg.as_deref(), Some("action cancelled"));
    }

    #[test]
    fn dangerous_settings_action_opens_confirmation() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);
        app.tab = Tab::Settings;

        app.run_settings_action(SettingsAction::KernelUpgrade);

        let pending = app.pending_confirmation.as_ref().unwrap();
        assert!(pending.title.contains("kernel upgrade"));
        assert!(pending.message.contains("clashctl upgrade-kernel"));
    }

    #[test]
    fn secret_set_prompt_opens_redacted_confirmation() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);
        app.tab = Tab::Settings;

        app.begin_secret_set();
        app.push_settings_prompt_char('s');
        app.push_settings_prompt_char('3');
        app.push_settings_prompt_char('c');
        app.submit_settings_prompt();

        assert!(app.settings_prompt.is_none());
        let pending = app.pending_confirmation.as_ref().unwrap();
        assert!(pending.title.contains("Set secret"));
        assert!(pending.message.contains("clashctl secret ********"));
        assert!(!pending.message.contains("s3c"));
        match &pending.action {
            super::PendingAction::SettingsCommand {
                label,
                args,
                redact_output,
            } => {
                assert_eq!(label, "Set API secret");
                assert_eq!(args, &vec!["secret".to_string(), "s3c".to_string()]);
                assert!(*redact_output);
            }
            other => panic!("unexpected pending action: {other:?}"),
        }
    }

    #[test]
    fn config_set_ports_prompt_parses_cli_arguments() {
        assert_eq!(
            parse_config_set_ports_args("7897").unwrap(),
            vec![
                "config".to_string(),
                "set-port".to_string(),
                "7897".to_string()
            ]
        );
        assert_eq!(
            parse_config_set_ports_args("mixed=7897 http=7898 socks=7899").unwrap(),
            vec![
                "config".to_string(),
                "set-port".to_string(),
                "7897".to_string(),
                "--http".to_string(),
                "7898".to_string(),
                "--socks".to_string(),
                "7899".to_string()
            ]
        );
        assert!(parse_config_set_ports_args("mixed=0").is_err());
        assert!(parse_config_set_ports_args("mixed=7897 redir=7892").is_err());
    }

    #[test]
    fn config_set_api_prompt_parses_controller_secret_and_unsafe_flag() {
        assert_eq!(
            parse_config_set_api_args("127.0.0.1:9090").unwrap(),
            vec![
                "config".to_string(),
                "set-api".to_string(),
                "127.0.0.1:9090".to_string()
            ]
        );
        assert_eq!(
            parse_config_set_api_args("controller=0.0.0.0:9090 secret=s3c allow-unsafe=true")
                .unwrap(),
            vec![
                "config".to_string(),
                "set-api".to_string(),
                "0.0.0.0:9090".to_string(),
                "--secret".to_string(),
                "s3c".to_string(),
                "--allow-unsafe".to_string(),
            ]
        );
        assert_eq!(
            parse_config_set_api_args("0.0.0.0:9090 --secret s3c --allow-unsafe").unwrap(),
            vec![
                "config".to_string(),
                "set-api".to_string(),
                "0.0.0.0:9090".to_string(),
                "--secret".to_string(),
                "s3c".to_string(),
                "--allow-unsafe".to_string(),
            ]
        );
        assert!(parse_config_set_api_args("secret=s3c").is_err());
        assert!(parse_config_set_api_args("127.0.0.1:9090 allow-unsafe=maybe").is_err());
        assert!(parse_config_set_api_args("127.0.0.1:9090 unknown=value").is_err());
    }

    #[test]
    fn traffic_prune_retention_prompt_parses_duration_windows() {
        assert_eq!(
            parse_traffic_prune_retention_args("24h").unwrap(),
            vec![
                "traffic".to_string(),
                "prune".to_string(),
                "--retention".to_string(),
                "24h".to_string(),
            ]
        );
        assert_eq!(
            parse_traffic_prune_retention_args("raw=24h rollup-10s=7d rollup-1m=90d").unwrap(),
            vec![
                "traffic".to_string(),
                "prune".to_string(),
                "--retention".to_string(),
                "24h".to_string(),
                "--rollup-10s-retention".to_string(),
                "168h".to_string(),
                "--rollup-1m-retention".to_string(),
                "2160h".to_string(),
            ]
        );
        assert!(parse_traffic_prune_retention_args("").is_err());
        assert!(parse_traffic_prune_retention_args("0h").is_err());
        assert!(parse_traffic_prune_retention_args("raw=24h yearly=1y").is_err());
        assert!(parse_traffic_prune_retention_args("1.5h").is_err());
    }

    #[test]
    fn geodata_update_version_prompt_builds_version_flag() {
        assert_eq!(
            parse_geodata_update_version_args("latest").unwrap(),
            vec![
                "geodata".to_string(),
                "update".to_string(),
                "--version".to_string(),
                "latest".to_string(),
            ]
        );
        assert_eq!(
            parse_geodata_update_version_args("v2025.01.01").unwrap(),
            vec![
                "geodata".to_string(),
                "update".to_string(),
                "--version".to_string(),
                "v2025.01.01".to_string(),
            ]
        );
        assert!(parse_geodata_update_version_args("").is_err());
        assert!(parse_geodata_update_version_args("latest extra").is_err());
    }

    #[test]
    fn config_set_api_prompt_redacts_secret_confirmation() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);
        app.tab = Tab::Settings;

        app.begin_settings_prompt(SettingsPromptKind::ConfigSetApi);
        for c in "controller=0.0.0.0:9090 secret=s3c allow-unsafe=true".chars() {
            app.push_settings_prompt_char(c);
        }
        app.submit_settings_prompt();

        assert!(app.settings_prompt.is_none());
        let pending = app.pending_confirmation.as_ref().unwrap();
        assert!(pending.title.contains("Set API"));
        assert!(pending
            .message
            .contains("clashctl config set-api 0.0.0.0:9090 --secret ******** --allow-unsafe"));
        assert!(!pending.message.contains("s3c"));
        match &pending.action {
            super::PendingAction::SettingsCommand {
                label,
                args,
                redact_output,
            } => {
                assert_eq!(label, "Set API controller");
                assert_eq!(
                    args,
                    &vec![
                        "config".to_string(),
                        "set-api".to_string(),
                        "0.0.0.0:9090".to_string(),
                        "--secret".to_string(),
                        "s3c".to_string(),
                        "--allow-unsafe".to_string(),
                    ]
                );
                assert!(*redact_output);
            }
            other => panic!("unexpected pending action: {other:?}"),
        }
    }

    #[test]
    fn config_set_prompt_opens_confirmation_with_cli_args() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);
        app.tab = Tab::Settings;

        app.begin_settings_prompt(SettingsPromptKind::ConfigSetPorts);
        for c in "mixed=7897 http=7898 socks=7899".chars() {
            app.push_settings_prompt_char(c);
        }
        app.submit_settings_prompt();

        assert!(app.settings_prompt.is_none());
        let pending = app.pending_confirmation.as_ref().unwrap();
        assert!(pending.title.contains("Set ports"));
        assert!(pending
            .message
            .contains("clashctl config set-port 7897 --http 7898 --socks 7899"));
        match &pending.action {
            super::PendingAction::SettingsCommand {
                label,
                args,
                redact_output,
            } => {
                assert_eq!(label, "Set proxy ports");
                assert_eq!(
                    args,
                    &vec![
                        "config".to_string(),
                        "set-port".to_string(),
                        "7897".to_string(),
                        "--http".to_string(),
                        "7898".to_string(),
                        "--socks".to_string(),
                        "7899".to_string()
                    ]
                );
                assert!(*redact_output);
            }
            other => panic!("unexpected pending action: {other:?}"),
        }
    }

    #[test]
    fn settings_prompt_structured_fields_build_cli_args() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);
        app.tab = Tab::Settings;

        app.begin_settings_prompt(SettingsPromptKind::ConfigSetPorts);
        for c in "7897".chars() {
            app.push_settings_prompt_char(c);
        }
        app.next_settings_prompt_field();
        for c in "7898".chars() {
            app.push_settings_prompt_char(c);
        }
        app.next_settings_prompt_field();
        for c in "7899".chars() {
            app.push_settings_prompt_char(c);
        }
        app.submit_settings_prompt();

        let pending = app.pending_confirmation.as_ref().unwrap();
        match &pending.action {
            super::PendingAction::SettingsCommand { args, .. } => {
                assert_eq!(
                    args,
                    &vec![
                        "config".to_string(),
                        "set-port".to_string(),
                        "7897".to_string(),
                        "--http".to_string(),
                        "7898".to_string(),
                        "--socks".to_string(),
                        "7899".to_string(),
                    ]
                );
            }
            other => panic!("unexpected pending action: {other:?}"),
        }
    }

    #[test]
    fn mouse_selects_settings_prompt_field() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);
        app.tab = Tab::Settings;
        app.begin_settings_prompt(SettingsPromptKind::ConfigSetApi);
        app.hitboxes.register(
            Rect::new(0, 1, 20, 1),
            HitboxAction::SelectSettingsPromptField(1),
        );

        app.handle_mouse_event(MouseEventKind::Down(MouseButton::Left), 1, 1);

        assert_eq!(app.settings_prompt.as_ref().unwrap().active, 1);
        assert_eq!(app.status_msg.as_deref(), Some("editing Set API · Secret"));
    }

    #[test]
    fn config_set_prompt_validation_keeps_prompt_open() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);
        app.tab = Tab::Settings;

        app.begin_settings_prompt(SettingsPromptKind::ConfigSetDnsMode);
        for c in "invalid".chars() {
            app.push_settings_prompt_char(c);
        }
        app.submit_settings_prompt();

        assert!(app.pending_confirmation.is_none());
        assert!(app.settings_prompt.is_some());
        assert!(app
            .error_msg
            .as_deref()
            .unwrap_or("")
            .contains("DNS mode must be"));
    }

    #[test]
    fn settings_mouse_config_form_actions_open_prompts() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);
        app.tab = Tab::Settings;
        app.hitboxes
            .register(Rect::new(0, 1, 10, 1), HitboxAction::BeginConfigSetLan);

        app.handle_mouse_event(MouseEventKind::Down(MouseButton::Left), 1, 1);

        assert_eq!(
            app.settings_prompt.as_ref().map(|prompt| prompt.kind),
            Some(SettingsPromptKind::ConfigSetLan)
        );
    }

    #[test]
    fn mouse_click_focuses_settings_prompt_value() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);
        app.tab = Tab::Settings;
        app.begin_settings_prompt(SettingsPromptKind::GeodataUpdateVersion);
        app.error_msg = Some("stale error".into());
        app.hitboxes.register(
            Rect::new(0, 1, 20, 1),
            HitboxAction::FocusSettingsPromptValue,
        );
        app.hitboxes.register(
            Rect::new(0, 2, 20, 1),
            HitboxAction::RunSettings(SettingsAction::Doctor),
        );

        app.handle_mouse_event(MouseEventKind::Down(MouseButton::Left), 1, 1);

        assert!(app.settings_prompt.is_some());
        assert!(app.error_msg.is_none());
        assert_eq!(
            app.status_msg.as_deref(),
            Some("editing Update geodata version · Version")
        );

        app.handle_mouse_event(MouseEventKind::Down(MouseButton::Left), 1, 2);
        assert!(app.pending_confirmation.is_none());
        assert_eq!(
            app.settings_prompt.as_ref().map(|prompt| prompt.kind),
            Some(SettingsPromptKind::GeodataUpdateVersion)
        );
    }

    #[test]
    fn dangerous_traffic_action_opens_confirmation() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);
        app.tab = Tab::Traffic;

        app.run_traffic_action(TrafficAction::Reset);

        let pending = app.pending_confirmation.as_ref().unwrap();
        assert!(pending.title.contains("traffic reset"));
        assert!(pending.message.contains("clashctl traffic reset --yes"));
    }

    #[test]
    fn profile_interval_label_prefers_new_metadata_and_disables_file_profiles() {
        let mut profile = ProfileEntry {
            id: 1,
            path: String::new(),
            url: "https://example.test/sub".into(),
            name: String::new(),
            updated: String::new(),
            interval: "12h".into(),
            update_enabled: None,
            update_interval: "6h".into(),
            update_proxy: String::new(),
            user_agent: String::new(),
            convert_mode: String::new(),
            tags: Vec::new(),
            last_error: String::new(),
            last_updated: String::new(),
            next_update: String::new(),
        };
        assert_eq!(profile_interval_label(&profile), "6h");

        profile.update_interval.clear();
        assert_eq!(profile_interval_label(&profile), "12h");

        profile.update_enabled = Some(false);
        assert_eq!(profile_interval_label(&profile), "off");

        profile.update_enabled = None;
        profile.interval.clear();
        profile.url = "file:///tmp/local.yaml".into();
        assert_eq!(profile_interval_label(&profile), "off");
    }

    #[test]
    fn subscription_edit_field_builds_cli_args() {
        assert_eq!(
            SubscriptionEditField::ImportDirectory.args(0, "/tmp/subs".into()),
            vec![
                "sub".to_string(),
                "import".to_string(),
                "/tmp/subs".to_string()
            ]
        );
        assert_eq!(
            SubscriptionEditField::Interval.args(7, "24h".into()),
            vec![
                "sub".to_string(),
                "set-interval".to_string(),
                "7".to_string(),
                "24h".to_string()
            ]
        );
        assert_eq!(
            SubscriptionEditField::AddTag.args(7, "work".into()),
            vec![
                "sub".to_string(),
                "tag".to_string(),
                "add".to_string(),
                "7".to_string(),
                "work".to_string()
            ]
        );
    }

    #[test]
    fn subscription_add_form_builds_parameterized_cli_args() {
        let mut form = SubscriptionAddForm::new();
        form.source = "https://example.test/sub".into();
        form.name = "Work".into();
        form.interval = "6h".into();
        form.update_proxy = "Core".into();
        form.user_agent = "clashctl-test".into();
        form.convert_mode = "Force".into();
        form.tags = "daily, work, daily".into();

        assert_eq!(
            form.args().unwrap(),
            vec![
                "sub".to_string(),
                "add".to_string(),
                "https://example.test/sub".to_string(),
                "--name".to_string(),
                "Work".to_string(),
                "--interval".to_string(),
                "6h".to_string(),
                "--update-proxy".to_string(),
                "core".to_string(),
                "--user-agent".to_string(),
                "clashctl-test".to_string(),
                "--convert".to_string(),
                "force".to_string(),
                "--tag".to_string(),
                "daily".to_string(),
                "--tag".to_string(),
                "work".to_string(),
            ]
        );
    }

    #[test]
    fn subscription_add_form_validates_required_and_enum_fields() {
        let mut form = SubscriptionAddForm::new();
        assert!(form.args().unwrap_err().contains("source"));

        form.source = "https://example.test/sub".into();
        form.update_proxy = "vpn".into();
        assert!(form.args().unwrap_err().contains("update proxy"));

        form.update_proxy = "auto".into();
        form.convert_mode = "maybe".into();
        assert!(form.args().unwrap_err().contains("convert mode"));
    }

    #[test]
    fn subscription_edit_form_builds_changed_cli_commands() {
        let profile = ProfileEntry {
            id: 7,
            path: String::new(),
            url: "https://example.test/old".into(),
            name: "Old".into(),
            updated: String::new(),
            interval: "12h".into(),
            update_enabled: Some(true),
            update_interval: "12h".into(),
            update_proxy: "auto".into(),
            user_agent: "old-ua".into(),
            convert_mode: "auto".into(),
            tags: vec!["daily".into(), "home".into()],
            last_error: String::new(),
            last_updated: String::new(),
            next_update: String::new(),
        };
        let mut form = SubscriptionEditForm::from_profile(&profile);
        form.name = "New".into();
        form.url = "https://example.test/new".into();
        form.interval = "6h".into();
        form.update_proxy = "Core".into();
        form.user_agent = "new-ua".into();
        form.convert_mode = "Force".into();
        form.tags = "daily, work".into();

        assert_eq!(
            form.commands().unwrap(),
            vec![
                vec![
                    "sub".to_string(),
                    "rename".to_string(),
                    "7".to_string(),
                    "New".to_string()
                ],
                vec![
                    "sub".to_string(),
                    "set-url".to_string(),
                    "7".to_string(),
                    "https://example.test/new".to_string()
                ],
                vec![
                    "sub".to_string(),
                    "set-interval".to_string(),
                    "7".to_string(),
                    "6h".to_string()
                ],
                vec![
                    "sub".to_string(),
                    "set-update-proxy".to_string(),
                    "7".to_string(),
                    "core".to_string()
                ],
                vec![
                    "sub".to_string(),
                    "set-user-agent".to_string(),
                    "7".to_string(),
                    "new-ua".to_string()
                ],
                vec![
                    "sub".to_string(),
                    "set-convert".to_string(),
                    "7".to_string(),
                    "force".to_string()
                ],
                vec![
                    "sub".to_string(),
                    "tag".to_string(),
                    "remove".to_string(),
                    "7".to_string(),
                    "home".to_string()
                ],
                vec![
                    "sub".to_string(),
                    "tag".to_string(),
                    "add".to_string(),
                    "7".to_string(),
                    "work".to_string()
                ],
            ]
        );
    }

    #[test]
    fn subscription_edit_form_validates_required_fields_and_noop() {
        let profile = ProfileEntry {
            id: 8,
            path: String::new(),
            url: "https://example.test/sub".into(),
            name: String::new(),
            updated: String::new(),
            interval: String::new(),
            update_enabled: None,
            update_interval: String::new(),
            update_proxy: String::new(),
            user_agent: String::new(),
            convert_mode: String::new(),
            tags: Vec::new(),
            last_error: String::new(),
            last_updated: String::new(),
            next_update: String::new(),
        };
        let mut form = SubscriptionEditForm::from_profile(&profile);
        assert!(form.commands().unwrap().is_empty());

        form.url.clear();
        assert!(form.commands().unwrap_err().contains("source"));

        form.url = profile.url.clone();
        form.update_proxy = "vpn".into();
        assert!(form.commands().unwrap_err().contains("update proxy"));
    }

    #[test]
    fn subscription_edit_prompt_prefills_profile_metadata() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);
        app.tab = Tab::Subscriptions;
        app.profiles = vec![ProfileEntry {
            id: 9,
            path: String::new(),
            url: "https://example.test/sub.yaml".into(),
            name: "Example".into(),
            updated: String::new(),
            interval: "12h".into(),
            update_enabled: Some(true),
            update_interval: "6h".into(),
            update_proxy: "core".into(),
            user_agent: "clashctl-test".into(),
            convert_mode: "force".into(),
            tags: vec!["daily".into()],
            last_error: String::new(),
            last_updated: String::new(),
            next_update: String::new(),
        }];

        app.begin_subscription_edit(SubscriptionEditField::Interval);
        let prompt = app.subscription_prompt.as_ref().unwrap();
        assert_eq!(prompt.profile_id, 9);
        assert_eq!(prompt.value, "6h");

        app.begin_subscription_edit(SubscriptionEditField::UpdateProxy);
        assert_eq!(
            app.subscription_prompt.as_ref().unwrap().value,
            "core".to_string()
        );
    }

    #[test]
    fn subscription_profile_edit_form_prefills_profile_metadata() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);
        app.tab = Tab::Subscriptions;
        app.profiles = vec![ProfileEntry {
            id: 10,
            path: String::new(),
            url: "https://example.test/sub.yaml".into(),
            name: "Example".into(),
            updated: String::new(),
            interval: "12h".into(),
            update_enabled: Some(true),
            update_interval: "6h".into(),
            update_proxy: "core".into(),
            user_agent: "clashctl-test".into(),
            convert_mode: "force".into(),
            tags: vec!["daily".into(), "work".into()],
            last_error: String::new(),
            last_updated: String::new(),
            next_update: String::new(),
        }];

        app.begin_subscription_profile_edit();

        let form = app.subscription_edit_form.as_ref().unwrap();
        assert_eq!(form.profile_id, 10);
        assert_eq!(form.name, "Example");
        assert_eq!(form.url, "https://example.test/sub.yaml");
        assert_eq!(form.interval, "6h");
        assert_eq!(form.update_proxy, "core");
        assert_eq!(form.user_agent, "clashctl-test");
        assert_eq!(form.convert_mode, "force");
        assert_eq!(form.tags, "daily, work");
    }

    #[test]
    fn subscription_add_and_import_prompts_do_not_require_existing_profile() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);
        app.tab = Tab::Subscriptions;

        app.begin_subscription_add();
        let form = app.subscription_add_form.as_ref().unwrap();
        assert!(form.source.is_empty());
        assert_eq!(form.update_proxy, "auto");
        assert_eq!(form.convert_mode, "auto");
        assert!(app.subscription_prompt.is_none());

        app.begin_subscription_import();
        let prompt = app.subscription_prompt.as_ref().unwrap();
        assert_eq!(prompt.field, SubscriptionEditField::ImportDirectory);
        assert_eq!(prompt.profile_id, 0);
        assert!(prompt.value.is_empty());
        assert!(app.subscription_add_form.is_none());
    }

    #[test]
    fn subscription_add_form_keyboard_navigation_targets_active_field() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);
        app.tab = Tab::Subscriptions;

        app.begin_subscription_add();
        app.push_subscription_prompt_char('h');
        app.next_subscription_form_field();
        app.push_subscription_prompt_char('W');
        app.prev_subscription_form_field();
        app.push_subscription_prompt_char('t');

        let form = app.subscription_add_form.as_ref().unwrap();
        assert_eq!(form.source, "ht");
        assert_eq!(form.name, "W");
    }

    #[test]
    fn subscription_add_form_empty_source_keeps_form_open() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);
        app.tab = Tab::Subscriptions;

        app.begin_subscription_add();
        app.submit_subscription_prompt();

        assert!(app.subscription_add_form.is_some());
        assert!(app.error_msg.as_deref().unwrap_or("").contains("source"));
    }

    #[test]
    fn remove_subscription_opens_confirmation() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);
        app.tab = Tab::Subscriptions;
        app.profiles = vec![ProfileEntry {
            id: 5,
            path: String::new(),
            url: "https://example.test/sub".into(),
            name: String::new(),
            updated: String::new(),
            interval: String::new(),
            update_enabled: None,
            update_interval: String::new(),
            update_proxy: String::new(),
            user_agent: String::new(),
            convert_mode: String::new(),
            tags: Vec::new(),
            last_error: String::new(),
            last_updated: String::new(),
            next_update: String::new(),
        }];

        app.remove_selected_subscription();

        let pending = app.pending_confirmation.as_ref().unwrap();
        assert!(pending.title.contains("remove subscription 5"));
        assert!(pending.message.contains("clashctl sub remove 5"));
    }

    #[test]
    fn mouse_click_switches_tab_through_hitbox_registry() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);
        app.hitboxes.register(
            Rect::new(1, 1, 10, 1),
            HitboxAction::SwitchTab(Tab::Connections),
        );

        app.handle_mouse_event(MouseEventKind::Down(MouseButton::Left), 2, 1);

        assert_eq!(app.tab, Tab::Connections);
    }

    #[test]
    fn mouse_click_selects_subscription_row() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);
        app.profiles = vec![
            ProfileEntry {
                id: 1,
                path: String::new(),
                url: "https://one.example/sub".into(),
                name: String::new(),
                updated: String::new(),
                interval: String::new(),
                update_enabled: None,
                update_interval: String::new(),
                update_proxy: String::new(),
                user_agent: String::new(),
                convert_mode: String::new(),
                tags: Vec::new(),
                last_error: String::new(),
                last_updated: String::new(),
                next_update: String::new(),
            },
            ProfileEntry {
                id: 2,
                path: String::new(),
                url: "https://two.example/sub".into(),
                name: String::new(),
                updated: String::new(),
                interval: String::new(),
                update_enabled: None,
                update_interval: String::new(),
                update_proxy: String::new(),
                user_agent: String::new(),
                convert_mode: String::new(),
                tags: Vec::new(),
                last_error: String::new(),
                last_updated: String::new(),
                next_update: String::new(),
            },
        ];
        app.hitboxes
            .register(Rect::new(0, 5, 40, 1), HitboxAction::SelectSubscription(1));

        app.handle_mouse_event(MouseEventKind::Down(MouseButton::Left), 5, 5);

        assert_eq!(app.selected_sub_idx, 1);
    }

    #[test]
    fn mouse_click_subscription_edit_action_opens_prompt() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);
        app.tab = Tab::Subscriptions;
        app.profiles = vec![ProfileEntry {
            id: 7,
            path: String::new(),
            url: "https://example.test/sub".into(),
            name: "Example".into(),
            updated: String::new(),
            interval: "12h".into(),
            update_enabled: None,
            update_interval: "6h".into(),
            update_proxy: "auto".into(),
            user_agent: String::new(),
            convert_mode: String::new(),
            tags: Vec::new(),
            last_error: String::new(),
            last_updated: String::new(),
            next_update: String::new(),
        }];
        app.hitboxes.register(
            Rect::new(0, 5, 20, 1),
            HitboxAction::EditSubscriptionInterval,
        );

        app.handle_mouse_event(MouseEventKind::Down(MouseButton::Left), 1, 5);

        let prompt = app.subscription_prompt.as_ref().unwrap();
        assert_eq!(prompt.profile_id, 7);
        assert_eq!(prompt.field, SubscriptionEditField::Interval);
        assert_eq!(prompt.value, "6h");
    }

    #[test]
    fn pending_confirmation_blocks_underlying_mouse_actions() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);
        app.profiles = vec![
            ProfileEntry {
                id: 1,
                path: String::new(),
                url: "https://one.example/sub".into(),
                name: String::new(),
                updated: String::new(),
                interval: String::new(),
                update_enabled: None,
                update_interval: String::new(),
                update_proxy: String::new(),
                user_agent: String::new(),
                convert_mode: String::new(),
                tags: Vec::new(),
                last_error: String::new(),
                last_updated: String::new(),
                next_update: String::new(),
            },
            ProfileEntry {
                id: 2,
                path: String::new(),
                url: "https://two.example/sub".into(),
                name: String::new(),
                updated: String::new(),
                interval: String::new(),
                update_enabled: None,
                update_interval: String::new(),
                update_proxy: String::new(),
                user_agent: String::new(),
                convert_mode: String::new(),
                tags: Vec::new(),
                last_error: String::new(),
                last_updated: String::new(),
                next_update: String::new(),
            },
        ];
        app.pending_confirmation = Some(super::PendingConfirmation {
            title: "Confirm stop".into(),
            message: "Run `clashctl stop`?".into(),
            action: super::PendingAction::Network(NetworkAction::Stop),
        });
        app.hitboxes
            .register(Rect::new(0, 5, 40, 1), HitboxAction::SelectSubscription(1));

        app.handle_mouse_event(MouseEventKind::Down(MouseButton::Left), 5, 5);

        assert_eq!(app.selected_sub_idx, 0);
        assert!(app.pending_confirmation.is_some());
    }

    #[test]
    fn mouse_prompt_submit_and_cancel_actions_are_dispatched() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);
        app.subscription_prompt = Some(SubscriptionPrompt {
            field: SubscriptionEditField::Url,
            profile_id: 1,
            value: String::new(),
        });
        app.hitboxes.register(
            Rect::new(1, 1, 10, 1),
            HitboxAction::SubmitSubscriptionPrompt,
        );

        app.handle_mouse_event(MouseEventKind::Down(MouseButton::Left), 2, 1);
        assert!(app
            .error_msg
            .as_deref()
            .unwrap_or("")
            .contains("cannot be empty"));
        assert!(app.subscription_prompt.is_some());

        app.hitboxes.clear();
        app.hitboxes.register(
            Rect::new(1, 2, 10, 1),
            HitboxAction::CancelSubscriptionPrompt,
        );
        app.handle_mouse_event(MouseEventKind::Down(MouseButton::Left), 2, 2);
        assert!(app.subscription_prompt.is_none());
    }

    #[test]
    fn mouse_selects_subscription_add_form_field() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);
        app.tab = Tab::Subscriptions;
        app.begin_subscription_add();
        app.hitboxes.register(
            Rect::new(1, 1, 20, 1),
            HitboxAction::SelectSubscriptionAddField(3),
        );

        app.handle_mouse_event(MouseEventKind::Down(MouseButton::Left), 2, 1);

        assert_eq!(app.subscription_add_form.as_ref().unwrap().active, 3);
    }

    #[test]
    fn mouse_selects_subscription_edit_form_field() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);
        app.tab = Tab::Subscriptions;
        app.profiles = vec![ProfileEntry {
            id: 12,
            path: String::new(),
            url: "https://example.test/sub".into(),
            name: "Example".into(),
            updated: String::new(),
            interval: String::new(),
            update_enabled: None,
            update_interval: String::new(),
            update_proxy: String::new(),
            user_agent: String::new(),
            convert_mode: String::new(),
            tags: Vec::new(),
            last_error: String::new(),
            last_updated: String::new(),
            next_update: String::new(),
        }];
        app.begin_subscription_profile_edit();
        app.hitboxes.register(
            Rect::new(1, 1, 20, 1),
            HitboxAction::SelectSubscriptionAddField(5),
        );

        app.handle_mouse_event(MouseEventKind::Down(MouseButton::Left), 2, 1);

        assert_eq!(app.subscription_edit_form.as_ref().unwrap().active, 5);
    }

    #[test]
    fn subscription_action_hitboxes_follow_translated_buttons() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);
        app.ui_settings.language = LanguageSetting::ZhCn;
        let area = Rect::new(10, 20, 180, 4);
        let y = area.y + area.height.saturating_sub(1);
        let mut x = area.x + 2;

        let buttons = subscription_action_buttons(&app);
        register_subscription_action_hitboxes(&mut app, area);

        for (label, action) in buttons {
            assert_eq!(app.hitboxes.action_at(x, y), Some(action));
            x = x.saturating_add(action_button_width(&label) + 1);
        }
    }

    #[test]
    fn subscription_form_buttons_follow_translated_widths() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);
        app.ui_settings.language = LanguageSetting::ZhCn;
        let inner = Rect::new(4, 8, 80, 16);
        let field_y = inner.y.saturating_add(2);
        let button_y = field_y
            .saturating_add(super::SubscriptionAddField::all().len() as u16)
            .saturating_add(1);
        let save_width = action_button_width(app.t(crate::i18n::Msg::CommonSave));

        register_subscription_form_hitboxes(&mut app, inner);

        assert_eq!(
            app.hitboxes.action_at(inner.x, button_y),
            Some(HitboxAction::SubmitSubscriptionPrompt)
        );
        assert_eq!(
            app.hitboxes.action_at(
                inner.x.saturating_add(save_width).saturating_add(3),
                button_y
            ),
            Some(HitboxAction::CancelSubscriptionPrompt)
        );
    }

    #[test]
    fn mouse_click_selects_proxy_node_when_picker_is_open() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);
        app.tab = Tab::Proxies;
        app.proxies
            .insert("Proxy".into(), selector_proxy("B", vec!["A", "B", "C"]));
        app.proxy_groups = vec![("Proxy".into(), "B".into())];
        app.open_node_picker();
        app.hitboxes
            .register(Rect::new(1, 1, 20, 1), HitboxAction::SelectProxyNode(2));

        app.handle_mouse_event(MouseEventKind::Down(MouseButton::Left), 2, 1);

        assert_eq!(app.selected_node_idx, 2);
    }

    #[test]
    fn node_picker_blocks_underlying_mouse_actions() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);
        app.tab = Tab::Proxies;
        app.proxy_groups = vec![("One".into(), "A".into()), ("Two".into(), "B".into())];
        app.proxies
            .insert("One".into(), selector_proxy("A", vec!["A"]));
        app.proxies
            .insert("Two".into(), selector_proxy("B", vec!["B"]));
        app.open_node_picker();
        app.hitboxes
            .register(Rect::new(1, 1, 20, 1), HitboxAction::SelectProxy(1));

        app.handle_mouse_event(MouseEventKind::Down(MouseButton::Left), 2, 1);

        assert_eq!(app.selected_proxy_idx, 0);
        assert!(app.node_picker_open);
    }

    #[test]
    fn mouse_wheel_scrolls_proxy_node_picker() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);
        app.tab = Tab::Proxies;
        app.proxies.insert(
            "Proxy".into(),
            selector_proxy("A", vec!["A", "B", "C", "D"]),
        );
        app.proxy_groups = vec![("Proxy".into(), "A".into())];
        app.open_node_picker();
        app.hitboxes
            .register(Rect::new(0, 0, 40, 10), HitboxAction::ScrollProxyNodes);

        app.handle_mouse_event(MouseEventKind::ScrollDown, 5, 5);
        assert_eq!(app.selected_node_idx, 3);
        app.handle_mouse_event(MouseEventKind::ScrollUp, 5, 5);
        assert_eq!(app.selected_node_idx, 0);
    }

    #[test]
    fn mouse_wheel_dispatches_to_registered_scroll_area() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);
        app.tab = Tab::Logs;
        app.logs = (0..10).map(|idx| format!("INFO line {idx}")).collect();
        app.hitboxes
            .register(Rect::new(0, 0, 40, 10), HitboxAction::ScrollLogs);

        app.handle_mouse_event(MouseEventKind::ScrollDown, 5, 5);
        assert_eq!(app.log_scroll, 3);
        app.handle_mouse_event(MouseEventKind::ScrollUp, 5, 5);
        assert_eq!(app.log_scroll, 0);
    }

    #[test]
    fn mode_display_normalizes_api_values() {
        assert_eq!(mode_display("rule"), "Rule");
        assert_eq!(mode_display("Global"), "Global");
        assert_eq!(mode_display("DIRECT"), "Direct");
        assert_eq!(mode_display("unexpected"), "Rule");
    }

    #[test]
    fn next_mode_cycles_rule_global_direct() {
        assert_eq!(next_mode("Rule"), "Global");
        assert_eq!(next_mode("Global"), "Direct");
        assert_eq!(next_mode("Direct"), "Rule");
    }

    #[test]
    fn log_level_filter_is_minimum_severity() {
        assert_eq!(log_line_rank("DEBUG noisy"), 0);
        assert_eq!(log_line_rank("INFO ready"), 1);
        assert_eq!(log_line_rank("WARNING slow"), 2);
        assert_eq!(log_line_rank("ERROR failed"), 3);

        assert!(LogLevelFilter::Info.matches_line("INFO ready"));
        assert!(!LogLevelFilter::Info.matches_line("DEBUG noisy"));
        assert!(LogLevelFilter::Warning.matches_line("ERROR failed"));
        assert!(!LogLevelFilter::Warning.matches_line("INFO ready"));
        assert!(LogLevelFilter::Error.matches_line("ERROR failed"));
        assert!(!LogLevelFilter::Error.matches_line("WARNING slow"));
    }

    #[test]
    fn log_level_cycle_covers_all_filters() {
        assert_eq!(LogLevelFilter::Debug.next(), LogLevelFilter::Info);
        assert_eq!(LogLevelFilter::Info.next(), LogLevelFilter::Warning);
        assert_eq!(LogLevelFilter::Warning.next(), LogLevelFilter::Error);
        assert_eq!(LogLevelFilter::Error.next(), LogLevelFilter::Debug);
    }

    #[test]
    fn node_picker_opens_on_current_node_and_wraps_selection() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);
        app.tab = Tab::Proxies;
        app.proxy_groups = vec![("Auto".into(), "B".into())];
        app.proxies
            .insert("Auto".into(), selector_proxy("B", vec!["A", "B", "C"]));

        app.open_node_picker();
        assert!(app.node_picker_open);
        assert_eq!(app.selected_node_idx, 1);

        app.select_node_down();
        assert_eq!(app.selected_node_idx, 2);
        app.select_node_down();
        assert_eq!(app.selected_node_idx, 0);
        app.select_node_up();
        assert_eq!(app.selected_node_idx, 2);
    }

    #[test]
    fn node_picker_stays_closed_when_selected_group_has_no_nodes() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = test_app(&rt);
        app.tab = Tab::Proxies;
        app.proxy_groups = vec![("Direct".into(), String::new())];
        app.proxies
            .insert("Direct".into(), selector_proxy("", Vec::new()));

        app.open_node_picker();
        assert!(!app.node_picker_open);
        assert_eq!(app.selected_node_idx, 0);
        assert_eq!(
            app.status_msg.as_deref(),
            Some("selected group has no nodes")
        );
    }
}
