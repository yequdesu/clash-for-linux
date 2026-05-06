use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

pub trait BackgroundEffect {
    fn update(&mut self, area: Rect, window: Rect, tick: u64);
    fn render(&self, area: Rect, buf: &mut Buffer);
}

pub struct NoopBackground;

impl BackgroundEffect for NoopBackground {
    fn update(&mut self, _area: Rect, _window: Rect, _tick: u64) {}
    fn render(&self, _area: Rect, _buf: &mut Buffer) {}
}
