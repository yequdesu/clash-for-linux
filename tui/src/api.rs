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
    pub up: u64,
    pub down: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Connection {
    pub id: String,
    pub host: Option<String>,
    pub network: Option<String>,
    #[serde(rename = "type")]
    pub conn_type: Option<String>,
    pub chain: Option<Vec<String>>,
    #[serde(rename = "downloadSpeed")]
    pub download_speed: Option<u64>,
    #[serde(rename = "uploadSpeed")]
    pub upload_speed: Option<u64>,
    pub start: Option<String>,
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
        resp.json::<T>().await.map_err(|e| e.to_string())
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
        self.get("/traffic").await
    }

    pub async fn get_connections(&self) -> Result<Vec<Connection>, String> {
        let resp: ConnectionsResponse = self.get("/connections").await?;
        Ok(resp.connections)
    }

    pub async fn get_memory(&self) -> Result<MemoryInfo, String> {
        self.get("/memory").await
    }

    pub async fn get_config(&self) -> Result<RuntimeConfig, String> {
        self.get("/configs").await
    }

    pub async fn switch_proxy(&self, group: &str, proxy: &str) -> Result<(), String> {
        let body = format!(r#"{{"name":"{}"}}"#, proxy);
        let _v: serde_json::Value = self.put(&format!("/proxies/{}", group), &body).await?;
        Ok(())
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
        let _v: serde_json::Value = self.put("/configs", &body).await?;
        Ok(())
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
        groups
    }
}
