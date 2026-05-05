use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Row, Table, TableState};
use ratatui::Frame;
use std::collections::HashMap;
use std::sync::mpsc;

use crate::api::{ApiClient, Connection, ProfileEntry, ProxyInfo};
use crate::background::{BackgroundEffect, NoopBackground};
use crate::config::Config;
use crate::event::DataEvent;
use crate::theme::CLASH_THEME;
use crate::widgets::sparkline::TrafficHistory;
use crate::window::WindowState;
use crate::widgets::tab_bar::Tab;

pub struct App {
    pub tab: Tab,
    pub should_quit: bool,
    pub show_help: bool,
    pub error_msg: Option<String>,
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

    pub profiles: Vec<ProfileEntry>,
    pub active_profile_id: i32,
    pub selected_sub_idx: usize,
    pub tun_enabled: bool,

    pub search_active: bool,
    pub search_query: String,
    pub sort_mode: bool, // false=by name, true=by delay
    pub proxy_mode_str: String, // Rule, Global, Direct
    pub window: WindowState,
    pub background: Box<dyn BackgroundEffect>,

    pub api: ApiClient,
    #[allow(dead_code)]
    pub config: Config,
    pub rt: tokio::runtime::Handle,
    pub data_tx: mpsc::Sender<DataEvent>,

    pub traffic_history: TrafficHistory,
}

impl App {
    pub fn new(config: Config, rt: tokio::runtime::Handle, data_tx: mpsc::Sender<DataEvent>) -> Self {
        let api = ApiClient::new(config.api_url.clone(), config.api_key.clone());
        let profiles_meta = crate::api::read_profiles();
        Self {
            tab: Tab::Overview,
            should_quit: false,
            show_help: false,
            error_msg: None,
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
            profiles: profiles_meta.profiles,
            active_profile_id: profiles_meta.active_id,
            selected_sub_idx: 0,
            tun_enabled: crate::api::read_tun_status(),
            search_active: false,
            search_query: String::new(),
            sort_mode: false,
            proxy_mode_str: "Rule".into(),
            window: WindowState::load(),
            background: Box::new(NoopBackground),
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
        if now - self.last_refresh >= 2 {
            self.last_refresh = now;
            self.refresh_data();
        }
    }

    pub fn refresh_data(&mut self) {
        self.fetch_proxies();
        self.fetch_connections();
        self.fetch_version();
        if self.tab == Tab::Logs && !self.log_paused {
            self.fetch_logs();
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
        self.rt.spawn(async move {
            let result = api.get_logs().await;
            let _ = tx.send(DataEvent::Logs(result));
        });
    }

    pub fn apply_data_event(&mut self, event: DataEvent) {
        match event {
            DataEvent::Proxies(Ok(resp)) => {
                self.proxies = resp.proxies;
                self.proxy_groups.clear();
                for (name, info) in &self.proxies {
                    if info.proxy_type == "Selector" || info.proxy_type == "Fallback" || info.proxy_type == "URLTest" {
                        let current = info.now.clone().unwrap_or_default();
                        self.proxy_groups.push((name.clone(), current));
                    }
                }
            }
            DataEvent::Proxies(Err(e)) => {
                if self.error_msg.is_none() { self.error_msg = Some(e); }
            }
            DataEvent::Connections(Ok(resp)) => {
                self.prev_upload = self.upload_total;
                self.prev_download = self.download_total;
                self.upload_total = resp.upload_total;
                self.download_total = resp.download_total;
                self.upload_rate = (self.upload_total.saturating_sub(self.prev_upload)) as f64;
                self.download_rate = (self.download_total.saturating_sub(self.prev_download)) as f64;
                let conn_len = resp.connections.len();
                self.connections = resp.connections;
                self.connections_active = self.connections.len();
                self.connections_total = conn_len;
                self.traffic_history.push(
                    resp.upload_total as f64,
                    resp.download_total as f64,
                );
            }
            DataEvent::Connections(Err(_)) => {}
            DataEvent::Version(Ok(info)) => {
                self.version = info.version.unwrap_or_default();
                self.mode = info.mode;
            }
            DataEvent::Version(Err(_)) => {}
            DataEvent::Logs(Ok(entries)) => {
                for e in entries {
                    let line = format!("{} {}", e.level.to_uppercase(), &e.payload[..e.payload.len().min(120)]);
                    self.logs.push(line);
                }
                if self.logs.len() > 500 {
                    self.logs.drain(0..self.logs.len() - 500);
                }
                self.log_scroll = self.logs.len().saturating_sub(1);
            }
            DataEvent::Logs(Err(_)) => {}
            DataEvent::Delay(name, delay) => {
                self.delays.insert(name, delay);
            }
            DataEvent::SwitchResult(Ok(())) => {
                self.error_msg = None;
            }
            DataEvent::SwitchResult(Err(e)) => {
                self.error_msg = Some(format!("Switch failed: {}", e));
            }
        }
    }

    pub fn next_tab(&mut self) { self.tab = self.tab.next(); self.show_help = false; self.reset_selection(); }
    pub fn prev_tab(&mut self) { self.tab = self.tab.prev(); self.show_help = false; self.reset_selection(); }

    fn reset_selection(&mut self) {
        self.selected_proxy_idx = 0;
        self.connections_selected = 0;
    }

    pub fn select_down(&mut self) {
        match self.tab {
            Tab::Proxies | Tab::Overview => {
                if self.proxy_groups.is_empty() { return; }
                self.selected_proxy_idx = (self.selected_proxy_idx + 1) % self.proxy_groups.len();
            }
            Tab::Connections => {
                if self.connections.is_empty() { return; }
                self.connections_selected = (self.connections_selected + 1) % self.connections.len();
            }
            _ => {}
        }
    }

    pub fn select_up(&mut self) {
        match self.tab {
            Tab::Proxies | Tab::Overview => {
                if self.proxy_groups.is_empty() { return; }
                self.selected_proxy_idx = if self.selected_proxy_idx == 0 {
                    self.proxy_groups.len().saturating_sub(1)
                } else {
                    self.selected_proxy_idx - 1
                };
            }
            Tab::Connections => {
                if self.connections.is_empty() { return; }
                self.connections_selected = if self.connections_selected == 0 {
                    self.connections.len().saturating_sub(1)
                } else {
                    self.connections_selected - 1
                };
            }
            _ => {}
        }
    }

    pub fn test_selected_delay(&mut self) {
        let api = self.api.clone();
        let tx = self.data_tx.clone();
        if !self.proxy_groups.is_empty() && (self.tab == Tab::Proxies || self.tab == Tab::Overview) {
            let (ref group, _) = self.proxy_groups[self.selected_proxy_idx % self.proxy_groups.len()];
            let name = group.clone();
            self.rt.spawn(async move {
                match api.test_delay(&name).await {
                    Ok(delay) => { let _ = tx.send(DataEvent::Delay(name, delay)); }
                    Err(_) => {}
                }
            });
        }
    }

    pub fn switch_selected(&mut self) {
        let api = self.api.clone();
        let tx = self.data_tx.clone();
        if !self.proxy_groups.is_empty() && (self.tab == Tab::Proxies || self.tab == Tab::Overview) {
            let idx = self.selected_proxy_idx % self.proxy_groups.len();
            let (ref group, _) = self.proxy_groups[idx];
            let group_name = group.clone();
            if let Some(info) = self.proxies.get(&group_name) {
                if let Some(ref all) = info.all {
                    let all = all.clone();
                    let api2 = api.clone();
                    let tx2 = tx.clone();
                    self.rt.spawn(async move {
                        let current = &all[idx % all.len()];
                        let result = api2.switch_proxy(&group_name, current).await;
                        let _ = tx2.send(DataEvent::SwitchResult(result));
                    });
                }
            }
        }
    }

    pub fn close_selected_connection(&mut self) {
        if self.tab != Tab::Connections || self.connections.is_empty() { return; }
        let api = self.api.clone();
        let conn_id = self.connections[self.connections_selected].id.clone();
        self.rt.spawn(async move {
            let _ = api.close_connection(&conn_id).await;
        });
    }

    pub fn close_all_connections(&mut self) {
        if self.tab != Tab::Connections { return; }
        let api = self.api.clone();
        self.rt.spawn(async move {
            let _ = api.close_all_connections().await;
        });
    }

    pub fn on_shutdown(&mut self) {
        self.window.save();
    }

    pub fn refresh_subscriptions(&mut self) {
        let meta = crate::api::read_profiles();
        self.profiles = meta.profiles;
        self.active_profile_id = meta.active_id;
    }

    pub fn toggle_sort(&mut self) {
        self.sort_mode = !self.sort_mode;
    }

    pub fn cycle_proxy_mode(&mut self) {
        self.proxy_mode_str = match self.proxy_mode_str.as_str() {
            "Rule" => "Global".into(),
            "Global" => "Direct".into(),
            _ => "Rule".into(),
        };
    }

    pub fn test_all_delays(&mut self) {
        let api = self.api.clone();
        let tx = self.data_tx.clone();
        for (group, _) in &self.proxy_groups {
            let name = group.clone();
            let api = api.clone();
            let tx = tx.clone();
            self.rt.spawn(async move {
                match api.test_delay(&name).await {
                    Ok(delay) => { let _ = tx.send(DataEvent::Delay(name, delay)); }
                    Err(_) => {}
                }
            });
        }
    }
}

pub fn render(frame: &mut Frame, app: &mut App) {
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
    let title = format!(" clash-tui · {} ", app.tab.label());
    let title_span = Span::styled(title.clone(), Style::default().fg(CLASH_THEME.primary).bold());
    let decor = "─".repeat(inner.width.saturating_sub(title.len() as u16) as usize);
    let title_line = Line::from(vec![title_span, Span::styled(decor, Style::default().fg(CLASH_THEME.muted))]);
    frame.render_widget(
        Paragraph::new(title_line).style(Style::default().bg(CLASH_THEME.bg)),
        Rect::new(win.x + 1, win.y, win.width.saturating_sub(2), 1),
    );

    let chunks = if inner.height >= 6 {
        Layout::vertical([Constraint::Length(2), Constraint::Min(4), Constraint::Length(1)]).split(inner)
    } else {
        Layout::vertical([Constraint::Length(0), Constraint::Min(2), Constraint::Length(1)]).split(inner)
    };

    fill_area(frame, chunks[0], CLASH_THEME.bg);
    crate::widgets::tab_bar::render_tab_bar(frame, chunks[0], app.tab);

    let content_area = chunks[1];
    fill_area(frame, content_area, CLASH_THEME.bg);

    if app.show_help {
        render_help(frame, content_area);
    } else {
        match app.tab {
            Tab::Overview => render_overview(frame, content_area, app),
            Tab::Proxies => render_proxies(frame, content_area, app),
            Tab::Subscriptions => render_subscriptions(frame, content_area, app),
            Tab::Connections => render_connections(frame, content_area, app),
            Tab::Logs => render_logs(frame, content_area, app),
        }
    }

    render_status_bar(frame, chunks[2], app);
}

fn render_overview(frame: &mut Frame, area: Rect, app: &App) {
    if area.height < 12 { return; }

    let rows = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(6),
        Constraint::Length(3),
        Constraint::Min(3),
    ]).split(area);

    // Row 0: Status bar
    let status_dot = if !app.version.is_empty() { "●" } else { "○" };
    let status_color = if !app.version.is_empty() { CLASH_THEME.accent } else { CLASH_THEME.muted };
    let ver_display = if app.version.is_empty() { "Mihomo —".to_string() } else { format!("Mihomo {}", app.version) };
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
    let status_lines = vec![
        Line::from(vec![
            Span::styled(format!(" {} ", status_dot), Style::default().fg(status_color).bold()),
            Span::styled(format!("{}    {}  ", ver_display, app.mode), CLASH_THEME.text),
            Span::styled("TUN ", CLASH_THEME.muted),
            Span::styled(if app.tun_enabled { "Enabled" } else { "Disabled" },
                if app.tun_enabled { CLASH_THEME.accent } else { CLASH_THEME.muted }),
            Span::styled(format!("    Uptime: {}", uptime), CLASH_THEME.text),
        ]),
    ];
    crate::widgets::card::Card::new("Status").render(frame, rows[0], status_lines);

    // Row 1: Traffic + Current Proxy cards
    let mid_top = Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(rows[1]);

    let traffic_lines = vec![
        Line::from(vec![
            Span::styled(format!("  ↑ {}/s  ↓ {}/s",
                format_bytes(app.upload_rate as u64),
                format_bytes(app.download_rate as u64)), CLASH_THEME.text),
        ]),
        Line::from(vec![
            Span::styled(format!("  Total  ↑ {}  ↓ {}",
                format_bytes(app.upload_total),
                format_bytes(app.download_total)), CLASH_THEME.muted),
        ]),
    ];
    crate::widgets::card::Card::new("Traffic").render(frame, mid_top[0], traffic_lines);

    let mut proxy_lines = vec![Line::from(Span::styled("  No proxy selected", CLASH_THEME.muted))];
    if let Some(first) = app.proxy_groups.first() {
        let (ref name, ref current) = *first;
        let delay = app.delays.get(name).map(|d| format!("{}ms", d)).unwrap_or_else(|| "—".into());
        proxy_lines = vec![
            Line::from(vec![
                Span::styled(format!("  {} → ", name), CLASH_THEME.muted),
                Span::styled(current.as_str(), CLASH_THEME.accent),
            ]),
            Line::from(vec![
                Span::styled("  Delay: ", CLASH_THEME.muted),
                Span::styled(delay, CLASH_THEME.text),
            ]),
        ];
    }
    crate::widgets::card::Card::new("Current Proxy").render(frame, mid_top[1], proxy_lines);

    // Row 2: Connections + System Info
    let mid_bot = Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(rows[2]);

    let conn_lines = vec![
        Line::from(vec![
            Span::styled(format!("  Active: {}   Total: {}", app.connections_active, app.connections_total), CLASH_THEME.text),
        ]),
    ];
    crate::widgets::card::Card::new("Connections").render(frame, mid_bot[0], conn_lines);

    let sys_lines = vec![
        Line::from(vec![
            Span::styled(format!("  OS: {}  ", app.os_info), CLASH_THEME.text),
            Span::styled(format!("Arch: {}", app.arch_info), CLASH_THEME.text),
        ]),
    ];
    crate::widgets::card::Card::new("System Info").render(frame, mid_bot[1], sys_lines);

    // Row 3: Subscription summary
    let sub_lines = if app.profiles.is_empty() {
        vec![Line::from(Span::styled("  No subscriptions — use 'clashctl sub add'", CLASH_THEME.muted))]
    } else {
        let active = app.profiles.iter().find(|p| p.id == app.active_profile_id);
        let summary = match active {
            Some(p) => {
                let name = if p.name.is_empty() { &p.url } else { &p.name };
                let updated = if p.updated.is_empty() { "—" } else { &p.updated };
                format!("{} | Updated: {} | URL: {}", name, updated, p.url)
            }
            None => format!("{} subscriptions (none active)", app.profiles.len()),
        };
        vec![Line::from(Span::styled(format!("  {}", summary), CLASH_THEME.text))]
    };
    crate::widgets::card::Card::new("Subscription").render(frame, rows[3], sub_lines);
}

fn render_proxies(frame: &mut Frame, area: Rect, app: &App) {
    let mut rows: Vec<Vec<String>> = Vec::new();
    for (name, info) in &app.proxies {
        if info.proxy_type != "Selector" && info.proxy_type != "Fallback" && info.proxy_type != "URLTest" {
            continue;
        }
        let current = info.now.as_deref().unwrap_or("—");
        let delay = app.delays.get(name).map(|d| format!("{}ms", d)).unwrap_or_else(|| "—".into());
        let all_count = info.all.as_ref().map(|a| a.len()).unwrap_or(0);
        rows.push(vec![name.clone(), current.to_string(), delay, format!("{} nodes", all_count)]);
    }

    fill_area(frame, area, CLASH_THEME.surface);

    if rows.is_empty() {
        let msg = Paragraph::new("No proxy groups found — is kernel running?")
            .style(Style::default().fg(CLASH_THEME.muted));
        frame.render_widget(msg, area);
        return;
    }

    let header = Row::new(vec!["Group", "Current", "Delay", "Nodes"])
        .style(Style::default().fg(CLASH_THEME.muted));
    let data_rows: Vec<Row> = rows.iter().enumerate().map(|(i, row)| {
        let style = if i == app.selected_proxy_idx {
            Style::default().fg(CLASH_THEME.text).bg(CLASH_THEME.primary)
        } else {
            Style::default().fg(CLASH_THEME.text).bg(CLASH_THEME.surface)
        };
        Row::new(row.clone()).style(style)
    }).collect();

    let widths = [Constraint::Ratio(1, 4), Constraint::Ratio(1, 4), Constraint::Ratio(1, 4), Constraint::Ratio(1, 4)];
    let table = Table::new(data_rows, widths).header(header)
        .style(Style::default().bg(CLASH_THEME.surface));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLASH_THEME.border))
        .title(Span::styled(" Proxies ", Style::default().fg(CLASH_THEME.primary).bold()))
        .title_bottom(Span::styled(" Enter:test  s:switch  d:test all  /:search ", Style::default().fg(CLASH_THEME.muted)));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_stateful_widget(table, inner, &mut app.proxy_table_state.clone());
}

fn render_subscriptions(frame: &mut Frame, area: Rect, app: &App) {
    fill_area(frame, area, CLASH_THEME.surface);

    if app.profiles.is_empty() {
        let msg = Paragraph::new("No subscriptions found — use 'clashctl sub add' to add one")
            .style(Style::default().fg(CLASH_THEME.muted));
        frame.render_widget(msg, area);
        return;
    }

    let header = Row::new(vec!["ID", "Name", "URL"])
        .style(Style::default().fg(CLASH_THEME.muted));
    let data_rows: Vec<Row> = app.profiles.iter().enumerate().map(|(i, p)| {
        let active_marker = if p.id == app.active_profile_id { "● " } else { "  " };
        let name = if p.name.is_empty() { &p.url } else { &p.name };
        let style = if i == app.selected_sub_idx {
            Style::default().fg(CLASH_THEME.text).bg(CLASH_THEME.primary)
        } else if p.id == app.active_profile_id {
            Style::default().fg(CLASH_THEME.accent).bg(CLASH_THEME.surface)
        } else {
            Style::default().fg(CLASH_THEME.text).bg(CLASH_THEME.surface)
        };
        Row::new(vec![
            format!("{}{}", active_marker, p.id),
            name.chars().take(30).collect(),
            p.url.chars().take(50).collect(),
        ]).style(style)
    }).collect();

    let widths = [Constraint::Length(6), Constraint::Length(32), Constraint::Ratio(1, 1)];
    let table = Table::new(data_rows, widths).header(header)
        .style(Style::default().bg(CLASH_THEME.surface));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLASH_THEME.border))
        .title(Span::styled(" Subscriptions ", Style::default().fg(CLASH_THEME.primary).bold()))
        .title_bottom(Span::styled(" ● active  |  Use 'clashctl sub' for management  ", Style::default().fg(CLASH_THEME.muted)));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(table, inner);
}

fn render_connections(frame: &mut Frame, area: Rect, app: &App) {
    fill_area(frame, area, CLASH_THEME.surface);

    let header_text = format!(" Active: {} | c:close  C:close all ", app.connections.len());

    let mut rows: Vec<Vec<String>> = Vec::new();
    for conn in &app.connections {
        let host = conn.metadata.as_ref().and_then(|m| m.host.as_ref()).cloned().unwrap_or_default();
        let network = conn.metadata.as_ref().and_then(|m| m.network.as_ref()).cloned().unwrap_or_default();
        let chain = conn.chains.first().cloned().unwrap_or_default();
        rows.push(vec![host, network, chain]);
    }

    if rows.is_empty() {
        let msg = Paragraph::new("No active connections")
            .style(Style::default().fg(CLASH_THEME.muted));
        frame.render_widget(msg, area);
        return;
    }

    let header = Row::new(vec!["Host", "Type", "Chain"])
        .style(Style::default().fg(CLASH_THEME.muted));
    let data_rows: Vec<Row> = rows.iter().enumerate().map(|(i, row)| {
        let style = if i == app.connections_selected {
            Style::default().fg(CLASH_THEME.text).bg(CLASH_THEME.primary)
        } else {
            Style::default().fg(CLASH_THEME.text).bg(CLASH_THEME.surface)
        };
        Row::new(row.clone()).style(style)
    }).collect();

    let widths = [Constraint::Ratio(2, 5), Constraint::Length(10), Constraint::Ratio(2, 5)];
    let table = Table::new(data_rows, widths).header(header)
        .style(Style::default().bg(CLASH_THEME.surface));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLASH_THEME.border))
        .title(Span::styled(" Connections ", Style::default().fg(CLASH_THEME.primary).bold()))
        .title_bottom(Span::styled(format!(" {} ", header_text), Style::default().fg(CLASH_THEME.muted)));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_stateful_widget(table, inner, &mut app.connections_table_state.clone());
}

fn render_logs(frame: &mut Frame, area: Rect, app: &App) {
    fill_area(frame, area, CLASH_THEME.surface);

    let max_lines = area.height.saturating_sub(2) as usize;
    let start = if app.logs.len() > max_lines {
        (app.log_scroll.min(app.logs.len().saturating_sub(1))).saturating_sub(max_lines.saturating_sub(1))
    } else {
        0
    };
    let end = (start + max_lines).min(app.logs.len());

    let lines: Vec<Line> = app.logs[start..end].iter().map(|l| {
        let color = if l.contains("ERROR") || l.contains("error") {
            CLASH_THEME.danger
        } else if l.contains("WARN") || l.contains("warn") {
            CLASH_THEME.warning
        } else if l.contains("DEBUG") {
            CLASH_THEME.muted
        } else {
            CLASH_THEME.text
        };
        Line::from(Span::styled(l.as_str(), Style::default().fg(color)))
    }).collect();

    let pause_str = if app.log_paused { "⏸" } else { "▶" };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLASH_THEME.border))
        .title(Span::styled(" Logs ", Style::default().fg(CLASH_THEME.primary).bold()))
        .title_bottom(Span::styled(format!(" p:{}  mouse scroll to navigate ", pause_str), Style::default().fg(CLASH_THEME.muted)));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(lines).style(Style::default().bg(CLASH_THEME.surface)), inner);
}

fn render_help(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLASH_THEME.primary).bg(CLASH_THEME.bg))
        .style(Style::default().bg(CLASH_THEME.bg))
        .title(Span::styled(" HELP ", Style::default().fg(CLASH_THEME.primary).bold()));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from(""),
        Line::from(vec![Span::styled("  Tab/1-5        ", CLASH_THEME.primary), Span::styled("Switch tabs", CLASH_THEME.text)]),
        Line::from(vec![Span::styled("  j/k/↑↓         ", CLASH_THEME.primary), Span::styled("Navigate lists", CLASH_THEME.text)]),
        Line::from(vec![Span::styled("  Enter           ", CLASH_THEME.primary), Span::styled("Test delay / switch node", CLASH_THEME.text)]),
        Line::from(vec![Span::styled("  s               ", CLASH_THEME.primary), Span::styled("Toggle sort (Name/Delay)", CLASH_THEME.text)]),
        Line::from(vec![Span::styled("  d               ", CLASH_THEME.primary), Span::styled("Test current group delay", CLASH_THEME.text)]),
        Line::from(vec![Span::styled("  D               ", CLASH_THEME.primary), Span::styled("Test ALL groups delay", CLASH_THEME.text)]),
        Line::from(vec![Span::styled("  p               ", CLASH_THEME.primary), Span::styled("Cycle proxy mode (Rule/Global/Direct)", CLASH_THEME.text)]),
        Line::from(vec![Span::styled("  c               ", CLASH_THEME.primary), Span::styled("Close connection (Conn tab)", CLASH_THEME.text)]),
        Line::from(vec![Span::styled("  C               ", CLASH_THEME.primary), Span::styled("Close all connections", CLASH_THEME.text)]),
        Line::from(vec![Span::styled("  /               ", CLASH_THEME.primary), Span::styled("Search / filter", CLASH_THEME.text)]),
        Line::from(vec![Span::styled("  Ctrl+Arrows     ", CLASH_THEME.primary), Span::styled("Move window", CLASH_THEME.text)]),
        Line::from(vec![Span::styled("  =/-/0           ", CLASH_THEME.primary), Span::styled("Zoom in/out/reset", CLASH_THEME.text)]),
        Line::from(vec![Span::styled("  r/q/?           ", CLASH_THEME.primary), Span::styled("Refresh/Quit/Help", CLASH_THEME.text)]),
        Line::from(""),
        Line::from(Span::styled("  clash-tui  ·  Mihomo Dashboard", CLASH_THEME.muted)),
    ];
    frame.render_widget(Paragraph::new(lines).style(Style::default().fg(CLASH_THEME.text)), inner);
}

fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    fill_area(frame, area, CLASH_THEME.bg);
    let error_text = app.error_msg.as_deref().unwrap_or("");
    let search_info = if app.search_active {
        format!(" [/] Search: {} | ", app.search_query)
    } else {
        String::new()
    };
    let mode_info = if app.tab == Tab::Proxies || app.tab == Tab::Overview {
        format!("Mode: {} | Sort: {} | ", app.proxy_mode_str, if app.sort_mode { "Delay" } else { "Name" })
    } else {
        String::new()
    };
    let status = format!(" {}{}[q] Quit  [tab] Switch  [r] Refresh  [?] Help    {}", search_info, mode_info, error_text);
    let color = if app.error_msg.is_some() { CLASH_THEME.danger } else { CLASH_THEME.muted };
    let line = Line::from(Span::styled(status, Style::default().fg(color)));
    frame.render_widget(Paragraph::new(line).style(Style::default().bg(CLASH_THEME.bg)), area);
}

fn fill_area(frame: &mut Frame, area: Rect, bg: Color) {
    if area.width == 0 || area.height == 0 { return; }
    let line_str = " ".repeat(area.width as usize);
    for y in 0..area.height {
        frame.buffer_mut().set_string(area.x, area.y + y, &line_str, Style::default().bg(bg));
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
