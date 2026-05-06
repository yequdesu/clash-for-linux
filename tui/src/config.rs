use serde::Deserialize;
use std::path::PathBuf;
use std::{env, fs};

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub api_url: String,
    pub api_key: String,
    pub mixed_port: u16,
}

impl Config {
    pub fn load() -> Self {
        let path = Self::config_path();

        let mut api_url = String::from("http://127.0.0.1:9090");
        let mut api_key = String::new();
        let mut mixed_port: u16 = 7897;

        if let Ok(content) = fs::read_to_string(&path) {
            for line in content.lines() {
                let line = line.trim();
                if line.starts_with('#') || line.is_empty() {
                    continue;
                }
                if let Some((key, value)) = line.split_once('=') {
                    let val = value.trim();
                    match key.trim() {
                        "CLASH_CONTROLLER" => {
                            if val.starts_with("http://") || val.starts_with("https://") {
                                api_url = val.to_string();
                            } else {
                                api_url = format!("http://{}", val);
                            }
                        }
                        "CLASH_MIXED_PORT" => {
                            if let Ok(p) = val.parse() {
                                mixed_port = p;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Try to read API secret
        if let Ok(home) = env::var("HOME") {
            let secret_path = PathBuf::from(&home).join(".clashctl").join("runtime").join("api_secret");
            if let Ok(secret) = fs::read_to_string(&secret_path) {
                api_key = secret.trim().to_string();
            }
        }

        Self {
            api_url,
            api_key,
            mixed_port,
        }
    }

    fn config_path() -> PathBuf {
        if let Ok(home) = env::var("HOME") {
            let p = PathBuf::from(&home).join(".clashctl").join(".env");
            if p.exists() {
                return p;
            }
        }

        let local = PathBuf::from("resources/.env");
        if local.exists() {
            return local;
        }

        PathBuf::from(".env")
    }

    pub fn port(&self) -> u16 {
        self.mixed_port
    }
}
