use crate::ui::prelude::*;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};
use ratatui::Frame;

use crate::action_registry::{self};
use crate::i18n::Msg;
use crate::mouse::HitboxAction;

use crate::ui::components::action_bar::*;
use crate::ui::hitbox::register_table_row_hitboxes;

pub(crate) fn render_proxies(frame: &mut Frame, area: Rect, app: &mut App) {
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
            let style = if i == app.ui_state.proxies.selected_idx {
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
        .border_style(
            Style::default()
                .fg(CLASH_THEME.border)
                .bg(CLASH_THEME.surface),
        )
        .style(Style::default().bg(CLASH_THEME.surface))
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
    fill_area(frame, inner, CLASH_THEME.surface);
    register_table_row_hitboxes(app, inner, rows.len(), HitboxAction::SelectProxy);
    frame.render_stateful_widget(table, inner, &mut app.proxy_table_state.clone());

    if let Some(actions_area) = actions_area {
        render_proxy_actions(frame, actions_area, app);
    }
}

pub(crate) fn render_proxy_actions(frame: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(CLASH_THEME.border)
                .bg(CLASH_THEME.surface),
        )
        .style(Style::default().bg(CLASH_THEME.surface))
        .title(Span::styled(
            format!(" {} ", app.t(Msg::ProxyActions)),
            Style::default().fg(CLASH_THEME.primary).bold(),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    fill_area(frame, inner, CLASH_THEME.surface);
    let buttons = action_buttons_from_specs(app, action_registry::proxy_action_specs());
    render_action_buttons(frame, app, inner, &buttons);
}
