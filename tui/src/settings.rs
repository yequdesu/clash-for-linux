use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LanguageSetting {
    Auto,
    EnUs,
    ZhCn,
}

impl LanguageSetting {
    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::EnUs => "English",
            Self::ZhCn => "简体中文",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Auto => Self::ZhCn,
            Self::ZhCn => Self::EnUs,
            Self::EnUs => Self::Auto,
        }
    }

    pub fn is_zh(self) -> bool {
        match self {
            Self::ZhCn => true,
            Self::EnUs => false,
            Self::Auto => std::env::var("LANG")
                .map(|lang| lang.to_ascii_lowercase().starts_with("zh"))
                .unwrap_or(false),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct UiSettings {
    pub language: LanguageSetting,
    pub theme: String,
    pub default_page: String,
    pub refresh_interval_secs: u64,
    pub mouse_enabled: bool,
    pub confirm_dangerous_actions: bool,
    pub traffic_default_range: String,
    pub traffic_default_chart: String,
    pub traffic_default_dimension: String,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            language: LanguageSetting::Auto,
            theme: "default".into(),
            default_page: "auto".into(),
            refresh_interval_secs: 2,
            mouse_enabled: true,
            confirm_dangerous_actions: true,
            traffic_default_range: "24h".into(),
            traffic_default_chart: "line".into(),
            traffic_default_dimension: "route".into(),
        }
    }
}

impl UiSettings {
    pub fn load() -> Self {
        let path = settings_path();
        std::fs::read_to_string(path)
            .ok()
            .and_then(|raw| serde_yaml::from_str(&raw).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = settings_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let raw = serde_yaml::to_string(self).map_err(|e| e.to_string())?;
        std::fs::write(path, raw).map_err(|e| e.to_string())
    }

    pub fn effective_refresh_interval_secs(&self) -> u64 {
        effective_refresh_interval_secs(self.refresh_interval_secs)
    }

    pub fn refresh_interval_label(&self) -> String {
        format!("{}s", self.effective_refresh_interval_secs())
    }
}

const REFRESH_INTERVAL_OPTIONS_SECS: &[u64] = &[1, 2, 5, 10, 30, 60];

pub fn effective_refresh_interval_secs(raw: u64) -> u64 {
    if raw == 0 {
        2
    } else {
        raw.clamp(1, 300)
    }
}

pub fn next_refresh_interval_secs(raw: u64) -> u64 {
    let current = effective_refresh_interval_secs(raw);
    REFRESH_INTERVAL_OPTIONS_SECS
        .iter()
        .copied()
        .find(|option| *option > current)
        .unwrap_or(REFRESH_INTERVAL_OPTIONS_SECS[0])
}

fn settings_path() -> PathBuf {
    config_base_dir().join("clash-tui").join("settings.yaml")
}

pub fn export_dir() -> PathBuf {
    config_base_dir().join("clash-tui").join("exports")
}

fn config_base_dir() -> PathBuf {
    if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
        std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
                PathBuf::from(home).join(".config")
            })
    } else {
        std::env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
    }
}

#[cfg(test)]
mod tests {
    use super::{export_dir, next_refresh_interval_secs, LanguageSetting, UiSettings};

    #[test]
    fn ui_settings_defaults_are_safe_for_terminal_use() {
        let settings = UiSettings::default();
        assert_eq!(settings.language, LanguageSetting::Auto);
        assert_eq!(settings.theme, "default");
        assert_eq!(settings.default_page, "auto");
        assert_eq!(settings.refresh_interval_secs, 2);
        assert_eq!(settings.effective_refresh_interval_secs(), 2);
        assert!(settings.mouse_enabled);
        assert!(settings.confirm_dangerous_actions);
        assert_eq!(settings.traffic_default_range, "24h");
        assert_eq!(settings.traffic_default_chart, "line");
        assert_eq!(settings.traffic_default_dimension, "route");
    }

    #[test]
    fn language_cycles_auto_zh_english() {
        assert_eq!(LanguageSetting::Auto.next(), LanguageSetting::ZhCn);
        assert_eq!(LanguageSetting::ZhCn.next(), LanguageSetting::EnUs);
        assert_eq!(LanguageSetting::EnUs.next(), LanguageSetting::Auto);
    }

    #[test]
    fn refresh_interval_is_bounded_and_cycles_predictably() {
        let mut settings = UiSettings {
            refresh_interval_secs: 0,
            ..UiSettings::default()
        };
        assert_eq!(settings.effective_refresh_interval_secs(), 2);
        assert_eq!(settings.refresh_interval_label(), "2s");

        settings.refresh_interval_secs = 999;
        assert_eq!(settings.effective_refresh_interval_secs(), 300);

        assert_eq!(next_refresh_interval_secs(1), 2);
        assert_eq!(next_refresh_interval_secs(2), 5);
        assert_eq!(next_refresh_interval_secs(5), 10);
        assert_eq!(next_refresh_interval_secs(60), 1);
        assert_eq!(next_refresh_interval_secs(7), 10);
    }

    #[test]
    fn export_dir_is_under_clash_tui_config() {
        let dir = export_dir();
        assert!(dir.ends_with("clash-tui/exports") || dir.ends_with("clash-tui\\exports"));
    }
}
