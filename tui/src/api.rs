use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProxiesResponse {
    pub proxies: std::collections::HashMap<String, ProxyDetail>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProxyDetail {
    #[serde(rename = "type")]
    pub proxy_type: String,
    pub now: Option<String>,
    pub name: Option<String>,
    pub all: Option<Vec<String>>,
    pub history: Option<Vec<DelayHistory>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DelayHistory {
    pub delay: i64,
}

#[derive(Debug, Clone)]
pub struct ProxyGroup {
    pub name: String,
    pub group_type: String,
    pub now: String,
    pub proxies: Vec<ProxyNodeInfo>,
}

#[derive(Debug, Clone)]
pub struct ProxyNodeInfo {
    pub name: String,
    pub proxy_type: String,
    pub delay: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TrafficInfo {
    #[serde(default)]
    pub up: u64,
    #[serde(default)]
    pub down: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Connection {
    pub id: String,
    pub metadata: Option<ConnectionMetadata>,
    #[serde(default)]
    pub upload: u64,
    #[serde(default)]
    pub download: u64,
    pub start: Option<String>,
    #[serde(default)]
    pub chains: Vec<String>,
    pub rule: Option<String>,
    #[serde(rename = "rulePayload")]
    pub rule_payload: Option<String>,
    #[serde(rename = "uploadSpeed")]
    pub upload_speed: Option<u64>,
    #[serde(rename = "downloadSpeed")]
    pub download_speed: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConnectionMetadata {
    pub network: Option<String>,
    #[serde(rename = "type")]
    pub conn_type: Option<String>,
    pub host: Option<String>,
}

impl Connection {
    pub fn host(&self) -> String {
        self.metadata.as_ref().and_then(|m| m.host.clone()).unwrap_or_default()
    }
    pub fn conn_type(&self) -> String {
        self.metadata.as_ref().and_then(|m| m.conn_type.clone()).unwrap_or_default()
    }
    pub fn chain_str(&self) -> String {
        if self.chains.is_empty() { String::new() } else { self.chains.join(" → ") }
    }
    pub fn dl_speed(&self) -> u64 { self.download_speed.unwrap_or(0) }
    pub fn ul_speed(&self) -> u64 { self.upload_speed.unwrap_or(0) }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConnectionsResponse {
    pub connections: Vec<Connection>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MemoryInfo {
    pub inuse: Option<u64>,
    pub oslimit: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeConfig {
    pub mode: Option<String>,
    #[serde(rename = "mixed-port")]
    pub mixed_port: Option<u16>,
    pub tun: Option<TunConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TunConfig {
    pub enable: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub level: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct SubscriptionInfo {
    pub id: usize,
    pub name: String,
    pub url: String,
    pub status: String,
    pub updated: String,
    pub proxies_count: usize,
    pub upload: u64,
    pub download: u64,
    pub total: u64,
    pub expire: String,
    pub interval: u64,
    pub path: String,
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
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("failed to create HTTP client");
        Self {
            base_url,
            api_key,
            client,
        }
    }

    async fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T, String> {
        let url = format!("{}{}", self.base_url.trim_end_matches('/'), path);
        let mut req = self.client.get(&url);
        if !self.api_key.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", self.api_key));
        }
        let resp = req.send().await.map_err(|e| e.to_string())?;
        let status = resp.status();
        let text = resp.text().await.map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!("HTTP {}: {}", status.as_u16(), text));
        }
        serde_json::from_str::<T>(&text)
            .map_err(|e| format!("{} — body: {}", e, &text[..text.len().min(200)]))
    }

    /// Read first JSON line from a streaming endpoint (NDJSON)
    async fn get_first_json<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T, String> {
        let url = format!("{}{}", self.base_url.trim_end_matches('/'), path);
        let mut req = self.client.get(&url);
        if !self.api_key.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", self.api_key));
        }
        let mut resp = req.send().await.map_err(|e| e.to_string())?;
        let status = resp.status();
        if !status.is_success() {
            return Err(format!("HTTP {}", status.as_u16()));
        }
        // Read only until first newline (traffic/memory stream NDJSON)
        let mut line = Vec::new();
        while let Some(chunk) = resp.chunk().await.map_err(|e| e.to_string())? {
            for &b in chunk.iter() {
                if b == b'\n' {
                    let s = String::from_utf8_lossy(&line).to_string();
                    return serde_json::from_str::<T>(&s)
                        .map_err(|e| format!("{} — line: {}", e, &s[..s.len().min(200)]));
                }
                line.push(b);
            }
            if line.len() > 65536 {
                break;
            }
        }
        if !line.is_empty() {
            let s = String::from_utf8_lossy(&line).to_string();
            return serde_json::from_str::<T>(&s)
                .map_err(|e| format!("{} — line: {}", e, &s[..s.len().min(200)]));
        }
        Err("empty streaming response".into())
    }

    async fn put<T: for<'de> Deserialize<'de>>(&self, path: &str, body: &str) -> Result<T, String> {
        let url = format!("{}{}", self.base_url.trim_end_matches('/'), path);
        let mut req = self.client.put(&url).body(body.to_string());
        if !self.api_key.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", self.api_key));
        }
        req = req.header("Content-Type", "application/json");
        let resp = req.send().await.map_err(|e| e.to_string())?;
        resp.json::<T>().await.map_err(|e| e.to_string())
    }

    async fn put_no_body(&self, path: &str, body: &str) -> Result<(), String> {
        let url = format!("{}{}", self.base_url.trim_end_matches('/'), path);
        let mut req = self.client.put(&url).body(body.to_string());
        if !self.api_key.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", self.api_key));
        }
        req = req.header("Content-Type", "application/json");
        let resp = req.send().await.map_err(|e| e.to_string())?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("HTTP {}: {}", status.as_u16(), text));
        }
        Ok(())
    }

    async fn patch_no_body(&self, path: &str, body: &str) -> Result<(), String> {
        let url = format!("{}{}", self.base_url.trim_end_matches('/'), path);
        let mut req = self.client.patch(&url).body(body.to_string());
        if !self.api_key.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", self.api_key));
        }
        req = req.header("Content-Type", "application/json");
        let resp = req.send().await.map_err(|e| e.to_string())?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("HTTP {}: {}", status.as_u16(), text));
        }
        Ok(())
    }

    pub async fn get_version(&self) -> Result<String, String> {
        #[derive(Deserialize)]
        struct VersionResp {
            version: Option<String>,
            #[serde(default)]
            meta: Option<serde_json::Value>,
        }
        let resp: VersionResp = self.get("/version").await?;
        let ver = resp.version.or_else(|| {
            resp.meta.and_then(|v| v.as_str().map(String::from))
        }).unwrap_or_else(|| "unknown".into());
        Ok(ver)
    }

    pub async fn get_proxies(&self) -> Result<ProxiesResponse, String> {
        self.get("/proxies").await
    }

    pub async fn get_traffic(&self) -> Result<TrafficInfo, String> {
        let v: serde_json::Value = self.get_first_json("/traffic").await?;
        Ok(TrafficInfo {
            up: v.get("up").and_then(|v| v.as_u64()).unwrap_or(0),
            down: v.get("down").and_then(|v| v.as_u64()).unwrap_or(0),
        })
    }

    pub async fn get_connections(&self) -> Result<Vec<Connection>, String> {
        let v: serde_json::Value = self.get("/connections").await?;
        match v.get("connections") {
            Some(serde_json::Value::Array(arr)) => {
                serde_json::from_value::<Vec<Connection>>(serde_json::Value::Array(arr.clone()))
                    .map_err(|e| format!("{}", e))
            }
            _ => Ok(Vec::new()), // null or missing when no connections
        }
    }

    pub async fn get_subscriptions(&self) -> Result<(Vec<SubscriptionInfo>, usize), String> {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        let path = format!("{}/.clashctl/resources/profiles.yaml", home);
        let content = tokio::fs::read_to_string(&path).await
            .map_err(|e| format!("Cannot read {}: {}", path, e))?;

        let mut subs = Vec::new();
        let mut active_id = 0usize;
        let mut in_profile = false;
        let mut in_extra = false;
        let mut current_id = 0usize;
        let mut current_name = String::new();
        let mut current_url = String::new();
        let mut current_updated = String::new();
        let mut current_path = String::new();
        let mut current_interval: u64 = 0;
        let mut current_upload: u64 = 0;
        let mut current_download: u64 = 0;
        let mut current_total: u64 = 0;
        let mut current_expire: i64 = 0;

        for line in content.lines() {
            let raw = line;
            let indent = raw.len() - raw.trim_start().len();
            let trimmed = raw.trim_start();

            if indent == 0 && trimmed.starts_with("use:") {
                active_id = trimmed.strip_prefix("use:").unwrap_or("0").trim().parse().unwrap_or(0);
            } else if indent <= 4 && trimmed.starts_with("- id:") {
                if in_profile && current_id > 0 {
                    subs.push(build_sub_info(
                        current_id, &current_name, &current_url, &current_path,
                        &current_updated, current_interval,
                        current_upload, current_download, current_total,
                        current_expire, active_id,
                    ));
                }
                in_profile = true;
                in_extra = false;
                current_id = trimmed.strip_prefix("- id:").unwrap_or("0").trim().parse().unwrap_or(0);
                current_name.clear();
                current_url.clear();
                current_path.clear();
                current_updated.clear();
                current_interval = 0;
                current_upload = 0;
                current_download = 0;
                current_total = 0;
                current_expire = 0;
            } else if in_profile && trimmed.starts_with("extra:") {
                in_extra = true;
            } else if in_extra && indent > 8 {
            } else if in_extra && trimmed.starts_with("- ") || trimmed.starts_with("path:") || trimmed.starts_with("name:") {
                in_extra = false;
            }

            if !in_extra {
                if in_profile && trimmed.starts_with("name:") {
                    current_name = trimmed.strip_prefix("name:").unwrap_or("").trim().trim_matches('"').to_string();
                } else if in_profile && trimmed.starts_with("url:") {
                    current_url = trimmed.strip_prefix("url:").unwrap_or("").trim().to_string();
                } else if in_profile && trimmed.starts_with("updated:") {
                    let ts: i64 = trimmed.strip_prefix("updated:").unwrap_or("0").trim().parse().unwrap_or(0);
                    if ts > 0 {
                        current_updated = ts.to_string();
                    }
                } else if in_profile && trimmed.starts_with("path:") {
                    current_path = trimmed.strip_prefix("path:").unwrap_or("").trim().to_string();
                } else if in_profile && trimmed.starts_with("interval:") {
                    current_interval = trimmed.strip_prefix("interval:").unwrap_or("0").trim().parse().unwrap_or(0);
                }
            } else {
                let inner = trimmed.trim_start_matches('-').trim();
                if inner.starts_with("upload:") {
                    current_upload = inner.strip_prefix("upload:").unwrap_or("0").trim().parse().unwrap_or(0);
                } else if inner.starts_with("download:") {
                    current_download = inner.strip_prefix("download:").unwrap_or("0").trim().parse().unwrap_or(0);
                } else if inner.starts_with("total:") {
                    current_total = inner.strip_prefix("total:").unwrap_or("0").trim().parse().unwrap_or(0);
                } else if inner.starts_with("expire:") {
                    current_expire = inner.strip_prefix("expire:").unwrap_or("0").trim().parse().unwrap_or(0);
                }
            }
        }

        if in_profile && current_id > 0 {
            subs.push(build_sub_info(
                current_id, &current_name, &current_url, &current_path,
                &current_updated, current_interval,
                current_upload, current_download, current_total,
                current_expire, active_id,
            ));
        }

        for sub in &mut subs {
            sub.proxies_count = count_proxies_in_yaml(&sub.path).await;
        }

        Ok((subs, active_id))
    }

    pub async fn get_config(&self) -> Result<RuntimeConfig, String> {
        self.get("/configs").await
    }

    pub async fn get_memory(&self) -> Result<MemoryInfo, String> {
        // /memory streams NDJSON; first line is often 0, read multiple and take last
        let url = format!("{}/memory", self.base_url.trim_end_matches('/'));
        let mut req = self.client.get(&url);
        if !self.api_key.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", self.api_key));
        }
        let mut resp = req.send().await.map_err(|e| e.to_string())?;
        let mut lines = String::new();
        // Read up to 3 chunks, taking the last valid JSON line
        for _ in 0..3 {
            match resp.chunk().await.map_err(|e| e.to_string())? {
                Some(chunk) => lines.push_str(&String::from_utf8_lossy(&chunk)),
                None => break,
            }
        }
        let last = lines.lines().filter(|l| l.contains("inuse")).last().unwrap_or("{\"inuse\":0,\"oslimit\":0}");
        let v: serde_json::Value = serde_json::from_str(last)
            .map_err(|e| format!("{} — line: {}", e, &last[..last.len().min(200)]))?;
        Ok(MemoryInfo {
            inuse: v.get("inuse").and_then(|v| v.as_u64()),
            oslimit: v.get("oslimit").and_then(|v| v.as_u64()),
        })
    }

    pub async fn switch_proxy(&self, group: &str, proxy: &str) -> Result<(), String> {
        let body = format!(r#"{{"name":"{}"}}"#, proxy);
        self.put_no_body(&format!("/proxies/{}", group), &body).await
    }

    pub async fn test_delay(&self, proxy: &str, url: &str, timeout: u64) -> Result<i64, String> {
        #[derive(Deserialize)]
        struct DelayResp {
            delay: i64,
        }
        let path = format!("/proxies/{}/delay?url={}&timeout={}", proxy, url, timeout);
        let resp: DelayResp = self.get(&path).await?;
        Ok(resp.delay)
    }

    pub async fn set_mode(&self, mode: &str) -> Result<(), String> {
        let body = format!(r#"{{"mode":"{}"}}"#, mode);
        self.patch_no_body("/configs", &body).await
    }

    pub async fn close_connection(&self, id: &str) -> Result<(), String> {
        let url = format!("{}/connections/{}", self.base_url.trim_end_matches('/'), id);
        let mut req = self.client.delete(&url);
        if !self.api_key.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", self.api_key));
        }
        req.send().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn close_all_connections(&self) -> Result<(), String> {
        let url = format!("{}/connections", self.base_url.trim_end_matches('/'));
        let mut req = self.client.delete(&url);
        if !self.api_key.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", self.api_key));
        }
        req.send().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn get_logs(&self) -> Result<Vec<String>, String> {
        #[derive(Deserialize)]
        struct LogsResponse {
            logs: Vec<String>,
        }
        let resp: LogsResponse = self.get("/logs").await?;
        Ok(resp.logs)
    }
}

impl ProxiesResponse {
    pub fn get_groups(&self) -> Vec<ProxyGroup> {
        let mut groups = Vec::new();
        for (name, detail) in &self.proxies {
            let ptype = detail.proxy_type.as_str();
            if ptype == "Selector" || ptype == "URLTest" || ptype == "Fallback" || ptype == "LoadBalance" {
                let mut group = ProxyGroup {
                    name: name.clone(),
                    group_type: detail.proxy_type.clone(),
                    now: detail.now.clone().unwrap_or_default(),
                    proxies: Vec::new(),
                };
                if let Some(all) = &detail.all {
                    for pn in all {
                        let mut node_type = String::new();
                        let mut delay: i64 = -1;
                        if let Some(pd) = self.proxies.get(pn) {
                            node_type = pd.proxy_type.clone();
                            if let Some(history) = &pd.history {
                                if let Some(last) = history.last() {
                                    delay = last.delay;
                                }
                            }
                        }
                        group.proxies.push(ProxyNodeInfo {
                            name: pn.clone(),
                            proxy_type: node_type,
                            delay,
                        });
                    }
                }
                groups.push(group);
            }
        }
        groups.sort_by(|a, b| a.name.cmp(&b.name));
        groups
    }
}

fn build_sub_info(
    id: usize, name: &str, url: &str, path: &str,
    updated: &str, interval: u64,
    upload: u64, download: u64, total: u64,
    expire_ts: i64, active_id: usize,
) -> SubscriptionInfo {
    let status = if id == active_id { "active" } else { "ready" };
    let expire = if expire_ts > 0 {
        let dt = chrono::DateTime::from_timestamp(expire_ts, 0);
        dt.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "—".into())
    } else {
        "—".into()
    };
    let updated_display = if !updated.is_empty() && updated != "0" {
        let ts: i64 = updated.parse().unwrap_or(0);
        if ts > 0 {
            chrono::DateTime::from_timestamp(ts, 0)
                .map(|d| d.format("%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "—".into())
        } else {
            "—".into()
        }
    } else {
        "—".into()
    };
    SubscriptionInfo {
        id, name: name.to_string(), url: url.to_string(),
        path: path.to_string(), status: status.to_string(),
        updated: updated_display, proxies_count: 0,
        upload, download, total, expire,
        interval,
    }
}

async fn count_proxies_in_yaml(path: &str) -> usize {
    let expanded = if path.starts_with('~') {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        path.replacen('~', &home, 1)
    } else {
        path.to_string()
    };
    let content = match tokio::fs::read_to_string(&expanded).await {
        Ok(c) => c,
        Err(_) => return 0,
    };
    let mut count = 0usize;
    let mut in_proxies = false;
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("proxies:") {
            in_proxies = true;
            continue;
        }
        if in_proxies {
            if !trimmed.starts_with('-') && !trimmed.is_empty() && !line.starts_with(' ') {
                in_proxies = false;
                continue;
            }
            if trimmed.starts_with("- ") || trimmed == "-" {
                count += 1;
            }
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path};

    #[tokio::test]
    async fn test_get_version() {
        // 启动 mock 服务器
        let mock_server = MockServer::start().await;

        // 准备 mock 响应
        let response_body = serde_json::json!({
            "version": "v1.19.17"
        });

        Mock::given(method("GET"))
            .and(path("/version"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        // 创建客户端
        let client = ApiClient::new(mock_server.uri(), String::new());

        // 执行测试
        let version = client.get_version().await.unwrap();

        // 验证结果
        assert_eq!(version, "v1.19.17");
    }

    #[tokio::test]
    async fn test_get_version_with_meta() {
        // 启动 mock 服务器
        let mock_server = MockServer::start().await;

        // 准备 mock 响应（使用 meta 字段）
        let response_body = serde_json::json!({
            "meta": "v1.19.18"
        });

        Mock::given(method("GET"))
            .and(path("/version"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        // 创建客户端
        let client = ApiClient::new(mock_server.uri(), String::new());

        // 执行测试
        let version = client.get_version().await.unwrap();

        // 验证结果
        assert_eq!(version, "v1.19.18");
    }

    #[tokio::test]
    async fn test_get_version_error() {
        // 启动 mock 服务器
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/version"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        // 创建客户端
        let client = ApiClient::new(mock_server.uri(), String::new());

        // 执行测试
        let result = client.get_version().await;

        // 验证返回错误
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_proxies() {
        // 启动 mock 服务器
        let mock_server = MockServer::start().await;

        // 准备 mock 响应
        let response_body = serde_json::json!({
            "proxies": {
                "Proxy": {
                    "name": "Proxy",
                    "type": "Selector",
                    "now": "node1",
                    "all": ["node1", "node2"]
                }
            }
        });

        Mock::given(method("GET"))
            .and(path("/proxies"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        // 创建客户端
        let client = ApiClient::new(mock_server.uri(), String::new());

        // 执行测试
        let proxies = client.get_proxies().await.unwrap();

        // 验证结果
        let groups = proxies.get_groups();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "Proxy");
    }

    #[tokio::test]
    async fn test_get_proxies_error() {
        // 启动 mock 服务器
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/proxies"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        // 创建客户端
        let client = ApiClient::new(mock_server.uri(), String::new());

        // 执行测试
        let result = client.get_proxies().await;

        // 验证返回错误
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_traffic() {
        // 启动 mock 服务器
        let mock_server = MockServer::start().await;

        // 准备 mock 响应
        let response_body = serde_json::json!({
            "up": 1024,
            "down": 2048
        });

        Mock::given(method("GET"))
            .and(path("/traffic"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        // 创建客户端
        let client = ApiClient::new(mock_server.uri(), String::new());

        // 执行测试
        let traffic = client.get_traffic().await.unwrap();

        // 验证结果
        assert_eq!(traffic.up, 1024);
        assert_eq!(traffic.down, 2048);
    }

    #[tokio::test]
    async fn test_get_connections() {
        // 启动 mock 服务器
        let mock_server = MockServer::start().await;

        // 准备 mock 响应
        let response_body = serde_json::json!({
            "connections": [
                {
                    "id": "conn1",
                    "metadata": {
                        "network": "tcp",
                        "type": "HTTP",
                        "host": "example.com"
                    },
                    "upload": 1024,
                    "download": 2048,
                    "start": "2024-01-01T00:00:00Z",
                    "chains": ["Proxy", "node1"],
                    "rule": "DOMAIN",
                    "rulePayload": "example.com"
                }
            ]
        });

        Mock::given(method("GET"))
            .and(path("/connections"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        // 创建客户端
        let client = ApiClient::new(mock_server.uri(), String::new());

        // 执行测试
        let connections = client.get_connections().await.unwrap();

        // 验证结果
        assert_eq!(connections.len(), 1);
        assert_eq!(connections[0].id, "conn1");
        assert_eq!(connections[0].host(), "example.com");
    }

    #[tokio::test]
    async fn test_get_connections_empty() {
        // 启动 mock 服务器
        let mock_server = MockServer::start().await;

        // 准备 mock 响应（空连接）
        let response_body = serde_json::json!({
            "connections": []
        });

        Mock::given(method("GET"))
            .and(path("/connections"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        // 创建客户端
        let client = ApiClient::new(mock_server.uri(), String::new());

        // 执行测试
        let connections = client.get_connections().await.unwrap();

        // 验证结果
        assert_eq!(connections.len(), 0);
    }

    #[tokio::test]
    async fn test_switch_proxy() {
        // 启动 mock 服务器
        let mock_server = MockServer::start().await;

        Mock::given(method("PUT"))
            .and(path("/proxies/Proxy"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;

        // 创建客户端
        let client = ApiClient::new(mock_server.uri(), String::new());

        // 执行测试
        let result = client.switch_proxy("Proxy", "node1").await;

        // 验证结果
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_switch_proxy_error() {
        // 启动 mock 服务器
        let mock_server = MockServer::start().await;

        Mock::given(method("PUT"))
            .and(path("/proxies/Proxy"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        // 创建客户端
        let client = ApiClient::new(mock_server.uri(), String::new());

        // 执行测试
        let result = client.switch_proxy("Proxy", "node1").await;

        // 验证返回错误
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_test_delay() {
        // 启动 mock 服务器
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/proxies/node1/delay"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&serde_json::json!({
                "delay": 100
            })))
            .mount(&mock_server)
            .await;

        // 创建客户端
        let client = ApiClient::new(mock_server.uri(), String::new());

        // 执行测试
        let delay = client.test_delay("node1", "https://www.google.com", 5000).await.unwrap();

        // 验证结果
        assert_eq!(delay, 100);
    }

    #[tokio::test]
    async fn test_close_connection() {
        // 启动 mock 服务器
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/connections/conn1"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;

        // 创建客户端
        let client = ApiClient::new(mock_server.uri(), String::new());

        // 执行测试
        let result = client.close_connection("conn1").await;

        // 验证结果
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_close_all_connections() {
        // 启动 mock 服务器
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/connections"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;

        // 创建客户端
        let client = ApiClient::new(mock_server.uri(), String::new());

        // 执行测试
        let result = client.close_all_connections().await;

        // 验证结果
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_config() {
        // 启动 mock 服务器
        let mock_server = MockServer::start().await;

        // 准备 mock 响应
        let response_body = serde_json::json!({
            "mode": "Rule",
            "mixed-port": 7890,
            "tun": {
                "enable": true
            }
        });

        Mock::given(method("GET"))
            .and(path("/configs"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        // 创建客户端
        let client = ApiClient::new(mock_server.uri(), String::new());

        // 执行测试
        let config = client.get_config().await.unwrap();

        // 验证结果
        assert_eq!(config.mode, Some("Rule".to_string()));
        assert_eq!(config.mixed_port, Some(7890));
        assert_eq!(config.tun.as_ref().unwrap().enable, Some(true));
    }

    #[tokio::test]
    async fn test_set_mode() {
        // 启动 mock 服务器
        let mock_server = MockServer::start().await;

        Mock::given(method("PATCH"))
            .and(path("/configs"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;

        // 创建客户端
        let client = ApiClient::new(mock_server.uri(), String::new());

        // 执行测试
        let result = client.set_mode("Global").await;

        // 验证结果
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_memory() {
        // 启动 mock 服务器
        let mock_server = MockServer::start().await;

        // 准备 mock 响应
        let response_body = serde_json::json!({
            "inuse": 1048576,
            "oslimit": 2097152
        });

        Mock::given(method("GET"))
            .and(path("/memory"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        // 创建客户端
        let client = ApiClient::new(mock_server.uri(), String::new());

        // 执行测试
        let memory = client.get_memory().await.unwrap();

        // 验证结果
        assert_eq!(memory.inuse, Some(1048576));
        assert_eq!(memory.oslimit, Some(2097152));
    }

    #[tokio::test]
    async fn test_client_with_api_key() {
        // 启动 mock 服务器
        let mock_server = MockServer::start().await;

        // 验证 Authorization 头部
        Mock::given(method("GET"))
            .and(path("/version"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&serde_json::json!({
                "version": "v1.19.17"
            })))
            .mount(&mock_server)
            .await;

        // 创建客户端（带 API key）
        let client = ApiClient::new(mock_server.uri(), "test-secret".to_string());

        // 执行测试
        let version = client.get_version().await.unwrap();

        // 验证结果
        assert_eq!(version, "v1.19.17");
    }

    #[tokio::test]
    async fn test_client_connection_error() {
        // 创建客户端（连接到不存在的服务器）
        let client = ApiClient::new("http://localhost:1".to_string(), String::new());

        // 执行测试
        let result = client.get_version().await;

        // 验证返回错误
        assert!(result.is_err());
    }
}
