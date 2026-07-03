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
use crate::ui::layout::centered_rect;

pub(crate) fn render_node_picker(frame: &mut Frame, area: Rect, app: &mut App) {
    let popup = centered_rect(area, 72, 76);
    clear_floating_area(frame, popup, CLASH_THEME.bg);

    let group = app
        .selected_proxy_group_name()
        .unwrap_or_else(|| app.t(Msg::ProxyUnknownGroup).into());
    let current = app.selected_proxy_current_node();
    let nodes = app.selected_proxy_nodes();
    let has_actions = popup.height >= 8;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLASH_THEME.primary).bg(CLASH_THEME.bg))
        .style(Style::default().bg(CLASH_THEME.bg))
        .title(Span::styled(
            format!(" {} · {} ", app.t(Msg::ProxyNodes), group),
            Style::default().fg(CLASH_THEME.primary).bold(),
        ))
        .title_bottom(Span::styled(
            format!(" {} ", app.t(Msg::ProxyNodePickerHint)),
            Style::default().fg(CLASH_THEME.muted),
        ));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    fill_area(frame, inner, CLASH_THEME.bg);

    let (table_area, action_area) = if has_actions {
        let chunks = Layout::vertical([Constraint::Min(3), Constraint::Length(3)]).split(inner);
        (chunks[0], Some(chunks[1]))
    } else {
        (inner, None)
    };
    let visible_capacity = table_area.height.saturating_sub(1) as usize;
    let start = if visible_capacity == 0 {
        0
    } else {
        app.ui_state
            .proxies
            .selected_node_idx
            .saturating_add(1)
            .saturating_sub(visible_capacity)
    };

    let rows: Vec<Row> = nodes
        .iter()
        .enumerate()
        .skip(start)
        .take(visible_capacity)
        .map(|(idx, node)| {
            let marker = if node == &current { "●" } else { " " };
            let delay = app
                .delays
                .get(node)
                .map(|d| format!("{}ms", d))
                .unwrap_or_else(|| "—".into());
            let style = if idx == app.ui_state.proxies.selected_node_idx {
                Style::default()
                    .fg(CLASH_THEME.text)
                    .bg(CLASH_THEME.primary)
            } else if node == &current {
                Style::default().fg(CLASH_THEME.accent).bg(CLASH_THEME.bg)
            } else {
                Style::default().fg(CLASH_THEME.text).bg(CLASH_THEME.bg)
            };
            Row::new(vec![marker.to_string(), node.clone(), delay]).style(style)
        })
        .collect();

    let header = Row::new(vec!["", app.t(Msg::ProxyNode), app.t(Msg::ProxyDelay)])
        .style(Style::default().fg(CLASH_THEME.muted));
    let widths = [
        Constraint::Length(2),
        Constraint::Ratio(1, 1),
        Constraint::Length(10),
    ];
    let table = Table::new(rows, widths)
        .header(header)
        .style(Style::default().bg(CLASH_THEME.bg));
    if nodes.is_empty() {
        frame.render_widget(
            Paragraph::new(app.t(Msg::ProxyNoNodes))
                .style(Style::default().fg(CLASH_THEME.muted).bg(CLASH_THEME.bg)),
            table_area,
        );
    } else {
        fill_area(frame, table_area, CLASH_THEME.bg);
        app.ui_state
            .hitboxes
            .register(table_area, HitboxAction::ScrollProxyNodes);
        let row_count = visible_capacity.min(nodes.len().saturating_sub(start));
        for row_idx in 0..row_count {
            app.ui_state.hitboxes.register(
                Rect::new(
                    table_area.x,
                    table_area.y + 1 + row_idx as u16,
                    table_area.width,
                    1,
                ),
                HitboxAction::SelectProxyNode(start + row_idx),
            );
        }
        frame.render_widget(table, table_area);
    }
    if let Some(action_area) = action_area {
        let buttons = action_buttons_from_specs(app, action_registry::node_picker_action_specs());
        render_action_buttons(frame, app, action_area, &buttons);
    }
}
