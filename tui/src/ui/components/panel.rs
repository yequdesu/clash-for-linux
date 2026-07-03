use ratatui::layout::Rect;
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::theme::CLASH_THEME;

pub(crate) struct Panel<'a> {
    title: &'a str,
}

impl<'a> Panel<'a> {
    pub(crate) fn new(title: &'a str) -> Self {
        Self { title }
    }

    pub(crate) fn render(self, frame: &mut Frame, area: Rect, lines: Vec<Line<'a>>) {
        let inner = self.render_block(frame, area);
        if inner.width > 0 && inner.height > 0 {
            let empty = " ".repeat(inner.width as usize);
            for y in 0..inner.height {
                frame.buffer_mut().set_string(
                    inner.x,
                    inner.y + y,
                    &empty,
                    Style::default().bg(CLASH_THEME.surface),
                );
            }
        }
        let content = Paragraph::new(lines).style(
            Style::default()
                .fg(CLASH_THEME.text)
                .bg(CLASH_THEME.surface),
        );
        frame.render_widget(content, inner);
    }

    pub(crate) fn render_block(self, frame: &mut Frame, area: Rect) -> Rect {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(
                Style::default()
                    .fg(CLASH_THEME.border)
                    .bg(CLASH_THEME.surface),
            )
            .style(Style::default().bg(CLASH_THEME.surface))
            .title(Span::styled(
                format!(" {} ", self.title),
                Style::default().fg(CLASH_THEME.primary).bold(),
            ))
            .title_bottom("");
        let inner = block.inner(area);
        frame.render_widget(block, area);
        inner
    }
}
