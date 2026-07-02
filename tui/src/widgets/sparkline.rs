use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Span;
use ratatui::widgets::{Paragraph, Widget};
use ratatui::Frame;

use crate::theme::CLASH_THEME;

pub struct Sparkline {
    pub data: Vec<f64>,
    pub color: Color,
    pub max_value: f64,
}

impl Sparkline {
    pub fn new(data: Vec<f64>, color: Color) -> Self {
        let max = data.iter().cloned().fold(0.0f64, f64::max).max(1.0);
        Self {
            data,
            color,
            max_value: max,
        }
    }
}

impl Widget for &Sparkline {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let bars = "▁▂▃▄▅▆▇█";
        let chars: Vec<char> = bars.chars().collect();

        if area.width == 0 || area.height == 0 || self.data.is_empty() {
            return;
        }

        let visible: Vec<f64> = if self.data.len() > area.width as usize {
            let skip = self.data.len() - area.width as usize;
            self.data[skip..].to_vec()
        } else {
            self.data.clone()
        };

        let mut line = String::with_capacity(visible.len());
        for &val in &visible {
            let ratio = (val / self.max_value).clamp(0.0, 1.0);
            let idx = (ratio * (chars.len() - 1) as f64) as usize;
            line.push(chars[idx]);
        }

        let span = Span::styled(line, Style::default().fg(self.color));
        let p = Paragraph::new(span);
        p.render(area, buf);
    }
}

#[allow(dead_code)]
pub fn render_sparkline(frame: &mut Frame, area: Rect, data: &[f64], color: Color) {
    let s = Sparkline::new(data.to_vec(), color);
    frame.render_widget(&s, area);
}

#[derive(Clone)]
pub struct TrafficHistory {
    pub upload: Vec<f64>,
    pub download: Vec<f64>,
    max_points: usize,
}

impl TrafficHistory {
    pub fn new(max_points: usize) -> Self {
        Self {
            upload: Vec::new(),
            download: Vec::new(),
            max_points,
        }
    }

    pub fn push(&mut self, up: f64, down: f64) {
        self.upload.push(up);
        self.download.push(down);
        if self.upload.len() > self.max_points {
            self.upload.remove(0);
        }
        if self.download.len() > self.max_points {
            self.download.remove(0);
        }
    }

    #[allow(dead_code)]
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if area.width < 10 || area.height < 3 {
            return;
        }

        let mid = area.height / 2;
        let up_area = Rect::new(area.x, area.y, area.width, 1);
        let down_area = Rect::new(area.x, area.y + mid, area.width, 1);

        render_sparkline(frame, up_area, &self.upload, CLASH_THEME.accent);
        render_sparkline(frame, down_area, &self.download, CLASH_THEME.primary);
    }
}
