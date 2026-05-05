use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
            .danger_accept_invalid_certs(true)
            .build()
            .expect("failed to create HTTP client");
        Self { base_url, api_key, client }
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
        let mut req = self.client.put(&url).header("Content-Type", "application/json").body(body);
        if !self.api_key.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", self.api_key));
        }
        req
    }

    pub async fn get_version(&self) -> Result<KernelInfo, String> {
        let resp = self.get("/").send().await.map_err(|e| e.to_string())?;
        resp.json::<KernelInfo>().await.map_err(|e| e.to_string())
    }

    pub async fn get_proxies(&self) -> Result<ProxiesResponse, String> {
        let resp = self.get("/proxies").send().await.map_err(|e| e.to_string())?;
        let raw = resp.text().await.map_err(|e| e.to_string())?;
        serde_json::from_str(&raw).map_err(|e| format!("{}: {}", e, raw))
    }

    pub async fn get_connections(&self) -> Result<ConnectionsResponse, String> {
        let resp = self.get("/connections").send().await.map_err(|e| e.to_string())?;
        resp.json::<ConnectionsResponse>().await.map_err(|e| e.to_string())
    }

    pub async fn test_delay(&self, name: &str) -> Result<u64, String> {
        let path = format!("/proxies/{}/delay?url=http://www.gstatic.com/generate_204&timeout=5000", name);
        let resp = self.get(&path).send().await.map_err(|e| e.to_string())?;
        let delay: DelayResponse = resp.json().await.map_err(|e| e.to_string())?;
        Ok(delay.delay)
    }

    pub async fn switch_proxy(&self, group: &str, target: &str) -> Result<(), String> {
        let path = format!("/proxies/{}", group);
        let body = format!(r#"{{"name":"{}"}}"#, target);
        let resp = self.put(&path, body).send().await.map_err(|e| e.to_string())?;
        if resp.status() == 204 {
            Ok(())
        } else {
            Err(format!("HTTP {}", resp.status()))
        }
    }

    pub async fn get_logs(&self) -> Result<Vec<LogEntry>, String> {
        let resp = self.get("/logs?level=info").send().await.map_err(|e| e.to_string())?;
        let raw = resp.text().await.map_err(|e| e.to_string())?;
        let result: serde_json::Value = serde_json::from_str(&raw).map_err(|e| format!("{}", e))?;
        if let Some(logs) = result.get("logs") {
            return serde_json::from_value(logs.clone()).map_err(|e| e.to_string());
        }
        Ok(vec![])
    }

    pub async fn close_connection(&self, id: &str) -> Result<(), String> {
        let path = format!("/connections/{}", id);
        let url = format!("{}{}", self.base_url.trim_end_matches('/'), path);
        let mut req = self.client.delete(&url);
        if !self.api_key.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", self.api_key));
        }
        let resp = req.send().await.map_err(|e| e.to_string())?;
        if resp.status() == 204 {
            Ok(())
        } else {
            Err(format!("HTTP {}", resp.status()))
        }
    }

    pub async fn close_all_connections(&self) -> Result<(), String> {
        let path = "/connections";
        let url = format!("{}{}", self.base_url.trim_end_matches('/'), path);
        let mut req = self.client.delete(&url);
        if !self.api_key.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", self.api_key));
        }
        let resp = req.send().await.map_err(|e| e.to_string())?;
        if resp.status() == 204 {
            Ok(())
        } else {
            Err(format!("HTTP {}", resp.status()))
        }
    }
}
