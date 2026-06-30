use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use rand::Rng;

use crate::theme::CLASH_THEME;

pub trait BackgroundEffect {
    fn update(&mut self, area: Rect, window: Rect, tick: u64);
    fn render(&self, area: Rect, buf: &mut Buffer);
}

const PARTICLE_COUNT: usize = 80;
const DUST_CHARS: &[char] = &['·', '•', '∙', '⋅', '⋄', '✧', '✦', '·', '·', '·'];
const BRIGHT_CHARS: &[char] = &['◆', '◇', '⬩', '⬥', '✦'];

struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    brightness: u8,
    phase: f32,
}

pub struct ParticleField {
    particles: Vec<Particle>,
    last_area: Rect,
    last_window: Rect,
    initialized: bool,
}

impl ParticleField {
    pub fn new() -> Self {
        Self {
            particles: Vec::with_capacity(PARTICLE_COUNT),
            last_area: Rect::default(),
            last_window: Rect::default(),
            initialized: false,
        }
    }

    fn ensure_particles(&mut self, area: Rect) {
        let needed = PARTICLE_COUNT - self.particles.len();
        let mut rng = rand::thread_rng();
        for _ in 0..needed {
            self.particles.push(Particle {
                x: rng.gen_range(0.0..area.width as f32),
                y: rng.gen_range(0.0..area.height as f32),
                vx: rng.gen_range(-0.15..0.15),
                vy: rng.gen_range(-0.1..0.1),
                brightness: rng.gen_range(30..200),
                phase: rng.gen_range(0.0..std::f32::consts::TAU),
            });
        }
    }
}

impl BackgroundEffect for ParticleField {
    fn update(&mut self, area: Rect, window: Rect, tick: u64) {
        self.last_area = area;
        self.last_window = window;

        if !self.initialized {
            self.ensure_particles(area);
            self.initialized = true;
        }

        let t = tick as f32 * 0.02;
        let w = area.width as f32;
        let h = area.height as f32;

        for p in &mut self.particles {
            p.x += p.vx;
            p.y += p.vy;

            if p.x < 0.0 {
                p.x += w;
            } else if p.x >= w {
                p.x -= w;
            }
            if p.y < 0.0 {
                p.y += h;
            } else if p.y >= h {
                p.y -= h;
            }

            p.brightness = (30.0 + 170.0 * ((t + p.phase).sin() * 0.5 + 0.5)) as u8;
        }
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        for (i, p) in self.particles.iter().enumerate() {
            let px = p.x as u16;
            let py = p.y as u16;

            if px >= area.width || py >= area.height {
                continue;
            }

            let gx = area.x + px;
            let gy = area.y + py;

            if gx >= self.last_window.x
                && gx < self.last_window.x + self.last_window.width
                && gy >= self.last_window.y
                && gy < self.last_window.y + self.last_window.height
            {
                continue;
            }

            if gx >= buf.area().width || gy >= buf.area().height {
                continue;
            }

            if buf.cell((gx, gy)).map_or(true, |c| c.symbol() != " ") {
                continue;
            }

            let ch = if i % 15 == 0 && p.brightness > 140 {
                BRIGHT_CHARS[(i * 3) % BRIGHT_CHARS.len()]
            } else {
                DUST_CHARS[(i * 7 + (p.brightness as usize / 30)) % DUST_CHARS.len()]
            };

            let alpha = p.brightness as f32 / 255.0;
            let color = blend_color(CLASH_THEME.bg_outer, CLASH_THEME.primary, alpha * 0.15);
            buf.get_mut(gx, gy).set_char(ch).set_style(Style::default().fg(color));
        }
    }
}

fn blend_color(bg: Color, fg: Color, ratio: f32) -> Color {
    let r = |c: Color| -> u8 {
        match c {
            Color::Rgb(r, _, _) => r,
            _ => 0,
        }
    };
    let g = |c: Color| -> u8 {
        match c {
            Color::Rgb(_, g, _) => g,
            _ => 0,
        }
    };
    let b = |c: Color| -> u8 {
        match c {
            Color::Rgb(_, _, b) => b,
            _ => 0,
        }
    };
    let ratio = ratio.clamp(0.0, 1.0);
    Color::Rgb(
        (r(bg) as f32 + (r(fg) as f32 - r(bg) as f32) * ratio) as u8,
        (g(bg) as f32 + (g(fg) as f32 - g(bg) as f32) * ratio) as u8,
        (b(bg) as f32 + (b(fg) as f32 - b(bg) as f32) * ratio) as u8,
    )
}

pub struct NoopBackground;

impl BackgroundEffect for NoopBackground {
    fn update(&mut self, _area: Rect, _window: Rect, _tick: u64) {}
    fn render(&self, _area: Rect, _buf: &mut Buffer) {}
}
