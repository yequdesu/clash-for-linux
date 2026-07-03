use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

use crate::config;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KernelInfo {
    pub version: Option<String>,
    #[serde(default)]
    pub mode: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProxiesResponse {
    pub proxies: HashMap<String, ProxyInfo>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProxyInfo {
    #[serde(rename = "type")]
    pub proxy_type: String,
    pub now: Option<String>,
    pub all: Option<Vec<String>>,
    pub history: Option<Vec<DelayHistory>>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DelayHistory {
    pub delay: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConnectionsResponse {
    #[serde(default)]
    pub download_total: u64,
    #[serde(default)]
    pub upload_total: u64,
    #[serde(default)]
    pub connections: Vec<Connection>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Connection {
    pub id: String,
    pub metadata: Option<ConnectionMeta>,
    #[serde(default)]
    pub chains: Vec<String>,
    #[serde(default)]
    pub download: u64,
    #[serde(default)]
    pub upload: u64,
    #[serde(default)]
    pub start: String,
    #[serde(default)]
    pub rule: String,
    #[serde(rename = "rulePayload", default)]
    pub rule_payload: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConnectionMeta {
    pub network: Option<String>,
    pub host: Option<String>,
    #[serde(rename = "sourceIP", default)]
    pub source_ip: Option<String>,
    #[serde(rename = "destinationPort", default)]
    pub destination_port: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DelayResponse {
    pub delay: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LogEntry {
    #[serde(rename = "type")]
    pub level: String,
    pub payload: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct TrafficPoint {
    pub ts: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub upload_delta: u64,
    #[serde(default)]
    pub download_delta: u64,
    #[serde(default)]
    pub up_bps: u64,
    #[serde(default)]
    pub down_bps: u64,
    #[serde(default)]
    pub connections: usize,
    #[serde(default)]
    pub dimension: String,
    #[serde(default)]
    pub key: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct TrafficTopRow {
    #[serde(default)]
    pub dimension: String,
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub upload_delta: u64,
    #[serde(default)]
    pub download_delta: u64,
    #[serde(default)]
    pub total_delta: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct TrafficSnapshot {
    pub history: Vec<TrafficPoint>,
    pub top: Vec<TrafficTopRow>,
    pub status: TrafficStatus,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct TrafficStatus {
    #[serde(default)]
    pub store_dir: String,
    #[serde(default)]
    pub raw_samples: usize,
    #[serde(default)]
    pub rollup_10s: usize,
    #[serde(default)]
    pub rollup_1m: usize,
    #[serde(default)]
    pub last_sample: String,
    #[serde(default)]
    pub tracked_connections: usize,
    #[serde(default)]
    pub collector_running: bool,
    #[serde(default)]
    pub collector_stale: bool,
    #[serde(default)]
    pub collector_pid: i32,
    #[serde(default)]
    pub collector_started_at: String,
    #[serde(default)]
    pub collector_interval: String,
    #[serde(default)]
    pub collector_log: String,
    #[serde(default)]
    pub collector_status_read_error: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProfilesMeta {
    #[serde(default)]
    pub profiles: Vec<ProfileEntry>,
    #[serde(rename = "use", default)]
    pub active_id: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProfileEntry {
    pub id: i32,
    #[serde(default)]
    pub path: String,
    pub url: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub updated: String,
    #[serde(default)]
    pub interval: String,
    #[serde(default)]
    pub update_enabled: Option<bool>,
    #[serde(default)]
    pub update_interval: String,
    #[serde(default)]
    pub update_proxy: String,
    #[serde(default)]
    pub user_agent: String,
    #[serde(default)]
    pub convert_mode: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub last_error: String,
    #[serde(default)]
    pub last_updated: String,
    #[serde(default)]
    pub next_update: String,
}

#[derive(Debug, Default, Deserialize)]
struct RuntimeTunConfig {
    #[serde(default)]
    tun: TunConfig,
}

#[derive(Debug, Default, Deserialize)]
struct TunConfig {
    #[serde(default)]
    enable: bool,
}

pub fn read_profiles() -> ProfilesMeta {
    let empty = ProfilesMeta {
        profiles: vec![],
        active_id: 0,
    };
    for resources in config::resource_paths() {
        let path = resources.join("profiles.yaml");
        if let Ok(content) = fs::read_to_string(&path) {
            return serde_yaml::from_str(&content).unwrap_or(empty);
        }
    }
    empty
}

pub fn read_tun_status() -> bool {
    for resources in config::resource_paths() {
        let path = resources.join("runtime.yaml");
        if let Ok(content) = fs::read_to_string(&path) {
            return tun_enabled_from_yaml(&content).unwrap_or(false);
        }
    }
    false
}

fn tun_enabled_from_yaml(content: &str) -> Option<bool> {
    let runtime: RuntimeTunConfig = serde_yaml::from_str(content).ok()?;
    Some(runtime.tun.enable)
}

fn clashctl_bin() -> &'static str {
    if std::path::Path::new("/usr/local/bin/clashctl").exists() {
        "/usr/local/bin/clashctl"
    } else {
        "clashctl"
    }
}

pub async fn run_clashctl(args: &[String]) -> Result<String, String> {
    let mut command = Command::new(clashctl_bin());
    command.args(args);
    apply_install_env(&mut command);
    command_output(command).await
}

pub async fn run_clashctl_sudo(args: &[String], password: &str) -> Result<String, String> {
    let mut command = Command::new("sudo");
    command.arg("-S").arg("-p").arg("").arg(clashctl_bin());
    command.args(args);
    apply_install_env(&mut command);
    command.stdin(Stdio::piped());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(password.as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        stdin.write_all(b"\n").await.map_err(|e| e.to_string())?;
    }
    output_result(child.wait_with_output().await.map_err(|e| e.to_string())?)
}

fn apply_install_env(command: &mut Command) {
    if let Some(install) = config::active_install_env() {
        command.env("CLASH_BASE_DIR", &install.base_dir);
        if let Some(service_name) = install.service_name.as_deref() {
            command.env("SERVICE_NAME", service_name);
        }
        if let Some(kernel_name) = install.kernel_name.as_deref() {
            command.env("KERNEL_NAME", kernel_name);
        }
    }
}

async fn command_output(mut command: Command) -> Result<String, String> {
    output_result(command.output().await.map_err(|e| e.to_string())?)
}

fn output_result(output: std::process::Output) -> Result<String, String> {
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if output.status.success() {
        Ok(stdout)
    } else if stderr.is_empty() {
        Err(stdout)
    } else if stdout.is_empty() {
        Err(stderr)
    } else {
        Err(format!("{}\n{}", stderr, stdout))
    }
}

pub async fn run_clashctl_sub(action: &str, id: i32) -> Result<String, String> {
    let id_arg = id.to_string();
    let args = vec!["sub".to_string(), action.to_string(), id_arg];
    match run_clashctl(&args).await {
        Ok(stdout) => {
            if stdout.is_empty() {
                Ok(format!("subscription {} [{}] completed", action, id))
            } else {
                Ok(stdout.lines().last().unwrap_or("").to_string())
            }
        }
        Err(err) => Err(err),
    }
}

pub async fn read_traffic_snapshot(
    range: &str,
    step: &str,
    by: &str,
    key: Option<&str>,
) -> Result<TrafficSnapshot, String> {
    let history_args = vec![
        "traffic".to_string(),
        "history".to_string(),
        "--range".to_string(),
        range.to_string(),
        "--step".to_string(),
        step.to_string(),
        "--by".to_string(),
        if key.is_some() {
            by.to_string()
        } else {
            "total".to_string()
        },
    ];
    let mut history_args = history_args;
    if let Some(key) = key {
        history_args.push("--key".to_string());
        history_args.push(key.to_string());
    }
    history_args.push("--json".to_string());

    let top_args = vec![
        "traffic".to_string(),
        "top".to_string(),
        "--range".to_string(),
        range.to_string(),
        "--by".to_string(),
        by.to_string(),
        "--limit".to_string(),
        "12".to_string(),
        "--json".to_string(),
    ];
    let status_args = vec![
        "traffic".to_string(),
        "status".to_string(),
        "--json".to_string(),
    ];
    let history_raw = run_clashctl(&history_args).await?;
    let top_raw = run_clashctl(&top_args).await?;
    let status_raw = run_clashctl(&status_args).await?;
    let history = if history_raw.trim().is_empty() {
        Vec::new()
    } else {
        serde_json::from_str::<Vec<TrafficPoint>>(&history_raw)
            .map_err(|e| format!("parse traffic history: {}", e))?
    };
    let top = if top_raw.trim().is_empty() {
        Vec::new()
    } else {
        serde_json::from_str::<Vec<TrafficTopRow>>(&top_raw)
            .map_err(|e| format!("parse traffic top: {}", e))?
    };
    let status = if status_raw.trim().is_empty() {
        TrafficStatus::default()
    } else {
        serde_json::from_str::<TrafficStatus>(&status_raw)
            .map_err(|e| format!("parse traffic status: {}", e))?
    };
    Ok(TrafficSnapshot {
        history,
        top,
        status,
    })
}

pub async fn export_traffic_csv(range: &str, by: &str) -> Result<String, String> {
    let args = vec![
        "traffic".to_string(),
        "export".to_string(),
        "--range".to_string(),
        range.to_string(),
        "--by".to_string(),
        by.to_string(),
        "--format".to_string(),
        "csv".to_string(),
    ];
    let csv = run_clashctl(&args).await?;
    let path = traffic_export_path(range, by, "csv");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, csv).map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}

fn traffic_export_path(range: &str, by: &str, extension: &str) -> std::path::PathBuf {
    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let filename = format!(
        "traffic-{}-{}-{}.{}",
        timestamp,
        sanitize_filename_token(by),
        sanitize_filename_token(range),
        sanitize_filename_token(extension)
    );
    crate::settings::export_dir().join(filename)
}

fn sanitize_filename_token(input: &str) -> String {
    let mut token = String::with_capacity(input.len());
    for c in input.chars() {
        if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
            token.push(c);
        } else {
            token.push('-');
        }
    }
    let token = token.trim_matches('-').to_string();
    if token.is_empty() {
        "all".into()
    } else {
        token
    }
}

#[derive(Debug, Clone)]
pub struct ApiClient {
    pub base_url: String,
    pub api_key: String,
    client: reqwest::Client,
}

impl ApiClient {
    pub fn new(base_url: String, api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(8))
            .build()
            .expect("failed to create HTTP client");
        Self {
            base_url,
            api_key,
            client,
        }
    }

    fn get(&self, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}{}", self.base_url.trim_end_matches('/'), path);
        let mut req = self.client.get(&url);
        if !self.api_key.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", self.api_key));
        }
        req
    }

    fn put(&self, path: &str, body: String) -> reqwest::RequestBuilder {
        let url = format!("{}{}", self.base_url.trim_end_matches('/'), path);
        let mut req = self
            .client
            .put(&url)
            .header("Content-Type", "application/json")
            .body(body);
        if !self.api_key.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", self.api_key));
        }
        req
    }

    fn patch(&self, path: &str, body: String) -> reqwest::RequestBuilder {
        let url = format!("{}{}", self.base_url.trim_end_matches('/'), path);
        let mut req = self
            .client
            .patch(&url)
            .header("Content-Type", "application/json")
            .body(body);
        if !self.api_key.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", self.api_key));
        }
        req
    }

    fn delete(&self, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}{}", self.base_url.trim_end_matches('/'), path);
        let mut req = self.client.delete(&url);
        if !self.api_key.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", self.api_key));
        }
        req
    }

    async fn checked_text(resp: reqwest::Response) -> Result<String, String> {
        let status = resp.status();
        let text = resp.text().await.map_err(|e| e.to_string())?;
        if status.is_success() {
            Ok(text)
        } else {
            Err(Self::response_error(status, &text))
        }
    }

    async fn checked_success(resp: reqwest::Response) -> Result<(), String> {
        let status = resp.status();
        if status.is_success() {
            return Ok(());
        }
        let text = resp.text().await.map_err(|e| e.to_string())?;
        Err(Self::response_error(status, &text))
    }

    async fn checked_no_content(resp: reqwest::Response) -> Result<(), String> {
        let status = resp.status();
        if status == reqwest::StatusCode::NO_CONTENT {
            return Ok(());
        }
        let text = resp.text().await.map_err(|e| e.to_string())?;
        Err(Self::response_error(status, &text))
    }

    fn response_error(status: reqwest::StatusCode, text: &str) -> String {
        let snippet: String = text.chars().take(4096).collect();
        let snippet = snippet.trim();
        if snippet.is_empty() {
            format!("HTTP {}", status)
        } else {
            format!("HTTP {}: {}", status, snippet)
        }
    }

    pub async fn get_version(&self) -> Result<KernelInfo, String> {
        let resp = self
            .get("/version")
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let raw = Self::checked_text(resp).await?;
        serde_json::from_str::<KernelInfo>(&raw).map_err(|e| e.to_string())
    }

    pub async fn get_proxies(&self) -> Result<ProxiesResponse, String> {
        let resp = self
            .get("/proxies")
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let raw = Self::checked_text(resp).await?;
        serde_json::from_str(&raw).map_err(|e| format!("{}: {}", e, raw))
    }

    pub async fn get_connections(&self) -> Result<ConnectionsResponse, String> {
        let resp = self
            .get("/connections")
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let raw = Self::checked_text(resp).await?;
        serde_json::from_str::<ConnectionsResponse>(&raw).map_err(|e| e.to_string())
    }

    pub async fn test_delay(&self, name: &str) -> Result<u64, String> {
        let path = format!(
            "/proxies/{}/delay?url={}&timeout=5000",
            urlencoding::encode(name),
            urlencoding::encode("http://www.gstatic.com/generate_204"),
        );
        let resp = self.get(&path).send().await.map_err(|e| e.to_string())?;
        let raw = Self::checked_text(resp).await?;
        let delay: DelayResponse = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        Ok(delay.delay)
    }

    pub async fn switch_proxy(&self, group: &str, target: &str) -> Result<(), String> {
        let path = format!("/proxies/{}", urlencoding::encode(group));
        let body = serde_json::json!({ "name": target }).to_string();
        let resp = self
            .put(&path, body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        Self::checked_no_content(resp).await
    }

    pub async fn set_mode(&self, mode: &str) -> Result<(), String> {
        let body = serde_json::json!({ "mode": mode }).to_string();
        let resp = self
            .patch("/configs", body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        Self::checked_success(resp).await
    }

    pub async fn get_logs(&self, level: &str) -> Result<Vec<LogEntry>, String> {
        let level = urlencoding::encode(level);
        let path = format!("/logs?level={}", level);
        let mut resp = self.get(&path).send().await.map_err(|e| e.to_string())?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.map_err(|e| e.to_string())?;
            return Err(Self::response_error(status, &text));
        }

        let mut raw = String::new();
        let mut logs = Vec::new();
        for _ in 0..64 {
            match timeout(Duration::from_millis(250), resp.chunk()).await {
                Ok(Ok(Some(chunk))) => {
                    raw.push_str(&String::from_utf8_lossy(&chunk));
                    if raw.len() >= 65_536 {
                        break;
                    }
                }
                Ok(Ok(None)) => break,
                Ok(Err(e)) => return Err(e.to_string()),
                Err(_) => break,
            }
        }
        collect_logs_from_text(&raw, &mut logs);
        Ok(logs)
    }

    pub async fn close_connection(&self, id: &str) -> Result<(), String> {
        let path = format!("/connections/{}", urlencoding::encode(id));
        let resp = self.delete(&path).send().await.map_err(|e| e.to_string())?;
        Self::checked_no_content(resp).await
    }

    pub async fn close_all_connections(&self) -> Result<(), String> {
        let resp = self
            .delete("/connections")
            .send()
            .await
            .map_err(|e| e.to_string())?;
        Self::checked_no_content(resp).await
    }
}

fn collect_logs_from_text(raw: &str, logs: &mut Vec<LogEntry>) {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return;
    }
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
        append_log_value(value, logs);
        return;
    }
    for line in raw.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
            append_log_value(value, logs);
        }
    }
}

fn append_log_value(value: serde_json::Value, logs: &mut Vec<LogEntry>) {
    if let Some(batch) = value.get("logs") {
        if let Ok(mut entries) = serde_json::from_value::<Vec<LogEntry>>(batch.clone()) {
            logs.append(&mut entries);
        }
        return;
    }
    if let Ok(entry) = serde_json::from_value::<LogEntry>(value) {
        logs.push(entry);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        clashctl_bin, collect_logs_from_text, sanitize_filename_token, tun_enabled_from_yaml,
        ApiClient,
    };
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn clashctl_bin_has_fallback() {
        let bin = clashctl_bin();
        assert!(bin == "/usr/local/bin/clashctl" || bin == "clashctl");
    }

    #[test]
    fn tun_status_is_read_from_structured_yaml() {
        let enabled = tun_enabled_from_yaml(
            r#"
mixed-port: 7890
tun:
  enable: true
  stack: system
"#,
        );
        assert_eq!(enabled, Some(true));

        let disabled = tun_enabled_from_yaml(
            r#"
tun:
  enable: false
"#,
        );
        assert_eq!(disabled, Some(false));
    }

    #[test]
    fn tun_status_missing_block_defaults_false() {
        assert_eq!(tun_enabled_from_yaml("mixed-port: 7890\n"), Some(false));
    }

    #[test]
    fn export_filename_tokens_are_sanitized() {
        assert_eq!(sanitize_filename_token("route"), "route");
        assert_eq!(sanitize_filename_token("24h"), "24h");
        assert_eq!(sanitize_filename_token("../../route"), "route");
        assert_eq!(sanitize_filename_token(""), "all");
    }

    #[test]
    fn collect_logs_supports_batch_and_streaming_json() {
        let mut logs = Vec::new();
        collect_logs_from_text(r#"{"logs":[{"type":"info","payload":"batch"}]}"#, &mut logs);
        collect_logs_from_text(
            "{\"type\":\"warning\",\"payload\":\"stream one\"}\n{\"type\":\"error\",\"payload\":\"stream two\"}\n",
            &mut logs,
        );

        assert_eq!(logs.len(), 3);
        assert_eq!(logs[0].payload, "batch");
        assert_eq!(logs[1].level, "warning");
        assert_eq!(logs[2].payload, "stream two");
    }

    fn serve_once(status: &str, body: &str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
        let addr = listener.local_addr().expect("listener address");
        let status = status.to_string();
        let body = body.to_string();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let mut buf = [0_u8; 4096];
            let _ = stream.read(&mut buf);
            let response = format!(
                "HTTP/1.1 {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                status,
                body.len(),
                body
            );
            stream
                .write_all(response.as_bytes())
                .expect("write response");
        });
        format!("http://{}", addr)
    }

    #[tokio::test]
    async fn switch_proxy_error_includes_response_body() {
        let client = ApiClient::new(
            serve_once("400 Bad Request", "bad proxy name"),
            String::new(),
        );
        let err = client
            .switch_proxy("Auto", "missing")
            .await
            .expect_err("switch should fail");

        assert!(err.contains("HTTP 400 Bad Request"), "err = {}", err);
        assert!(err.contains("bad proxy name"), "err = {}", err);
    }

    #[tokio::test]
    async fn close_connection_error_includes_response_body() {
        let client = ApiClient::new(
            serve_once("404 Not Found", "connection missing"),
            String::new(),
        );
        let err = client
            .close_connection("conn/1")
            .await
            .expect_err("close should fail");

        assert!(err.contains("HTTP 404 Not Found"), "err = {}", err);
        assert!(err.contains("connection missing"), "err = {}", err);
    }

    #[tokio::test]
    async fn set_mode_error_body_is_truncated() {
        let mut body = "x".repeat(4096);
        body.push_str("TAIL");
        let client = ApiClient::new(
            serve_once("500 Internal Server Error", &body),
            String::new(),
        );
        let err = client
            .set_mode("global")
            .await
            .expect_err("mode switch should fail");

        assert!(
            err.contains("HTTP 500 Internal Server Error"),
            "err = {}",
            err
        );
        assert!(!err.contains("TAIL"), "err was not truncated");
    }
}
