use crate::ui::prelude::*;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::action_registry::{self};
use crate::i18n::Msg;
use crate::mouse::HitboxAction;

use crate::ui::components::action_bar::*;

pub(crate) fn render_logs(frame: &mut Frame, area: Rect, app: &mut App) {
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
        (app.ui_state
            .logs
            .scroll
            .min(visible_logs.len().saturating_sub(1)))
        .saturating_sub(max_lines.saturating_sub(1))
    } else {
        0
    };
    let end = (start + max_lines).min(visible_logs.len());

    let lines: Vec<Line> = if visible_logs.is_empty() {
        let hint = if app.ui_settings.language.is_zh() {
            "  在 Network 页运行诊断，或启动内核后刷新。"
        } else {
            "  Run diagnostics from Network, or refresh after the kernel starts."
        };
        vec![
            Line::from(Span::styled(app.t(Msg::LogsEmpty), CLASH_THEME.muted)),
            Line::from(Span::styled(hint, CLASH_THEME.muted)),
        ]
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

    let pause_str = if app.ui_state.logs.paused {
        "⏸"
    } else {
        "▶"
    };
    let search_suffix =
        if app.ui_state.logs.search_active || !app.ui_state.logs.search_query.is_empty() {
            format!("  search:{} ", app.ui_state.logs.search_query)
        } else {
            String::new()
        };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(CLASH_THEME.border)
                .bg(CLASH_THEME.surface),
        )
        .style(Style::default().bg(CLASH_THEME.surface))
        .title(Span::styled(
            format!(" {} ", app.t(Msg::PageLogs)),
            Style::default().fg(CLASH_THEME.primary).bold(),
        ))
        .title_bottom(Span::styled(
            format!(
                " p:{}  f:{}+  c:clear  /:search{}  scroll:navigate ",
                pause_str,
                app.ui_state.logs.level.label(),
                search_suffix
            ),
            Style::default().fg(CLASH_THEME.muted),
        ));
    let inner = block.inner(log_area);
    frame.render_widget(block, log_area);
    fill_area(frame, inner, CLASH_THEME.surface);
    app.ui_state
        .hitboxes
        .register(inner, HitboxAction::ScrollLogs);
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(CLASH_THEME.surface)),
        inner,
    );
    if let Some(actions_area) = actions_area {
        render_log_actions(frame, actions_area, app);
    }
}

pub(crate) fn render_log_actions(frame: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(CLASH_THEME.border)
                .bg(CLASH_THEME.surface),
        )
        .style(Style::default().bg(CLASH_THEME.surface))
        .title(Span::styled(
            format!(" {} ", app.t(Msg::LogsActions)),
            Style::default().fg(CLASH_THEME.primary).bold(),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    fill_area(frame, inner, CLASH_THEME.surface);
    let buttons = action_buttons_from_specs(app, action_registry::log_action_specs());
    render_action_buttons(frame, app, inner, &buttons);
}
