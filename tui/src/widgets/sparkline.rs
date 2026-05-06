use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::theme::CLASH_THEME;

const SPARK_CHARS: &[char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Render a sparkline (mini line chart) from a series of values
pub fn render_sparkline(
    frame: &mut Frame,
    area: Rect,
    values: &[u64],
    color: Color,
) {
    if values.is_empty() || area.width == 0 {
        return;
    }

    let max_val = values.iter().max().copied().unwrap_or(1).max(1);
    let width = area.width as usize;

    let sampled: Vec<char> = if values.len() <= width {
        values.iter()
            .map(|&v| {
                let ratio = v as f64 / max_val as f64;
                let idx = (ratio * 7.0).clamp(0.0, 7.0) as usize;
                SPARK_CHARS[idx.min(7)]
            })
            .collect()
    } else {
        let step = values.len() as f64 / width as f64;
        (0..width)
            .map(|i| {
                let idx = (i as f64 * step) as usize;
                let v = values[idx.min(values.len() - 1)];
                let ratio = v as f64 / max_val as f64;
                let cidx = (ratio * 7.0).clamp(0.0, 7.0) as usize;
                SPARK_CHARS[cidx.min(7)]
            })
            .collect()
    };

    let spark_str: String = sampled.into_iter().collect();
    let spark_width = spark_str.chars().count() as u16;
    let line = Line::from(Span::styled(spark_str, Style::default().fg(color).bg(CLASH_THEME.sparkline_bg)));

    if spark_width < area.width {
        let pad = " ".repeat((area.width - spark_width) as usize);
        let padded = format!("{}{}", spark_str, pad);
        let line = Line::from(Span::styled(padded, Style::default().fg(color).bg(CLASH_THEME.sparkline_bg)));
        frame.render_widget(Paragraph::new(line).style(Style::default().bg(CLASH_THEME.sparkline_bg)), area);
    } else {
        frame.render_widget(Paragraph::new(line).style(Style::default().bg(CLASH_THEME.sparkline_bg)), area);
    }
}
