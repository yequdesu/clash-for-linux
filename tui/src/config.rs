use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub api_url: String,
    pub api_key: String,
}

impl Config {
    pub fn load() -> Self {
        let (api_url, api_key) = Self::resolve();
        Self { api_url, api_key }
    }

    fn resolve() -> (String, String) {
        if let Some((url, key)) = Self::read_config_file() {
            return (url, key);
        }

        if let Some(home) = env::var("HOME").ok() {
            for base in &["clashctl", ".clashctl"] {
                let runtime = PathBuf::from(&home).join(base).join("resources").join("runtime.yaml");
                if let Ok(content) = fs::read_to_string(&runtime) {
                    let url = Self::parse_yaml_field(&content, "external-controller", "http://127.0.0.1:9090");
                    let key = Self::parse_yaml_field(&content, "secret", "");
                    return (url, key);
                }
            }
        }

        ("http://127.0.0.1:9090".into(), String::new())
    }

    fn read_config_file() -> Option<(String, String)> {
        let config_path = Self::config_path()?;
        let content = fs::read_to_string(&config_path).ok()?;
        let mut url = None;
        let mut key = None;

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                match k.trim() {
                    "API_URL" => url = Some(v.trim().to_string()),
                    "API_KEY" => key = Some(v.trim().to_string()),
                    _ => {}
                }
            }
        }
        Some((url?, key.unwrap_or_default()))
    }

    fn config_path() -> Option<PathBuf> {
        let home = env::var("HOME").ok()?;
        let path = PathBuf::from(&home).join(".config").join("clash-tui").join("config.env");
        if path.exists() {
            return Some(path);
        }
        let local = PathBuf::from("config.env");
        if local.exists() {
            return Some(local);
        }
        None
    }

    fn parse_yaml_field(content: &str, field: &str, default: &str) -> String {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix(&format!("{}:", field)) {
                let val = rest.trim().trim_matches('"');
                if field == "external-controller" {
                    return format!("http://{}", val);
                }
                return val.to_string();
            }
        }
        default.to_string()
    }
}
