use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};

/// 创建一个 mock 服务器
pub async fn create_mock_server() -> MockServer {
    MockServer::start().await
}

/// 创建一个 mock 响应
pub fn create_mock_response(status_code: u16, body: &str) -> ResponseTemplate {
    ResponseTemplate::new(status_code).set_body_string(body)
}

/// 创建一个 JSON mock 响应
pub fn create_json_mock_response(status_code: u16, json: &serde_json::Value) -> ResponseTemplate {
    ResponseTemplate::new(status_code).set_body_json(json)
}
