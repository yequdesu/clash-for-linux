use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, TableState};
use ratatui::Frame;
use std::sync::mpsc;

use crate::api::{ApiClient, Connection, ProxyGroup, TrafficInfo};
use crate::background::{BackgroundEffect, NoopBackground};
use crate::config::Config;
use crate::event::DataEvent;
use crate::theme::CLASH_THEME;
use crate::window::WindowState;
use crate::widgets::tab_bar::Tab;

pub struct App {
    pub tab: Tab,
    pub should_quit: bool,
    pub show_help: bool,
    pub error_msg: Option<String>,
    pub tick_count: u64,
    pub last_refresh: i64,

    pub kernel_running: bool,
    pub kernel_version: String,
    pub kernel_mode: String,
    pub kernel_uptime: String,
    pub tun_enabled: bool,

    pub traffic: TrafficInfo,
    pub traffic_history: Vec<u64>,

    pub proxy_groups: Vec<ProxyGroup>,
    pub proxy_selected: usize,
    pub proxy_group_selected: usize,
    pub proxy_scroll_offset: usize,
    pub proxy_state: TableState,
    pub proxy_content_y: u16,
    pub proxy_content_x: u16,
    pub proxy_content_w: u16,
    pub proxy_content_h: u16,
    pub proxy_mode_y: u16,

    pub connections: Vec<Connection>,
    pub connections_active: usize,
    pub connections_total: usize,
    pub connection_state: TableState,
    pub connection_selected: usize,

    pub logs: Vec<String>,
    pub log_paused: bool,
    pub log_level_filter: String,
    pub log_scroll: usize,

    pub subscriptions: Vec<crate::api::SubscriptionInfo>,
    pub sub_selected: usize,
    pub sub_active_id: usize,

    pub memory_bytes: u64,
    pub memory_limit: u64,

    pub confirm_action: bool,
    pub confirm_timer: u16,

    pub window: WindowState,
    pub background: Box<dyn BackgroundEffect>,

    pub api: ApiClient,
    pub config: Config,
    pub rt: tokio::runtime::Handle,
    pub data_tx: mpsc::Sender<DataEvent>,
}

impl App {
    pub fn new(config: Config, rt: tokio::runtime::Handle, data_tx: mpsc::Sender<DataEvent>) -> Self {
        let api = ApiClient::new(config.api_url.clone(), config.api_key.clone());

        Self {
            tab: Tab::Overview,
            should_quit: false,
            show_help: false,
            error_msg: None,
            tick_count: 0,
            last_refresh: 0,

            kernel_running: false,
            kernel_version: String::new(),
            kernel_mode: String::from("rule"),
            kernel_uptime: String::new(),
            tun_enabled: false,

            traffic: TrafficInfo { up: 0, down: 0 },
            traffic_history: Vec::new(),

            proxy_groups: Vec::new(),
            proxy_selected: 0,
            proxy_group_selected: 0,
            proxy_scroll_offset: 0,
            proxy_state: TableState::default(),
            proxy_content_y: 0,
            proxy_content_x: 0,
            proxy_content_w: 0,
            proxy_content_h: 0,
            proxy_mode_y: 0,

            connections: Vec::new(),
            connections_active: 0,
            connections_total: 0,
            connection_state: TableState::default(),
            connection_selected: 0,

            logs: Vec::new(),
            log_paused: false,
            log_level_filter: String::from("ALL"),
            log_scroll: 0,

            subscriptions: Vec::new(),
            sub_selected: 0,
            sub_active_id: 0,

            memory_bytes: 0,
            memory_limit: 0,

            confirm_action: false,
            confirm_timer: 0,

            window: WindowState::load(),
            background: Box::new(NoopBackground),

            api,
            config,
            rt,
            data_tx,
        }
    }

    pub fn on_tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);

        let now = chrono::Utc::now().timestamp();
        let interval = if self.tab == Tab::Connections || self.tab == Tab::Overview { 2 } else { 5 };
        if now - self.last_refresh >= interval {
            self.last_refresh = now;
            self.refresh_data();
        }

        if self.confirm_action {
            self.confirm_timer += 1;
            if self.confirm_timer > 60 {
                self.confirm_action = false;
                self.confirm_timer = 0;
            }
        }
    }

    pub fn refresh_data(&mut self) {
        let api = self.api.clone();
        let tx = self.data_tx.clone();

        // Fetch version
        {
            let api = api.clone();
            let tx = tx.clone();
            self.rt.spawn(async move {
                let result = api.get_version().await;
                let _ = tx.send(DataEvent::VersionFetched(result));
            });
        }

        // Fetch proxies
        {
            let api = api.clone();
            let tx = tx.clone();
            self.rt.spawn(async move {
                let result = api.get_proxies().await;
                let _ = tx.send(DataEvent::ProxiesFetched(result));
            });
        }

        // Fetch traffic
        {
            let api = api.clone();
            let tx = tx.clone();
            self.rt.spawn(async move {
                let result = api.get_traffic().await;
                let _ = tx.send(DataEvent::TrafficFetched(result));
            });
        }

        // Fetch connections
        {
            let api = api.clone();
            let tx = tx.clone();
            self.rt.spawn(async move {
                let result = api.get_connections().await;
                let _ = tx.send(DataEvent::ConnectionsFetched(result));
            });
        }

        // Fetch memory
        {
            let api = api.clone();
            let tx = tx.clone();
            self.rt.spawn(async move {
                let result = api.get_memory().await;
                let _ = tx.send(DataEvent::MemoryFetched(result));
            });
        }

        // Fetch config
        {
            let api = api.clone();
            let tx = tx.clone();
            self.rt.spawn(async move {
                let result = api.get_config().await;
                let _ = tx.send(DataEvent::ConfigFetched(result));
            });
        }

        // Fetch logs from file
        {
            let tx = tx.clone();
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            let log_path = format!("{}/.clashctl/logs/mihomo.log", home);
            self.rt.spawn(async move {
                let result = tokio::task::spawn_blocking(move || -> Result<Vec<String>, String> {
                    let content = std::fs::read_to_string(&log_path)
                        .map_err(|e| format!("Cannot read log: {}", e))?;
                    let lines: Vec<&str> = content.lines().collect();
                    let start = lines.len().saturating_sub(500);
                    Ok(lines[start..].iter().map(|s| s.to_string()).collect())
                }).await.map_err(|e| e.to_string()).and_then(|r| r);
                let _ = tx.send(DataEvent::LogsFetched(result));
            });
        }

        // Fetch subscriptions from profiles.yaml
        {
            let api = api.clone();
            let tx = tx.clone();
            self.rt.spawn(async move {
                let result = api.get_subscriptions().await;
                let _ = tx.send(DataEvent::SubscriptionsFetched(result));
            });
        }
    }

    pub fn apply_data_event(&mut self, event: DataEvent) {
        match event {
            DataEvent::VersionFetched(Ok(v)) => {
                self.kernel_running = true;
                self.kernel_version = v;
            }
            DataEvent::VersionFetched(Err(e)) => {
                self.kernel_running = false;
                self.log_api_error(format!("Version fetch failed: {}", e));
            }
            DataEvent::ProxiesFetched(Ok(resp)) => {
                self.proxy_groups = resp.get_groups();
            }
            DataEvent::ProxiesFetched(Err(e)) => {
                self.log_api_error(format!("Proxies fetch failed: {}", e));
            }
            DataEvent::TrafficFetched(Ok(t)) => {
                let down = t.down;
                self.traffic = t;
                self.traffic_history.push(down);
                if self.traffic_history.len() > 60 {
                    self.traffic_history.remove(0);
                }
            }
            DataEvent::TrafficFetched(Err(e)) => {
                self.log_api_error(format!("Traffic fetch failed: {}", e));
            }
            DataEvent::ConnectionsFetched(Ok(conns)) => {
                self.connections = conns;
                self.connections_total = self.connections.len();
                self.connections_active = self.connections.iter()
                    .filter(|c| c.dl_speed() > 0 || c.ul_speed() > 0)
                    .count();
            }
            DataEvent::ConnectionsFetched(Err(e)) => {
                self.log_api_error(format!("Connections fetch failed: {}", e));
            }
            DataEvent::MemoryFetched(Ok(m)) => {
                self.memory_bytes = m.inuse.unwrap_or(0);
                self.memory_limit = m.oslimit.unwrap_or(0);
            }
            DataEvent::MemoryFetched(Err(e)) => {
                self.log_api_error(format!("Memory fetch failed: {}", e));
            }
            DataEvent::ConfigFetched(Ok(c)) => {
                self.kernel_mode = c.mode.unwrap_or_else(|| "rule".into());
                self.tun_enabled = c.tun.as_ref().and_then(|t| t.enable).unwrap_or(false);
            }
            DataEvent::ConfigFetched(Err(e)) => {
                self.log_api_error(format!("Config fetch failed: {}", e));
            }
            DataEvent::ModeSet(Ok(())) => {
                self.refresh_data();
            }
            DataEvent::ModeSet(Err(e)) => {
                self.log_api_error(format!("Mode change failed: {}", e));
            }
            DataEvent::ProxySwitched(Ok(())) => {
                self.refresh_data();
            }
            DataEvent::ProxySwitched(Err(e)) => {
                self.log_api_error(format!("Proxy switch failed: {}", e));
            }
            DataEvent::ConnectionClosed(Ok(())) => {
                self.refresh_data();
            }
            DataEvent::ConnectionClosed(Err(e)) => {
                self.log_api_error(format!("Connection close failed: {}", e));
            }
            DataEvent::LogsFetched(Ok(lines)) => {
                if !self.log_paused {
                    self.logs = lines;
                    if self.logs.len() > 1000 {
                        self.logs.drain(0..self.logs.len() - 1000);
                    }
                }
            }
            DataEvent::LogsFetched(Err(e)) => {
                self.log_api_error(format!("Log file read failed: {}", e));
            }
            DataEvent::SubscriptionsFetched(Ok((subs, active_id))) => {
                self.subscriptions = subs;
                self.sub_active_id = active_id;
            }
            DataEvent::SubscriptionsFetched(Err(e)) => {
                self.log_api_error(format!("Subscription read failed: {}", e));
            }
            DataEvent::DelayTested(name, Ok(delay)) => {
                for group in &mut self.proxy_groups {
                    for proxy in &mut group.proxies {
                        if proxy.name == name {
                            proxy.delay = delay;
                        }
                    }
                }
            }
            DataEvent::DelayTested(name, Err(e)) => {
                self.log_api_error(format!("Delay test for {} failed: {}", name, e));
            }
            _ => {}
        }
    }

    fn log_api_error(&mut self, msg: String) {
        let line = format!("[TUI] {}", msg);
        self.logs.push(line);
        if self.logs.len() > 1000 {
            self.logs.drain(0..self.logs.len() - 1000);
        }
        self.error_msg = Some(msg);
    }

    pub fn next_tab(&mut self) {
        self.tab = self.tab.next();
        self.show_help = false;
    }

    pub fn prev_tab(&mut self) {
        self.tab = self.tab.prev();
        self.show_help = false;
    }

    pub fn select_down(&mut self) {
        match self.tab {
            Tab::Proxies => {
                if self.proxy_groups.is_empty() {
                    return;
                }
                let g = &self.proxy_groups[self.proxy_group_selected.min(self.proxy_groups.len() - 1)];
                if self.proxy_selected + 1 < g.proxies.len() {
                    self.proxy_selected += 1;
                } else if self.proxy_group_selected + 1 < self.proxy_groups.len() {
                    self.proxy_group_selected += 1;
                    self.proxy_selected = 0;
                }
                self.ensure_proxy_visible();
            }
            Tab::Connections => {
                self.connection_selected = (self.connection_selected + 1).min(self.connections.len().saturating_sub(1));
            }
            Tab::Subscriptions => {
                self.sub_selected = (self.sub_selected + 1).min(self.subscriptions.len().saturating_sub(1));
            }
            Tab::Logs => {
                self.log_scroll = self.log_scroll.saturating_add(1);
            }
            Tab::Overview => {}
        }
    }

    pub fn select_up(&mut self) {
        match self.tab {
            Tab::Proxies => {
                if self.proxy_groups.is_empty() {
                    return;
                }
                if self.proxy_selected > 0 {
                    self.proxy_selected -= 1;
                } else if self.proxy_group_selected > 0 {
                    self.proxy_group_selected -= 1;
                    let g = &self.proxy_groups[self.proxy_group_selected];
                    self.proxy_selected = g.proxies.len().saturating_sub(1);
                }
                self.ensure_proxy_visible();
            }
            Tab::Connections => {
                self.connection_selected = self.connection_selected.saturating_sub(1);
            }
            Tab::Subscriptions => {
                self.sub_selected = self.sub_selected.saturating_sub(1);
            }
            Tab::Logs => {
                self.log_scroll = self.log_scroll.saturating_sub(1);
            }
            Tab::Overview => {}
        }
    }

    pub fn on_shutdown(&mut self) {
        self.window.save();
    }

    pub fn ensure_proxy_visible(&mut self) {
        let total_groups = self.proxy_groups.len();
        if total_groups == 0 {
            return;
        }
        let sel_gi = self.proxy_group_selected.min(total_groups.saturating_sub(1));
        let proxy_count = self.proxy_groups[sel_gi].proxies.len();
        let sel_pi = if proxy_count > 0 {
            self.proxy_selected.min(proxy_count.saturating_sub(1))
        } else {
            0
        };

        // Compute selected proxy's absolute line number
        let mut abs_line = 2usize;
        for gi in 0..sel_gi {
            if gi < self.proxy_groups.len() {
                abs_line += 1 + self.proxy_groups[gi].proxies.len() + 1;
            }
        }
        abs_line += 1; // group header
        abs_line += sel_pi;

        let view_h = (self.proxy_content_h as usize).max(1);

        let total_lines: usize = 2 + self.proxy_groups.iter()
            .map(|g| 1 + g.proxies.len().saturating_add(1))
            .sum::<usize>();
        let max_scroll = total_lines.saturating_sub(view_h);

        if abs_line < self.proxy_scroll_offset {
            self.proxy_scroll_offset = abs_line;
        } else if abs_line >= self.proxy_scroll_offset.saturating_add(view_h) {
            self.proxy_scroll_offset = (abs_line + 1).saturating_sub(view_h).min(max_scroll);
        }
        self.proxy_scroll_offset = self.proxy_scroll_offset.min(max_scroll);
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
    let title = format!(" Clash-Terminal · {} ", app.tab.label());
    let title_span = Span::styled(title.clone(), Style::default().fg(CLASH_THEME.primary).bold());
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

    if let Some(ref err) = app.error_msg.clone() {
        let error_area = Rect::new(content_area.x, content_area.y + content_area.height.saturating_sub(1), content_area.width, 1);
        let msg = Span::styled(err, Style::default().fg(CLASH_THEME.danger).bg(CLASH_THEME.bg));
        frame.render_widget(Paragraph::new(Line::from(msg)), error_area);
    }

    render_status_bar(frame, chunks[2], app);
}

fn render_overview(frame: &mut Frame, area: Rect, app: &mut App) {
    let rows = if area.height >= 18 {
        Layout::vertical([
            Constraint::Length(3),  // Status
            Constraint::Length(7),  // Traffic + Proxy
            Constraint::Length(4),  // System Info
            Constraint::Length(4),  // Subscription
        ])
        .split(area)
    } else {
        Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(4),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .split(area)
    };

    // Status row
    let (status_dot, status_color, status_text) = if app.kernel_running {
        ("●", CLASH_THEME.accent, format!(" Running    {}    {} mode", app.kernel_version, app.kernel_mode))
    } else {
        ("○", CLASH_THEME.danger, " Stopped    Kernel not connected".to_string())
    };
    let status_line = Line::from(vec![
        Span::styled(format!(" {} ", status_dot), Style::default().fg(status_color).bg(CLASH_THEME.surface)),
        Span::styled(status_text, Style::default().fg(CLASH_THEME.text).bg(CLASH_THEME.surface)),
    ]);
    crate::widgets::card::Card::new("Status")
        .render(frame, rows[0], vec![status_line]);

    // Traffic + Current Proxy row
    let mid = Layout::horizontal([
        Constraint::Ratio(1, 4),
        Constraint::Ratio(2, 4),
        Constraint::Ratio(1, 4),
    ])
    .split(rows[1]);

    // Traffic card
    let up_speed = format_speed(app.traffic.up);
    let down_speed = format_speed(app.traffic.down);
    let traffic_lines = vec![
        Line::from(Span::styled(
            format!("  ↑ {}  ↓ {}", up_speed, down_speed),
            Style::default().fg(CLASH_THEME.text),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("  Total: ↑ {} ↓ {}", format_bytes(app.traffic.up * 60), format_bytes(app.traffic.down * 60)),
            Style::default().fg(CLASH_THEME.muted),
        )),
    ];

    if !app.traffic_history.is_empty() {
        crate::widgets::sparkline::render_sparkline(
            frame,
            Rect::new(mid[0].x + 2, mid[0].y + 3, mid[0].width.saturating_sub(4), 2),
            &app.traffic_history,
            CLASH_THEME.primary,
        );
    }

    crate::widgets::card::Card::new("Traffic")
        .render(frame, mid[0], traffic_lines);

    // Current Proxy card
    let mut proxy_lines = Vec::new();
    if let Some(g) = app.proxy_groups.first() {
        proxy_lines.push(Line::from(Span::styled(
            format!("  ♯ {}", g.name),
            Style::default().fg(CLASH_THEME.primary),
        )));
        proxy_lines.push(Line::from(Span::styled(
            format!("  ● {} ({}ms)", g.now, if let Some(p) = g.proxies.iter().find(|p| p.name == g.now) {
                if p.delay > 0 { format!("{}", p.delay) } else { "—".into() }
            } else { "—".into() }),
            Style::default().fg(CLASH_THEME.accent),
        )));
        proxy_lines.push(Line::from(""));
    } else {
        proxy_lines.push(Line::from(Span::styled("  No proxies", CLASH_THEME.muted)));
    }
    crate::widgets::card::Card::new("Current Proxy")
        .render(frame, mid[1], proxy_lines);

    // Proxy Mode card
    let mode_lines = vec![
        Line::from(""),
        Line::from({
            let rule_style = if app.kernel_mode == "rule" {
                Style::default().fg(CLASH_THEME.accent).bold()
            } else {
                Style::default().fg(CLASH_THEME.muted)
            };
            let global_style = if app.kernel_mode == "global" {
                Style::default().fg(CLASH_THEME.accent).bold()
            } else {
                Style::default().fg(CLASH_THEME.muted)
            };
            let direct_style = if app.kernel_mode == "direct" {
                Style::default().fg(CLASH_THEME.accent).bold()
            } else {
                Style::default().fg(CLASH_THEME.muted)
            };
            vec![
                Span::styled("  [", Style::default().fg(CLASH_THEME.muted)),
            Span::styled("Rule", rule_style),
            Span::styled("] ", Style::default().fg(CLASH_THEME.muted)),
            Span::styled("Global", global_style),
            Span::styled(" ", Style::default().fg(CLASH_THEME.muted)),
            Span::styled("Direct", direct_style),
        ]}),
        Line::from(""),
    ];
    crate::widgets::card::Card::new("Proxy Mode")
        .render(frame, mid[2], mode_lines);

    // System Info + Memory row
    let sys = Layout::horizontal([
        Constraint::Ratio(1, 2),
        Constraint::Ratio(1, 2),
    ])
    .split(rows[2]);

    let sys_lines = vec![
        Line::from(Span::styled(
            "  OS: Linux  |  Arch: x86_64",
            Style::default().fg(CLASH_THEME.text),
        )),
        Line::from(Span::styled(
            format!("  TUN: {}", if app.tun_enabled { "Enabled" } else { "Disabled" }),
            if app.tun_enabled { Style::default().fg(CLASH_THEME.accent) } else { Style::default().fg(CLASH_THEME.muted) },
        )),
    ];
    crate::widgets::card::Card::new("System Info")
        .render(frame, sys[0], sys_lines);

    let mem_lines = if app.memory_limit > 0 {
        let mem_ratio = (app.memory_bytes as f64 / app.memory_limit as f64 * 100.0) as u64;
        vec![Line::from(Span::styled(
            format!("  {} / {} ({:.1}%)", format_bytes(app.memory_bytes), format_bytes(app.memory_limit), mem_ratio),
            Style::default().fg(CLASH_THEME.text),
        ))]
    } else {
        vec![Line::from(Span::styled(
            format!("  {}", format_bytes(app.memory_bytes)),
            Style::default().fg(CLASH_THEME.text),
        ))]
    };
    if app.memory_limit > 0 {
        crate::widgets::gauge::render_gauge(
            frame,
            Rect::new(sys[1].x + 2, sys[1].y + 1, sys[1].width.saturating_sub(4), 1),
            "",
            app.memory_bytes,
            app.memory_limit,
            CLASH_THEME.primary,
        );
    }
    crate::widgets::card::Card::new("Memory")
        .render(frame, sys[1], mem_lines);

    // Subscription card
    let sub_lines = if let Some(s) = app.subscriptions.iter().find(|s| s.id == app.sub_active_id) {
        vec![Line::from(Span::styled(
            format!("  ● {} ({} proxies)  —  {}", s.name, s.proxies_count, s.updated),
            Style::default().fg(CLASH_THEME.text),
        ))]
    } else {
        vec![Line::from(Span::styled(
            "  No subscriptions configured",
            CLASH_THEME.muted,
        ))]
    };
    crate::widgets::card::Card::new("Subscription")
        .render(frame, rows[3], sub_lines);
}

fn render_proxies(frame: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::vertical([
        Constraint::Length(1),  // mode bar
        Constraint::Min(3),     // content
        Constraint::Length(1),  // help
    ])
    .split(area);

    render_proxies_mode_bar(frame, chunks[0], app);
    let content = chunks[1];
    let help_area = chunks[2];

    app.proxy_mode_y = chunks[0].y;
    app.proxy_content_y = content.y;
    app.proxy_content_x = content.x;
    app.proxy_content_w = content.width;
    app.proxy_content_h = content.height;

    if app.proxy_groups.is_empty() {
        let msg = Span::styled("  No proxy groups available", CLASH_THEME.muted);
        frame.render_widget(Paragraph::new(Line::from(msg)), content);
        render_proxies_help(frame, help_area);
        return;
    }

    // Compute total virtual lines
    let total_lines: usize = 2 + app.proxy_groups.iter()
        .map(|g| 1 + g.proxies.len() + 1)
        .sum::<usize>();

    let view_h = (app.proxy_content_h as usize).max(1);
    let max_scroll = total_lines.saturating_sub(view_h);
    app.proxy_scroll_offset = app.proxy_scroll_offset.min(max_scroll);

    render_proxy_groups(frame, content, app, app.proxy_scroll_offset);
    render_proxies_help(frame, help_area);
}

fn render_proxies_mode_bar(frame: &mut Frame, area: Rect, app: &App) {
    let muted = Style::default().fg(CLASH_THEME.muted);
    let active = Style::default().fg(CLASH_THEME.accent).bold();
    let spans = vec![
        Span::styled(" Mode: ", muted),
        Span::styled("[Rule]", if app.kernel_mode == "rule" { active } else { muted }),
        Span::styled(" ", muted),
        Span::styled("Global", if app.kernel_mode == "global" { active } else { muted }),
        Span::styled(" ", muted),
        Span::styled("Direct", if app.kernel_mode == "direct" { active } else { muted }),
        Span::styled("  |  p: switch mode", muted),
    ];
    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(CLASH_THEME.bg)),
        area,
    );
}

fn render_proxy_groups(frame: &mut Frame, area: Rect, app: &App, scroll_line: usize) {
    if area.height < 2 {
        return;
    }
    let x = area.x;
    let w = area.width;
    let scroll = scroll_line as isize;
    let ay = area.y as isize;
    let ah = area.height as isize;

    // Table header (virtual line 0)
    let header_sy = ay - scroll; // screen_y = area.y - scroll
    if header_sy >= ay && header_sy < ay + ah {
        let hdr = Line::from(Span::styled(
            " ♯    Proxy Node                          Type       Delay",
            Style::default().fg(CLASH_THEME.muted),
        ));
        frame.render_widget(Paragraph::new(hdr), Rect::new(x, header_sy as u16, w, 1));
    }
    // Divider (virtual line 1)
    let div_sy = ay - scroll + 1;
    if div_sy >= ay && div_sy < ay + ah {
        let div = "─".repeat(w.saturating_sub(2) as usize);
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(div, Style::default().fg(CLASH_THEME.border)))),
            Rect::new(x + 1, div_sy as u16, w.saturating_sub(2), 1),
        );
    }

    // Compute group start lines
    let mut gs_lines: Vec<usize> = Vec::new();
    let mut nl = 2usize;
    for gi in 0..app.proxy_groups.len() {
        gs_lines.push(nl);
        nl += 1 + app.proxy_groups[gi].proxies.len() + 1;
    }

    // Render visible groups
    let vis_s = scroll_line;
    let vis_e = scroll_line + area.height as usize;
    for gi in 0..app.proxy_groups.len() {
        let gs = gs_lines[gi];
        let group = &app.proxy_groups[gi];
        let n = group.proxies.len();
        let ge = gs + 1 + n + 1;

        if ge <= vis_s || gs >= vis_e {
            continue;
        }

        let screen_y0 = ay + gs as isize - scroll;

        // Group header
        if screen_y0 >= ay && screen_y0 < ay + ah {
            let hdr = format!(" [♯ {}]", group.name);
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(hdr, Style::default().fg(CLASH_THEME.primary).bold()))),
                Rect::new(x, screen_y0 as u16, w, 1),
            );
        }

        // Box: virtual lines [gs+1, gs+1+n+1)
        let box_virt_start = gs + 1;
        let box_virt_end = gs + 1 + n + 1;
        let vis_s = scroll_line.max(box_virt_start);
        let vis_e = (scroll_line + area.height as usize).min(box_virt_end);

        if vis_s < vis_e {
            let box_screen_y = (vis_s - scroll_line) as u16 + area.y;
            let box_h = (vis_e - vis_s) as u16;

            let box_block = Block::default()
                .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
                .border_style(Style::default().fg(CLASH_THEME.border))
                .style(Style::default().bg(CLASH_THEME.surface));
            let box_area = Rect::new(x, box_screen_y, w, box_h);
            frame.render_widget(box_block, box_area);
            let inner = box_area.inner(Margin::new(1, 0));

            let skip = vis_s - box_virt_start;
            let limit = (inner.height as usize).min(n.saturating_sub(skip));
            for pi in skip..skip + limit {
                let proxy = &group.proxies[pi];
                let row = inner.y + (pi - skip) as u16;
                let is_active = proxy.name == group.now;
                let is_sel = gi == app.proxy_group_selected && pi == app.proxy_selected;
                let marker = if is_active { "●" } else { "○" };
                let mc = if is_active { CLASH_THEME.accent } else { CLASH_THEME.muted };

                let delay_s = if proxy.delay > 0 { format!("{}ms", proxy.delay) } else { "—  ".into() };
                let name = crate::widgets::table::truncate(&proxy.name, 34);
                let ptype = crate::widgets::table::truncate(&proxy.proxy_type, 8);
                let line = format!("   {} {:36} {:10} {:>6}", marker, name, ptype, delay_s);

                let style = if is_sel {
                    Style::default().fg(CLASH_THEME.text).bg(CLASH_THEME.primary)
                } else {
                    Style::default().fg(mc).bg(CLASH_THEME.surface)
                };
                frame.render_widget(
                    Paragraph::new(Line::from(Span::styled(line, style))),
                    Rect::new(inner.x, row, inner.width, 1),
                );

                if proxy.delay > 0 {
                    let bx = inner.x + 64;
                    let bw = inner.width.saturating_sub(66);
                    if bw > 4 {
                        crate::widgets::gauge::render_delay_bar(frame, Rect::new(bx, row, bw, 1), proxy.delay);
                    }
                }
            }
        }
    }
}

fn render_proxies_help(frame: &mut Frame, area: Rect) {
    let help = Line::from(Span::styled(
        " Enter:switch  d:test  D:test all  p:mode  g/G:top/bottom  wheel:scroll  click:select",
        Style::default().fg(CLASH_THEME.muted),
    ));
    frame.render_widget(
        Paragraph::new(help).style(Style::default().bg(CLASH_THEME.bg)),
        area,
    );
}

fn render_subscriptions(frame: &mut Frame, area: Rect, app: &mut App) {
    if area.height < 3 {
        return;
    }
    let list = Rect::new(area.x, area.y, area.width, area.height.saturating_sub(1));
    let help_area = Rect::new(area.x, area.y + area.height.saturating_sub(1), area.width, 1);

    if app.subscriptions.is_empty() {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled("  No subscriptions configured", CLASH_THEME.muted)),
            Line::from(Span::styled("  Use CLI: clashctl sub add <url>", CLASH_THEME.muted)),
        ];
        frame.render_widget(Paragraph::new(lines).style(Style::default().fg(CLASH_THEME.text)), list);
        render_sub_help(frame, help_area);
        return;
    }

    let active_id = app.sub_active_id;

    let mut y = 0u16;
    for (i, sub) in app.subscriptions.iter().enumerate() {
        // card: 1 top border + 1 header + 2 detail + 1 bottom border = 5 lines
        // plus 1 line gap after each card
        let card_h = 5u16;
        if y + card_h > list.height {
            break;
        }

        let is_active = sub.id == active_id;
        let is_sel = i == app.sub_selected;
        let marker = if is_active { "●" } else { "○" };
        let bg = if is_sel { CLASH_THEME.primary } else { CLASH_THEME.surface };

        let card = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if is_sel { CLASH_THEME.accent } else { CLASH_THEME.border }))
            .style(Style::default().bg(bg));
        let card_area = Rect::new(list.x + 1, list.y + y, list.width.saturating_sub(2), card_h);
        frame.render_widget(card, card_area);
        let inner = card_area.inner(Margin::new(1, 1));

        // Header: marker + name
        let hdr = format!(" {} {} (ID: {})", marker, sub.name, sub.id);
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(hdr, Style::default().fg(if is_active { CLASH_THEME.accent } else { CLASH_THEME.text })))),
            Rect::new(inner.x, inner.y, inner.width, 1),
        );
        // Detail line 1: URL
        let url_short = crate::widgets::table::truncate(&sub.url, inner.width as usize - 6);
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(format!(" URL: {}", url_short), Style::default().fg(CLASH_THEME.muted)))),
            Rect::new(inner.x, inner.y + 1, inner.width, 1),
        );
        // Detail line 2: proxies + updated + status
        let detail = format!(" Proxies: {}  |  Status: {}  |  Updated: {}",
            sub.proxies_count, sub.status,
            if sub.updated != "0" && !sub.updated.is_empty() { &sub.updated } else { "—" });
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(detail, Style::default().fg(CLASH_THEME.muted)))),
            Rect::new(inner.x, inner.y + 2, inner.width, 1),
        );

        y += card_h + 1; // card height + 1 line gap
    }
    render_sub_help(frame, help_area);
}

fn render_sub_help(frame: &mut Frame, area: Rect) {
    let help = Line::from(Span::styled(
        " a:add  u:update  U:update all  Enter:switch  d:delete  j/k:navigate",
        Style::default().fg(CLASH_THEME.muted),
    ));
    frame.render_widget(Paragraph::new(help).style(Style::default().bg(CLASH_THEME.bg)), area);
}

fn render_connections(frame: &mut Frame, area: Rect, app: &mut App) {
    if area.height < 3 {
        return;
    }
    let hdr_area = Rect::new(area.x, area.y, area.width, 1);
    let table_area = Rect::new(area.x, area.y + 1, area.width, area.height.saturating_sub(2));
    let help_area = Rect::new(area.x, area.y + area.height.saturating_sub(1), area.width, 1);

    let header = format!(
        " Active: {}  |  Total: {}  |  ↑ {}/s ↓ {}/s",
        app.connections_active,
        app.connections_total,
        format_speed(app.traffic.up),
        format_speed(app.traffic.down),
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(header, Style::default().fg(CLASH_THEME.muted)))),
        hdr_area,
    );

    if app.connections.is_empty() {
        let msg = Span::styled("  No active connections", CLASH_THEME.muted);
        frame.render_widget(Paragraph::new(Line::from(msg)), table_area);
        render_conn_help(frame, help_area);
        return;
    }

    let headers = vec!["Host", "Type", "Chain", "DL Speed"];
    let rows: Vec<Vec<String>> = app.connections.iter()
        .map(|c| {
            vec![
                crate::widgets::table::truncate(&c.host(), 25),
                c.conn_type(),
                c.chain_str(),
                format_speed(c.dl_speed()),
            ]
        })
        .collect();

    crate::widgets::table::render_simple_table(
        frame, table_area,
        &headers.iter().map(|s| *s).collect::<Vec<_>>(),
        &rows,
        &mut app.connection_state,
        app.connection_selected,
    );
    render_conn_help(frame, help_area);
}

fn render_conn_help(frame: &mut Frame, area: Rect) {
    let help = Line::from(Span::styled(
        " c:close selected  C:close all  j/k:navigate  wheel:navigate  click:close",
        Style::default().fg(CLASH_THEME.muted),
    ));
    frame.render_widget(Paragraph::new(help).style(Style::default().bg(CLASH_THEME.bg)), area);
}

fn render_logs(frame: &mut Frame, area: Rect, app: &App) {
    if area.height < 3 {
        return;
    }
    let level_hdr = Rect::new(area.x, area.y, area.width, 1);
    let log_area = Rect::new(area.x, area.y + 1, area.width, area.height.saturating_sub(2));
    let help_area = Rect::new(area.x, area.y + area.height.saturating_sub(1), area.width, 1);

    let level_header = format!(
        " Level: [{}] INFO WARN ERROR DEBUG  |  {}  |  {} lines",
        app.log_level_filter,
        if app.log_paused { "⏸ Paused" } else { "▶ Live" },
        app.logs.len(),
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(level_header, Style::default().fg(CLASH_THEME.muted)))),
        level_hdr,
    );

    if app.logs.is_empty() {
        let msg = Span::styled("  No log entries (kernel may not be running)", CLASH_THEME.muted);
        frame.render_widget(Paragraph::new(Line::from(msg)), log_area);
        render_log_help(frame, help_area);
        return;
    }

    let visible_h = log_area.height as usize;
    let total = app.logs.len();
    let scroll = app.log_scroll.min(total.saturating_sub(visible_h));

    let shown: Vec<(usize, &String)> = app.logs.iter().enumerate()
        .skip(scroll)
        .take(visible_h)
        .filter(|(_, line)| {
            if app.log_level_filter == "ALL" { return true; }
            line.starts_with("[TUI]") || line.to_uppercase().contains(&app.log_level_filter)
        })
        .collect();

    let lines: Vec<Line> = shown.iter().map(|(idx, l)| {
        let color = if l.starts_with("[TUI]") {
            CLASH_THEME.warning
        } else if l.contains("level=error") || l.contains("ERROR") || l.contains("fail") {
            CLASH_THEME.danger
        } else if l.contains("level=warn") || l.contains("WARN") || l.contains("timeout") {
            CLASH_THEME.warning
        } else if l.contains("level=debug") || l.contains("DEBUG") {
            CLASH_THEME.muted
        } else {
            CLASH_THEME.text
        };
        Line::from(Span::styled(l.clone(), Style::default().fg(color)))
    }).collect();

    frame.render_widget(Paragraph::new(lines).style(Style::default().bg(CLASH_THEME.surface)), log_area);

    // Scrollbar on the right edge
    if total > visible_h && visible_h > 0 {
        let sb_x = log_area.x + log_area.width.saturating_sub(1);
        let sb_h = log_area.height;
        let thumb_start = (scroll as f64 / total as f64 * sb_h as f64) as u16;
        let thumb_size = (visible_h as f64 / total as f64 * sb_h as f64).max(1.0) as u16;
        for sy in 0..sb_h {
            let ch = if sy >= thumb_start && sy < thumb_start + thumb_size { "█" } else { "│" };
            let color = if sy >= thumb_start && sy < thumb_start + thumb_size {
                CLASH_THEME.primary
            } else {
                CLASH_THEME.border
            };
            frame.buffer_mut().set_string(sb_x, log_area.y + sy, ch, Style::default().fg(color));
        }
    }

    render_log_help(frame, help_area);
}

fn render_log_help(frame: &mut Frame, area: Rect) {
    let help = Line::from(Span::styled(
        " f:filter level  p:toggle pause  j/k:scroll  g/G:top/bottom  wheel:scroll",
        Style::default().fg(CLASH_THEME.muted),
    ));
    frame.render_widget(Paragraph::new(help).style(Style::default().bg(CLASH_THEME.bg)), area);
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
        Line::from(vec![
            Span::styled("  Tab / ←→     ", CLASH_THEME.primary),
            Span::styled("Switch tabs", CLASH_THEME.text),
        ]),
        Line::from(vec![
            Span::styled("  1-5           ", CLASH_THEME.primary),
            Span::styled("Jump to tab directly", CLASH_THEME.text),
        ]),
        Line::from(vec![
            Span::styled("  j / k / ↑↓   ", CLASH_THEME.primary),
            Span::styled("Navigate lists", CLASH_THEME.text),
        ]),
        Line::from(vec![
            Span::styled("  /             ", CLASH_THEME.primary),
            Span::styled("Search / filter", CLASH_THEME.text),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+Arrows   ", CLASH_THEME.primary),
            Span::styled("Move window", CLASH_THEME.text),
        ]),
        Line::from(vec![
            Span::styled("  = / - / 0     ", CLASH_THEME.primary),
            Span::styled("Zoom in / out / reset", CLASH_THEME.text),
        ]),
        Line::from(vec![
            Span::styled("  r / q / ?     ", CLASH_THEME.primary),
            Span::styled("Refresh / Quit / Help", CLASH_THEME.text),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Proxies Tab:  ", CLASH_THEME.secondary),
            Span::styled("Enter=switch  d=test delay  D=test all  p=switch mode  s=sort", CLASH_THEME.text),
        ]),
        Line::from(vec![
            Span::styled("  Sub Tab:      ", CLASH_THEME.secondary),
            Span::styled("a=add  u=update  U=update all  Enter=switch  d=delete", CLASH_THEME.text),
        ]),
        Line::from(vec![
            Span::styled("  Conn Tab:     ", CLASH_THEME.secondary),
            Span::styled("c=close selected  C=close all", CLASH_THEME.text),
        ]),
        Line::from(vec![
            Span::styled("  Logs Tab:     ", CLASH_THEME.secondary),
            Span::styled("f=filter level  p=toggle pause", CLASH_THEME.text),
        ]),
        Line::from(""),
        Line::from(Span::styled("  Clash-Terminal  ·  Mihomo Dashboard", CLASH_THEME.muted)),
    ];

    frame.render_widget(Paragraph::new(lines).style(Style::default().fg(CLASH_THEME.text)), inner);
}

fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    fill_area(frame, area, CLASH_THEME.bg);

    if app.confirm_action {
        let msg = Line::from(Span::styled(
            " Confirm? [y] confirm  [any other key] cancel",
            Style::default().fg(CLASH_THEME.danger).bg(CLASH_THEME.bg),
        ));
        frame.render_widget(Paragraph::new(msg), area);
        return;
    }

    let status_dot_str = if app.kernel_running { "●" } else { "○" };
    let status_color = if app.kernel_running { CLASH_THEME.accent } else { CLASH_THEME.danger };
    let status_text = if app.kernel_running { "Running" } else { "Stopped" };

    let line = Line::from(vec![
        Span::styled(format!(" {} {} ", status_dot_str, status_text), Style::default().fg(status_color)),
        Span::styled(format!("| ↑ {} ↓ {} | [q] Quit  [tab] Switch  [r] Refresh  [?] Help",
            format_speed(app.traffic.up), format_speed(app.traffic.down)),
            Style::default().fg(CLASH_THEME.muted)),
    ]);
    frame.render_widget(Paragraph::new(line).style(Style::default().bg(CLASH_THEME.bg)), area);

    // Status dot
    let dot_area = Rect::new(area.x + 1, area.y, 1, 1);
    crate::widgets::status_dot::status_dot(frame, dot_area, app.kernel_running, app.tick_count);
}

fn fill_area(frame: &mut Frame, area: Rect, bg: Color) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let line = " ".repeat(area.width as usize);
    for y in 0..area.height {
        frame.buffer_mut().set_string(area.x, area.y + y, &line, Style::default().bg(bg));
    }
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn format_speed(bytes_per_sec: u64) -> String {
    if bytes_per_sec >= 1024 * 1024 {
        format!("{:.1} MB/s", bytes_per_sec as f64 / (1024.0 * 1024.0))
    } else if bytes_per_sec >= 1024 {
        format!("{:.1} KB/s", bytes_per_sec as f64 / 1024.0)
    } else {
        format!("{} B/s", bytes_per_sec)
    }
}
