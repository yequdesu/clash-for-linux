use ratatui::widgets::TableState;
use std::collections::HashMap;
use std::sync::mpsc;

use crate::api::{
    ApiClient, Connection, ProfileEntry, ProxyInfo, TrafficPoint, TrafficStatus, TrafficTopRow,
};
use crate::background::{BackgroundEffect, NoopBackground};
use crate::config::Config;
use crate::event::DataEvent;
use crate::settings::UiSettings;
use crate::theme::{apply_theme_key, normalize_theme_key};
use crate::widgets::sparkline::TrafficHistory;
use crate::window::WindowState;

pub(crate) use crate::ui::model::*;
pub(crate) use crate::ui::utils::*;

pub struct App {
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
    pub help_scroll: usize,
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
    pub network_output_scroll: usize,
    pub settings_output: Vec<String>,
    pub settings_prompt: Option<SettingsPrompt>,
    pub settings_section: SettingsSection,
    pub sudo_prompt: Option<SudoPrompt>,
    pub(crate) sudo_candidate: Option<SudoPrompt>,
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
    pub ui_state: crate::ui::model::UiState,

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
            help_scroll: 0,
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
            network_output_scroll: 0,
            settings_output: Vec::new(),
            settings_prompt: None,
            settings_section: SettingsSection::General,
            sudo_prompt: None,
            sudo_candidate: None,
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
            ui_state: crate::ui::model::UiState::new(initial_tab),
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
}

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;
