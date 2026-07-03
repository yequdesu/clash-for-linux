use crate::ui::prelude::*;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Clear;
use ratatui::Frame;

pub(crate) fn fill_area(frame: &mut Frame, area: Rect, bg: Color) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let line_str = " ".repeat(area.width as usize);
    for y in 0..area.height {
        frame
            .buffer_mut()
            .set_string(area.x, area.y + y, &line_str, Style::default().bg(bg));
    }
}

pub(crate) fn clear_floating_area(frame: &mut Frame, area: Rect, bg: Color) {
    frame.render_widget(Clear, area);
    fill_area(frame, area, bg);
}

pub(crate) fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

pub(crate) fn scaled_bar(value: u64, max: u64, width: usize) -> String {
    if width == 0 || value == 0 || max == 0 {
        return String::new();
    }
    let filled = ((value as f64 / max as f64) * width as f64).round() as usize;
    let filled = filled.clamp(1, width);
    "█".repeat(filled)
}

pub(crate) fn traffic_summary(points: &[TrafficPoint]) -> (u64, u64, u64, u64) {
    let mut down_total = 0_u64;
    let mut up_total = 0_u64;
    let mut down_rate = 0_u64;
    let mut up_rate = 0_u64;
    for point in points {
        down_total = down_total.saturating_add(point.download_delta);
        up_total = up_total.saturating_add(point.upload_delta);
        down_rate = down_rate.max(point.down_bps);
        up_rate = up_rate.max(point.up_bps);
    }
    (down_total, up_total, down_rate, up_rate)
}

pub(crate) fn profile_interval_label(profile: &ProfileEntry) -> String {
    if profile.update_enabled == Some(false) {
        return "off".into();
    }
    if !profile.update_interval.is_empty() {
        return profile.update_interval.clone();
    }
    if !profile.interval.is_empty() {
        return profile.interval.clone();
    }
    if profile.url.starts_with("file://") {
        "off".into()
    } else {
        "12h".into()
    }
}

pub(crate) fn trunc_str(value: &str, max: usize) -> String {
    let count = value.chars().count();
    if count <= max {
        return value.to_string();
    }
    if max <= 1 {
        return "…".into();
    }
    let mut out: String = value.chars().take(max - 1).collect();
    out.push('…');
    out
}

pub(crate) fn read_os_info() -> String {
    if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
        for line in content.lines() {
            if let Some(rest) = line.strip_prefix("PRETTY_NAME=") {
                return rest.trim_matches('"').to_string();
            }
        }
    }
    "Linux".into()
}

pub(crate) fn log_line_rank(line: &str) -> u8 {
    match line
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase()
        .as_str()
    {
        "ERROR" => 3,
        "WARN" | "WARNING" => 2,
        "DEBUG" => 0,
        _ => 1,
    }
}

pub(crate) fn mode_display(mode: &str) -> String {
    match mode.to_ascii_lowercase().as_str() {
        "global" => "Global".into(),
        "direct" => "Direct".into(),
        _ => "Rule".into(),
    }
}

pub(crate) fn next_mode(mode: &str) -> String {
    match mode_display(mode).as_str() {
        "Rule" => "Global".into(),
        "Global" => "Direct".into(),
        _ => "Rule".into(),
    }
}

pub(crate) fn command_output_lines(output: &str) -> Vec<String> {
    let mut lines: Vec<String> = output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| trunc_str(line, 120))
        .collect();
    if lines.len() > 200 {
        lines = lines[lines.len() - 200..].to_vec();
    }
    lines
}

pub(crate) fn sudo_required_error(output: &str) -> bool {
    let lower = output.to_ascii_lowercase();
    lower.contains("requires root")
        || lower.contains("interactive authentication required")
        || lower.contains("authentication required")
        || lower.contains("permission denied")
        || lower.contains("operation not permitted")
        || lower.contains("not running as root")
        || lower.contains("sudo")
}

pub(crate) fn redact_sensitive_output(output: &str) -> String {
    output
        .lines()
        .map(redact_sensitive_line)
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn redact_sensitive_line(line: &str) -> String {
    let lower = line.to_ascii_lowercase();
    if let Some(pos) = lower.find("\"secret\"") {
        if let Some(colon) = line[pos..].find(':') {
            let split = pos + colon + 1;
            return format!("{} ********", &line[..split]);
        }
    }
    if let Some(pos) = lower.find("secret:") {
        let split = pos + "secret:".len();
        return format!("{} ********", &line[..split]);
    }
    line.to_string()
}

pub(crate) fn initial_tab_for_profiles(profile_count: usize, default_page: &str) -> Tab {
    if !is_auto_default_page(default_page) {
        if let Some(tab) = Tab::from_setting_key(default_page) {
            return tab;
        }
    }
    if profile_count == 0 {
        Tab::Subscriptions
    } else {
        Tab::Proxies
    }
}

pub(crate) fn next_default_page(current: &str) -> String {
    if is_auto_default_page(current) {
        return Tab::Subscriptions.setting_key().into();
    }
    match Tab::from_setting_key(current) {
        Some(Tab::Help) | None => "auto".into(),
        Some(tab) => tab.next().setting_key().to_string(),
    }
}

pub(crate) fn is_auto_default_page(raw: &str) -> bool {
    raw.trim().eq_ignore_ascii_case("auto")
}
