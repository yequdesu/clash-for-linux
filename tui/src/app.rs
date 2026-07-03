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
use crate::window::WindowState;

pub(crate) use crate::ui::model::*;
pub(crate) use crate::ui::utils::*;

pub struct App {
    pub should_quit: bool,
    pub force_terminal_clear: bool,
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
    pub proxy_table_state: TableState,
    pub delays: HashMap<String, u64>,

    pub connections: Vec<Connection>,
    pub connections_active: usize,
    pub connections_total: usize,
    pub connections_table_state: TableState,

    pub logs: Vec<String>,

    pub profiles: Vec<ProfileEntry>,
    pub active_profile_id: i32,
    pub network_output: Vec<String>,
    pub tun_enabled: bool,
    pub traffic_points: Vec<TrafficPoint>,
    pub traffic_top: Vec<TrafficTopRow>,
    pub traffic_status: TrafficStatus,
    pub ui_state: crate::ui::model::UiState,

    pub proxy_mode_str: String,
    pub window: WindowState,
    pub background: Box<dyn BackgroundEffect>,
    pub ui_settings: UiSettings,

    pub api: ApiClient,
    pub config: Config,
    pub rt: tokio::runtime::Handle,
    pub data_tx: mpsc::Sender<DataEvent>,
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
            force_terminal_clear: true,
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
            proxy_table_state: TableState::default(),
            delays: HashMap::new(),
            connections: Vec::new(),
            connections_active: 0,
            connections_total: 0,
            connections_table_state: TableState::default(),
            logs: Vec::new(),
            profiles: profiles_meta.profiles,
            active_profile_id: profiles_meta.active_id,
            network_output: Vec::new(),
            tun_enabled: crate::api::read_tun_status(),
            traffic_points: Vec::new(),
            traffic_top: Vec::new(),
            traffic_status: TrafficStatus::default(),
            ui_state: crate::ui::model::UiState::new(
                initial_tab,
                traffic_range,
                traffic_chart,
                traffic_dimension,
            ),
            proxy_mode_str: "Rule".into(),
            window: WindowState::load(),
            background: Box::new(NoopBackground),
            ui_settings,
            api,
            config,
            rt,
            data_tx,
        }
    }

    pub(crate) fn request_terminal_clear(&mut self) {
        self.force_terminal_clear = true;
    }

    pub(crate) fn reveal_command_output(&mut self) {
        self.ui_state.command_output.scroll = 0;
        self.ui_state.command_output.hidden = false;
    }
}

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;
