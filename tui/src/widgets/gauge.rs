use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::theme::CLASH_THEME;

/// Render a horizontal gauge (progress bar)
pub fn render_gauge(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: u64,
    max: u64,
    color: Color,
) {
    let ratio = if max > 0 {
        (value as f64 / max as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let bar_width = area.width.saturating_sub(label.len() as u16 + 4) as usize;
    let filled = (bar_width as f64 * ratio) as usize;
    let empty = bar_width.saturating_sub(filled);

    let bar = format!(
        "{}{}",
        "█".repeat(filled),
        "░".repeat(empty)
    );

    let line = Line::from(vec![
        Span::styled(format!("{} ", label), Style::default().fg(CLASH_THEME.muted)),
        Span::styled(bar, Style::default().fg(color).bg(CLASH_THEME.surface)),
        Span::styled(
            format!(" {:.0}%", ratio * 100.0),
            Style::default().fg(CLASH_THEME.text),
        ),
    ]);

    frame.render_widget(Paragraph::new(line), area);
}

/// Render a vertical bar chart for delay visualization
pub fn render_delay_bar(
    frame: &mut Frame,
    area: Rect,
    delay: i64, // ms, -1 means no data
) {
    if area.width < 2 {
        return;
    }

    let filled = if delay <= 0 {
        0
    } else if delay <= 50 {
        (area.width.saturating_sub(1) as f64 * 0.25) as usize
    } else if delay <= 100 {
        (area.width.saturating_sub(1) as f64 * 0.5) as usize
    } else if delay <= 200 {
        (area.width.saturating_sub(1) as f64 * 0.75) as usize
    } else {
        area.width.saturating_sub(1) as usize
    };

    let color = if delay <= 0 {
        CLASH_THEME.muted
    } else if delay <= 100 {
        CLASH_THEME.accent
    } else if delay <= 200 {
        CLASH_THEME.warning
    } else {
        CLASH_THEME.danger
    };

    let empty = area.width.saturating_sub(1).saturating_sub(filled as u16) as usize;
    let bar = format!("{}{}", "█".repeat(filled), "░".repeat(empty));
    let line = Line::from(Span::styled(bar, Style::default().fg(color).bg(CLASH_THEME.surface)));
    frame.render_widget(Paragraph::new(line), area);
}
