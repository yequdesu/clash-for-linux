use crate::ui::prelude::*;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::i18n::Msg;

use crate::ui::components::action_bar::*;
use crate::ui::components::panel::Panel;

const NETWORK_KERNEL_ACTION_IDS: &[&str] = &[
    "network.status",
    "network.doctor",
    "network.config_doctor",
    "network.start",
    "network.stop",
    "network.restart",
];
const NETWORK_TUN_ACTION_IDS: &[&str] = &["network.tun.on", "network.tun.off"];
const NETWORK_PROXY_ACTION_IDS: &[&str] = &["network.proxy.on", "network.env", "network.proxy.off"];
const NETWORK_DESKTOP_ACTION_IDS: &[&str] = &[
    "network.desktop.status",
    "network.desktop.on",
    "network.desktop.off",
];

pub(crate) fn render_network(frame: &mut Frame, area: Rect, app: &mut App) {
    if area.height < 12 {
        return;
    }

    let rows = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(6),
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

    // Row 2: grouped actions + diagnostics/status details
    let bottom = if rows[2].width >= 100 && rows[2].height >= 10 {
        Layout::horizontal([Constraint::Ratio(3, 5), Constraint::Ratio(2, 5)]).split(rows[2])
    } else {
        Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(rows[2])
    };
    render_network_actions(frame, bottom[0], app);

    let info_rows =
        Layout::vertical([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(bottom[1]);
    let conn_lines = vec![Line::from(vec![Span::styled(
        format!(
            "  Active: {}   Total: {}",
            app.connections_active, app.connections_total
        ),
        CLASH_THEME.text,
    )])];
    Panel::new(app.t(Msg::PageConnections)).render(frame, info_rows[0], conn_lines);

    let sys_lines = vec![Line::from(vec![
        Span::styled(format!("  OS: {}  ", app.os_info), CLASH_THEME.text),
        Span::styled(format!("Arch: {}", app.arch_info), CLASH_THEME.text),
    ])];
    Panel::new(app.t(Msg::NetworkSystemInfo)).render(frame, info_rows[1], sys_lines);
}

pub(crate) fn render_network_actions(frame: &mut Frame, area: Rect, app: &mut App) {
    let inner = Panel::new(app.t(Msg::NetworkActions)).render_block(frame, area);
    fill_area(frame, inner, CLASH_THEME.surface);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let groups = [
        (app.t(Msg::ActionsKernel), NETWORK_KERNEL_ACTION_IDS),
        (app.t(Msg::ActionsTun), NETWORK_TUN_ACTION_IDS),
        (app.t(Msg::ActionsProxy), NETWORK_PROXY_ACTION_IDS),
        (app.t(Msg::ActionsDesktop), NETWORK_DESKTOP_ACTION_IDS),
    ];
    let mut y = inner.y;
    let max_y = inner.y.saturating_add(inner.height);
    for (title, action_ids) in groups {
        if y >= max_y {
            break;
        }
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!("  {}", title),
                CLASH_THEME.primary,
            )))
            .style(Style::default().bg(CLASH_THEME.surface)),
            Rect::new(inner.x, y, inner.width, 1),
        );
        y = y.saturating_add(1);
        let buttons = action_buttons_from_ids(app, action_ids);
        let button_height = action_buttons_needed_height(inner.width, &buttons)
            .max(1)
            .min(max_y.saturating_sub(y));
        if button_height == 0 {
            break;
        }
        render_action_buttons(
            frame,
            app,
            Rect::new(inner.x, y, inner.width, button_height),
            &buttons,
        );
        y = y.saturating_add(button_height).saturating_add(1);
    }
}
