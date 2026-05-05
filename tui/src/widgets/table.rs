#![allow(dead_code)]

use ratatui::layout::{Constraint, Rect};
use ratatui::style::Style;
use ratatui::text::Span;
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
    let cols: usize = headers.len();
    let header_row = Row::new(headers.to_vec())
        .style(Style::default().fg(CLASH_THEME.muted));

    let data_rows: Vec<Row> = rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let style = if i == highlight_idx {
                Style::default().fg(CLASH_THEME.text).bg(CLASH_THEME.primary)
            } else {
                Style::default().fg(CLASH_THEME.text).bg(CLASH_THEME.surface)
            };
            Row::new(row.clone()).style(style)
        })
        .collect();

    let widths: Vec<Constraint> = (0..cols)
        .map(|_| Constraint::Length(18))
        .collect();
    let table = Table::new(data_rows, &widths).header(header_row);

    frame.render_stateful_widget(table, area, state);
}

pub fn cell_span(text: &str, color: ratatui::style::Color) -> Span<'_> {
    Span::styled(text.to_string(), Style::default().fg(color).bg(CLASH_THEME.surface))
}

pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max - 1).collect::<String>())
    }
}
