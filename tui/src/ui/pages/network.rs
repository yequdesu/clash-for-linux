use crate::ui::prelude::*;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders};
use ratatui::Frame;

use crate::action_registry::{self};
use crate::i18n::Msg;
use crate::mouse::HitboxAction;

use crate::ui::components::action_bar::*;
use crate::ui::components::panel::Panel;

pub(crate) fn render_network(frame: &mut Frame, area: Rect, app: &mut App) {
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
    Panel::new("Kernel").render(frame, rows[0], status_lines);

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
    Panel::new(app.t(Msg::PageTraffic)).render(frame, mid_top[0], traffic_lines);

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
    Panel::new(app.t(Msg::NetworkCurrentProxy)).render(frame, mid_top[1], proxy_lines);

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
    Panel::new(app.t(Msg::PageConnections)).render(frame, left_rows[0], conn_lines);

    let sys_lines = vec![Line::from(vec![
        Span::styled(format!("  OS: {}  ", app.os_info), CLASH_THEME.text),
        Span::styled(format!("Arch: {}", app.arch_info), CLASH_THEME.text),
    ])];
    Panel::new(app.t(Msg::NetworkSystemInfo)).render(frame, left_rows[1], sys_lines);

    let mut output_lines = if app.network_output.is_empty() {
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
    let output_visible_height = mid_bot[1].height.saturating_sub(2) as usize;
    if !app.network_output.is_empty() && output_visible_height > 0 {
        let max_scroll = output_lines.len().saturating_sub(output_visible_height);
        app.network_output_scroll = app.network_output_scroll.min(max_scroll);
        output_lines = output_lines
            .into_iter()
            .skip(app.network_output_scroll)
            .take(output_visible_height)
            .collect();
    }
    app.ui_state
        .hitboxes
        .register(mid_bot[1], HitboxAction::ScrollNetworkOutput);
    Panel::new(app.t(Msg::NetworkCommandOutput)).render(frame, mid_bot[1], output_lines);
}

pub(crate) fn render_network_actions(frame: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(CLASH_THEME.border)
                .bg(CLASH_THEME.surface),
        )
        .style(Style::default().bg(CLASH_THEME.surface))
        .title(Span::styled(
            format!(" {} ", app.t(Msg::NetworkActions)),
            Style::default().fg(CLASH_THEME.primary).bold(),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    fill_area(frame, inner, CLASH_THEME.surface);
    let buttons = action_buttons_from_specs(app, action_registry::network_action_specs());
    render_action_buttons(frame, app, inner, &buttons);
}
