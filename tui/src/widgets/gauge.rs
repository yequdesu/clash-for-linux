use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Span;
use ratatui::widgets::{Paragraph, Widget};
use ratatui::Frame;

pub struct Gauge {
    pub label: String,
    pub ratio: f64,
    pub color: Color,
}

impl Gauge {
    pub fn new(label: &str, ratio: f64, color: Color) -> Self {
        Self { label: label.to_string(), ratio, color }
    }
}

impl Widget for &Gauge {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 6 || area.height == 0 {
            return;
        }
        let bar_width = area.width.saturating_sub(4) as usize;
        if bar_width == 0 {
            return;
        }

        let filled = (bar_width as f64 * self.ratio.clamp(0.0, 1.0)) as usize;
        let empty = bar_width.saturating_sub(filled);

        let bar_str = format!(" {}{}{}",
            "█".repeat(filled),
            "░".repeat(empty),
            format!(" {:3.0}%", self.ratio * 100.0)
        );

        let line = Span::styled(bar_str, Style::default().fg(self.color));
        let p = Paragraph::new(line);
        p.render(area, buf);
    }
}

#[allow(dead_code)]
pub fn render_gauge(frame: &mut Frame, area: Rect, label: &str, ratio: f64, color: Color) {
    let g = Gauge::new(label, ratio, color);
    frame.render_widget(&g, area);
}
