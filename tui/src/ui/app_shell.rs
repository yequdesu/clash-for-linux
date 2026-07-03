use crate::ui::prelude::*;

use crate::i18n::Msg;
use crate::mouse::HitboxAction;
use crate::ui::components::nav::Tab;
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use super::components::action_bar::*;
pub fn render(frame: &mut Frame, app: &mut App) {
    app.ui_state.hitboxes.clear();
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
    let title = format!(" clash-tui · {} ", app.t(app.ui_state.active_page.msg()));
    let title_span = Span::styled(
        title.clone(),
        Style::default().fg(CLASH_THEME.primary).bold(),
    );
    let decor = "─".repeat(inner.width.saturating_sub(display_width(&title)) as usize);
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
    render_top_status(frame, chunks[0], app);

    let content_shell = chunks[1];
    let content_area = if content_shell.width >= 100 && content_shell.height >= 8 {
        let shell =
            Layout::horizontal([Constraint::Length(18), Constraint::Min(20)]).split(content_shell);
        render_sidebar_nav(frame, shell[0], app);
        shell[1]
    } else {
        render_compact_nav(frame, chunks[0], app);
        content_shell
    };
    fill_area(frame, content_area, CLASH_THEME.bg);
    render_active_page(frame, content_area, app);

    render_status_bar(frame, chunks[2], app);
}

fn render_active_page(frame: &mut Frame, content_area: Rect, app: &mut App) {
    if app.show_help {
        super::pages::help::render_help(frame, content_area, app);
    } else {
        match app.ui_state.active_page {
            Tab::Network => super::pages::network::render_network(frame, content_area, app),
            Tab::Proxies => super::pages::proxies::render_proxies(frame, content_area, app),
            Tab::Subscriptions => {
                super::pages::subscriptions::render_subscriptions(frame, content_area, app)
            }
            Tab::Connections => {
                super::pages::connections::render_connections(frame, content_area, app)
            }
            Tab::Traffic => super::pages::traffic::render_traffic(frame, content_area, app),
            Tab::Logs => super::pages::logs::render_logs(frame, content_area, app),
            Tab::Settings => super::pages::settings::render_settings(frame, content_area, app),
            Tab::Help => super::pages::help::render_help(frame, content_area, app),
        }
    }
    if app.ui_state.proxies.node_picker_open {
        super::modals::node_picker::render_node_picker(frame, content_area, app);
    }
    if app.ui_state.subscriptions.prompt.is_some() {
        super::modals::subscription::render_subscription_prompt(frame, content_area, app);
    }
    if app.ui_state.settings.prompt.is_some() {
        super::modals::settings_prompt::render_settings_prompt(frame, content_area, app);
    }
    if app.ui_state.modals.sudo_prompt.is_some() {
        super::modals::sudo::render_sudo_prompt(frame, content_area, app);
    }
    if app.ui_state.modals.pending_confirmation.is_some() {
        super::modals::confirm::render_confirmation_prompt(frame, content_area, app);
    }
    if app.ui_state.command_palette.open {
        super::modals::command_palette::render_command_palette(frame, content_area, app);
    }
}

fn render_top_status(frame: &mut Frame, area: Rect, app: &App) {
    fill_area(frame, area, CLASH_THEME.bg);
    if area.height == 0 {
        return;
    }
    let tun = if app.tun_enabled { "TUN on" } else { "TUN off" };
    let kernel = if app.version.is_empty() {
        "kernel --".to_string()
    } else {
        format!("kernel {}", app.version)
    };
    let line = Line::from(vec![
        Span::styled(
            format!(" {} ", app.t(app.ui_state.active_page.msg())),
            Style::default()
                .fg(CLASH_THEME.bg)
                .bg(CLASH_THEME.primary)
                .bold(),
        ),
        Span::styled(format!("  {}  ", kernel), CLASH_THEME.text),
        Span::styled(format!("{}  ", app.mode), CLASH_THEME.muted),
        Span::styled(tun, CLASH_THEME.muted),
        Span::styled(
            format!(
                "    ↓ {}/s ↑ {}/s",
                format_bytes(app.download_rate as u64),
                format_bytes(app.upload_rate as u64)
            ),
            CLASH_THEME.text,
        ),
    ]);
    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(CLASH_THEME.bg)),
        Rect::new(area.x, area.y, area.width, 1),
    );
}

fn render_compact_nav(frame: &mut Frame, area: Rect, app: &mut App) {
    crate::ui::components::nav::render_tab_bar(
        frame,
        area,
        app.ui_state.active_page,
        app.ui_settings.language,
    );
    register_tab_hitboxes(area, app);
}

fn render_sidebar_nav(frame: &mut Frame, area: Rect, app: &mut App) {
    fill_area(frame, area, CLASH_THEME.surface);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(CLASH_THEME.border)
                .bg(CLASH_THEME.surface),
        )
        .style(Style::default().bg(CLASH_THEME.surface))
        .title(Span::styled(
            " NAV ",
            Style::default().fg(CLASH_THEME.primary).bold(),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    fill_area(frame, inner, CLASH_THEME.surface);

    for (idx, tab) in Tab::all().iter().enumerate() {
        let y = inner.y.saturating_add(idx as u16);
        if y >= inner.y.saturating_add(inner.height) {
            break;
        }
        let rect = Rect::new(inner.x, y, inner.width, 1);
        let active = *tab == app.ui_state.active_page;
        let style = if active {
            Style::default()
                .fg(CLASH_THEME.bg)
                .bg(CLASH_THEME.primary)
                .bold()
        } else {
            Style::default()
                .fg(CLASH_THEME.text)
                .bg(CLASH_THEME.surface)
        };
        let label = format!(" {} {}", idx + 1, app.t(tab.msg()));
        frame.render_widget(Paragraph::new(label).style(style), rect);
        app.ui_state
            .hitboxes
            .register(rect, HitboxAction::SwitchTab(*tab));
    }
}

pub(crate) fn register_tab_hitboxes(area: Rect, app: &mut App) {
    for hitbox in crate::ui::components::nav::tab_hitboxes(area, app.ui_settings.language) {
        app.ui_state
            .hitboxes
            .register(hitbox.area, HitboxAction::SwitchTab(hitbox.tab));
    }
}

fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    fill_area(frame, area, CLASH_THEME.bg);
    let error_text = app.error_msg.as_deref().unwrap_or("");
    let search_info = if app.ui_state.proxies.search_active {
        format!(
            " [/] {}: {} | ",
            app.t(Msg::StatusSearch),
            app.ui_state.proxies.search_query
        )
    } else {
        String::new()
    };
    let mode_info = if app.ui_state.active_page == Tab::Proxies {
        format!(
            "{}: {} | {}: {} | ",
            app.t(Msg::StatusMode),
            app.proxy_mode_str,
            app.t(Msg::StatusSort),
            if app.ui_state.proxies.sort_by_delay {
                app.t(Msg::SortDelay)
            } else {
                app.t(Msg::SortName)
            }
        )
    } else {
        String::new()
    };
    let color = if app.error_msg.is_some() {
        CLASH_THEME.danger
    } else {
        CLASH_THEME.muted
    };
    let key_style = Style::default()
        .fg(CLASH_THEME.bg)
        .bg(CLASH_THEME.secondary)
        .bold();
    let message = if app.error_msg.is_some() {
        error_text
    } else {
        app.status_msg.as_deref().unwrap_or("")
    };
    let line = Line::from(vec![
        Span::styled(
            format!(" {}{}", search_info, mode_info),
            Style::default().fg(color),
        ),
        Span::styled(" q ", key_style),
        Span::styled(
            format!(" {}  ", app.t(Msg::StatusQuit)),
            Style::default().fg(color),
        ),
        Span::styled(" tab ", key_style),
        Span::styled(
            format!(" {}  ", app.t(Msg::StatusSwitch)),
            Style::default().fg(color),
        ),
        Span::styled(" r ", key_style),
        Span::styled(
            format!(" {}  ", app.t(Msg::StatusRefresh)),
            Style::default().fg(color),
        ),
        Span::styled(" ? ", key_style),
        Span::styled(
            format!(" {}    {}", app.t(Msg::PageHelp), message),
            Style::default().fg(color),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(CLASH_THEME.bg)),
        area,
    );
}
