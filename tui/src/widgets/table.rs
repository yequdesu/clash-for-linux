use ratatui::layout::{Constraint, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Row, Table, TableState};
use ratatui::Frame;

use crate::theme::CLASH_THEME;

pub fn render_simple_table(
    frame: &mut Frame,
    area: Rect,
    headers: &[&str],
    rows: &[Vec<String>],
    state: &mut TableState,
    highlight_idx: usize,
) {
    let cols = headers.len();
    let header_row = Row::new(Vec::from_iter(headers.iter().copied()))
        .style(Style::default().fg(CLASH_THEME.muted));

    let data_rows: Vec<Row> = rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let style = if i == highlight_idx {
                Style::default()
                    .fg(CLASH_THEME.text)
                    .bg(CLASH_THEME.primary)
            } else {
                Style::default()
                    .fg(CLASH_THEME.text)
                    .bg(CLASH_THEME.surface)
            };
            Row::new(row.iter().map(|s| s.as_str()).collect::<Vec<_>>()).style(style)
        })
        .collect();

    let col_width = area.width.saturating_sub(4) / cols.max(1) as u16;
    let widths: Vec<Constraint> = (0..cols).map(|_| Constraint::Length(col_width)).collect();
    let table = Table::new(data_rows, &widths).header(header_row);

    frame.render_stateful_widget(table, area, state);
}

pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max.saturating_sub(1)).collect::<String>())
    }
}
