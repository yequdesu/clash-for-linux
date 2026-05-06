use ratatui::layout::Rect;
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::theme::CLASH_THEME;

pub struct Card<'a> {
    pub title: &'a str,
}

impl<'a> Card<'a> {
    pub fn new(title: &'a str) -> Self {
        Self { title }
    }

    pub fn render(self, frame: &mut Frame, area: Rect, lines: Vec<Line<'a>>) {
        let inner = self.render_block(frame, area);
        if inner.width > 0 && inner.height > 0 {
            let empty = " ".repeat(inner.width as usize);
            for y in 0..inner.height {
                frame.buffer_mut().set_string(
                    inner.x, inner.y + y, &empty,
                    Style::default().bg(CLASH_THEME.surface),
                );
            }
        }
        let content = Paragraph::new(lines)
            .style(Style::default().fg(CLASH_THEME.text).bg(CLASH_THEME.surface));
        frame.render_widget(content, inner);
    }

    pub fn render_block(self, frame: &mut Frame, area: Rect) -> Rect {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(CLASH_THEME.border).bg(CLASH_THEME.surface))
            .style(Style::default().bg(CLASH_THEME.surface))
            .title(
                Span::styled(
                    format!(" {} ", self.title),
                    Style::default().fg(CLASH_THEME.primary).bold(),
                ),
            )
            .title_bottom("");
        let inner = block.inner(area);
        frame.render_widget(block, area);
        inner
    }
}

#[allow(dead_code)]
pub fn status_line(status: &str, label: &str) -> Line<'static> {
    let d = if matches!(status, "running" | "ok" | "active" | "Active") { "●" } else { "○" };

    let dot_color = match status {
        "running" | "ok" | "active" | "Active" => CLASH_THEME.accent,
        "error" | "Error" => CLASH_THEME.danger,
        _ => CLASH_THEME.warning,
    };

    Line::from(vec![
        Span::styled(format!("  {}  ", d), Style::default().fg(dot_color)),
        Span::styled(label.to_string(), Style::default().fg(CLASH_THEME.text)),
        Span::styled(format!("  {}", status), Style::default().fg(CLASH_THEME.muted)),
    ])
}
