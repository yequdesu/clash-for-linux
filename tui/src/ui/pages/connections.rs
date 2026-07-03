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

pub(crate) fn render_connections(frame: &mut Frame, area: Rect, app: &mut App) {
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
            let style = if i == app.ui_state.connections.selected_idx {
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
        .border_style(
            Style::default()
                .fg(CLASH_THEME.border)
                .bg(CLASH_THEME.surface),
        )
        .style(Style::default().bg(CLASH_THEME.surface))
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
    fill_area(frame, inner, CLASH_THEME.surface);
    register_table_row_hitboxes(app, inner, rows.len(), HitboxAction::SelectConnection);
    frame.render_stateful_widget(table, inner, &mut app.connections_table_state.clone());

    if let Some(actions_area) = actions_area {
        render_connection_actions(frame, actions_area, app);
    }
}

pub(crate) fn render_connection_actions(frame: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(CLASH_THEME.border)
                .bg(CLASH_THEME.surface),
        )
        .style(Style::default().bg(CLASH_THEME.surface))
        .title(Span::styled(
            format!(" {} ", app.t(Msg::ConnectionsActions)),
            Style::default().fg(CLASH_THEME.primary).bold(),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    fill_area(frame, inner, CLASH_THEME.surface);
    let buttons = action_buttons_from_specs(app, action_registry::connection_action_specs());
    render_action_buttons(frame, app, inner, &buttons);
}
