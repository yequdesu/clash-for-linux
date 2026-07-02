use ratatui::style::Color;
use std::ops::Deref;
use std::sync::atomic::{AtomicU8, Ordering};

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

pub struct DynamicTheme;

pub const CLASH_THEME: DynamicTheme = DynamicTheme;

const THEME_DEFAULT: u8 = 0;
const THEME_HIGH_CONTRAST: u8 = 1;
const THEME_CLASSIC: u8 = 2;

static CURRENT_THEME: AtomicU8 = AtomicU8::new(THEME_DEFAULT);

const DEFAULT_THEME: Theme = Theme {
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

const HIGH_CONTRAST_THEME: Theme = Theme {
    bg_outer: Color::Rgb(0, 0, 0),
    bg: Color::Rgb(8, 8, 10),
    surface: Color::Rgb(18, 18, 22),
    border: Color::Rgb(115, 115, 125),
    primary: Color::Rgb(96, 165, 250),
    secondary: Color::Rgb(244, 114, 182),
    accent: Color::Rgb(52, 211, 153),
    warning: Color::Rgb(251, 191, 36),
    danger: Color::Rgb(248, 113, 113),
    text: Color::Rgb(255, 255, 255),
    muted: Color::Rgb(209, 213, 219),
    sparkline_bg: Color::Rgb(18, 18, 22),
};

const CLASSIC_THEME: Theme = Theme {
    bg_outer: Color::Rgb(12, 16, 19),
    bg: Color::Rgb(16, 22, 26),
    surface: Color::Rgb(24, 32, 38),
    border: Color::Rgb(55, 70, 78),
    primary: Color::Rgb(125, 211, 252),
    secondary: Color::Rgb(196, 181, 253),
    accent: Color::Rgb(134, 239, 172),
    warning: Color::Rgb(253, 224, 71),
    danger: Color::Rgb(252, 165, 165),
    text: Color::Rgb(226, 232, 240),
    muted: Color::Rgb(148, 163, 184),
    sparkline_bg: Color::Rgb(24, 32, 38),
};

impl Deref for DynamicTheme {
    type Target = Theme;

    fn deref(&self) -> &Self::Target {
        current_theme()
    }
}

pub fn current_theme() -> &'static Theme {
    match CURRENT_THEME.load(Ordering::Relaxed) {
        THEME_HIGH_CONTRAST => &HIGH_CONTRAST_THEME,
        THEME_CLASSIC => &CLASSIC_THEME,
        _ => &DEFAULT_THEME,
    }
}

pub fn apply_theme_key(raw: &str) {
    CURRENT_THEME.store(theme_index(raw), Ordering::Relaxed);
}

pub fn normalize_theme_key(raw: &str) -> &'static str {
    match theme_index(raw) {
        THEME_HIGH_CONTRAST => "high-contrast",
        THEME_CLASSIC => "classic",
        _ => "default",
    }
}

pub fn theme_label(raw: &str) -> &'static str {
    match theme_index(raw) {
        THEME_HIGH_CONTRAST => "High contrast",
        THEME_CLASSIC => "Classic",
        _ => "Default",
    }
}

pub fn next_theme_key(raw: &str) -> &'static str {
    match theme_index(raw) {
        THEME_DEFAULT => "high-contrast",
        THEME_HIGH_CONTRAST => "classic",
        _ => "default",
    }
}

fn theme_index(raw: &str) -> u8 {
    match raw.trim().to_ascii_lowercase().as_str() {
        "high-contrast" | "high_contrast" | "contrast" => THEME_HIGH_CONTRAST,
        "classic" => THEME_CLASSIC,
        _ => THEME_DEFAULT,
    }
}

#[cfg(test)]
mod tests {
    use super::{apply_theme_key, current_theme, next_theme_key, normalize_theme_key, theme_label};

    #[test]
    fn theme_keys_are_stable_and_bounded() {
        assert_eq!(normalize_theme_key("default"), "default");
        assert_eq!(normalize_theme_key("contrast"), "high-contrast");
        assert_eq!(normalize_theme_key("classic"), "classic");
        assert_eq!(normalize_theme_key("unknown"), "default");

        assert_eq!(next_theme_key("default"), "high-contrast");
        assert_eq!(next_theme_key("high-contrast"), "classic");
        assert_eq!(next_theme_key("classic"), "default");
        assert_eq!(theme_label("high_contrast"), "High contrast");
    }

    #[test]
    fn applied_theme_changes_current_palette() {
        apply_theme_key("default");
        let default_bg = current_theme().bg;
        apply_theme_key("high-contrast");
        assert_ne!(current_theme().bg, default_bg);
        apply_theme_key("default");
        assert_eq!(current_theme().bg, default_bg);
    }
}
