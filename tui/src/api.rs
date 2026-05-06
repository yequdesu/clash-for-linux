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
        let v: serde_json::Value = self.get("/traffic").await?;
        Ok(TrafficInfo {
            up: v.get("up").and_then(|v| v.as_u64()).unwrap_or(0),
            down: v.get("down").and_then(|v| v.as_u64()).unwrap_or(0),
        })
    }

    pub async fn get_connections(&self) -> Result<Vec<Connection>, String> {
        let resp: ConnectionsResponse = self.get("/connections").await?;
        Ok(resp.connections)
    }

    pub async fn get_memory(&self) -> Result<MemoryInfo, String> {
        let v: serde_json::Value = self.get("/memory").await?;
        Ok(MemoryInfo {
            inuse: v.get("inuse").and_then(|v| v.as_u64()),
            oslimit: v.get("oslimit").and_then(|v| v.as_u64()),
        })
    }

    pub async fn get_config(&self) -> Result<RuntimeConfig, String> {
        self.get("/configs").await
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
