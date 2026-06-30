use ratatui::layout::{Constraint, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Row, Table, TableState};
use ratatui::Frame;

use crate::theme::CLASH_THEME;

#[allow(dead_code)]
pub fn render_simple_table(
    frame: &mut Frame,
    area: Rect,
    headers: &[&str],
    rows: &[Vec<String>],
    state: &mut TableState,
    highlight_idx: usize,
) {
    let cols = headers.len();
    let col_width = area.width.saturating_sub(4) / cols.max(1) as u16;
    let widths: Vec<Constraint> = (0..cols)
        .map(|_| Constraint::Length(col_width))
        .collect();
    render_table_inner(frame, area, headers, rows, state, highlight_idx, &widths);
}

pub fn render_weighted_table(
    frame: &mut Frame,
    area: Rect,
    headers: &[&str],
    rows: &[Vec<String>],
    state: &mut TableState,
    highlight_idx: usize,
    weights: &[u16],
) {
    let total_weight: u16 = weights.iter().sum();
    let available = area.width.saturating_sub(4);
    let widths: Vec<Constraint> = weights
        .iter()
        .map(|&w| {
            let cw = if total_weight > 0 {
                (available as u32 * w as u32 / total_weight as u32) as u16
            } else {
                available / weights.len().max(1) as u16
            };
            Constraint::Length(cw)
        })
        .collect();
    render_table_inner(frame, area, headers, rows, state, highlight_idx, &widths);
}

fn render_table_inner(
    frame: &mut Frame,
    area: Rect,
    headers: &[&str],
    rows: &[Vec<String>],
    state: &mut TableState,
    highlight_idx: usize,
    widths: &[Constraint],
) {
    let header_row = Row::new(Vec::from_iter(headers.iter().copied()))
        .style(Style::default().fg(CLASH_THEME.primary_dim).bg(CLASH_THEME.surface));

    let data_rows: Vec<Row> = rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let bg = if i % 2 == 0 {
                CLASH_THEME.surface
            } else {
                CLASH_THEME.surface_alt
            };
            let style = if i == highlight_idx {
                Style::default()
                    .fg(CLASH_THEME.text)
                    .bg(CLASH_THEME.primary_dim)
            } else {
                Style::default().fg(CLASH_THEME.text).bg(bg)
            };
            Row::new(row.iter().map(|s| s.as_str()).collect::<Vec<_>>()).style(style)
        })
        .collect();

    let table = Table::new(data_rows, widths)
        .header(header_row)
        .style(Style::default().bg(CLASH_THEME.surface));

    frame.render_stateful_widget(table, area, state);
}

pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!(
            "{}…",
            s.chars().take(max.saturating_sub(1)).collect::<String>()
        )
    }
}
