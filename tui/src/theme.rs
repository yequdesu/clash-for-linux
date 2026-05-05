use ratatui::style::Color;

#[allow(dead_code)]
pub struct Theme {
    pub bg_outer: Color,
    pub bg: Color,
    pub surface: Color,
    pub border: Color,
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,
    pub warning: Color,
    pub danger: Color,
    pub text: Color,
    pub muted: Color,
    pub sparkline_bg: Color,
}

pub const CLASH_THEME: Theme = Theme {
    bg_outer: Color::Rgb(15, 23, 42),
    bg: Color::Rgb(17, 24, 39),
    surface: Color::Rgb(30, 41, 59),
    border: Color::Rgb(51, 65, 85),
    primary: Color::Rgb(59, 130, 246),
    secondary: Color::Rgb(139, 92, 246),
    accent: Color::Rgb(16, 185, 129),
    warning: Color::Rgb(245, 158, 11),
    danger: Color::Rgb(239, 68, 68),
    text: Color::Rgb(249, 250, 251),
    muted: Color::Rgb(156, 163, 175),
    sparkline_bg: Color::Rgb(30, 41, 59),
};
