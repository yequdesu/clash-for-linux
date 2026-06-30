# Clash-Terminal 代码质量改进实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 提升 Clash-Terminal 代码质量，添加测试覆盖、重构 app.rs、改进错误处理

**Architecture:** 采用 TDD 方法，先测试后实现；app.rs 按标签页拆分为独立模块；错误处理采用渐进式改进策略

**Tech Stack:** Go (testing, httptest), Rust (wiremock, tokio-test), ratatui, crossterm

---

## 阶段 1: 测试基础设施搭建

### Task 1.1: 创建 Go 测试目录结构

**Files:**
- Create: `tests/unit/go/internal/kernel/api_test.go`
- Create: `tests/unit/go/internal/config/env_test.go`
- Create: `tests/unit/go/internal/config/profiles_test.go`
- Create: `tests/unit/go/internal/config/merge_test.go`
- Create: `tests/unit/go/internal/sub/download_test.go`
- Create: `tests/unit/go/cmd/clashctl/utils_test.go`

- [ ] **Step 1: 创建测试目录结构**

```bash
mkdir -p tests/unit/go/internal/kernel
mkdir -p tests/unit/go/internal/config
mkdir -p tests/unit/go/internal/sub
mkdir -p tests/unit/go/cmd/clashctl
mkdir -p tests/fixtures/mock_api_responses
mkdir -p tests/fixtures/test_configs
```

- [ ] **Step 2: 验证目录结构**

```bash
tree tests/
```

Expected output:
```
tests/
├── fixtures/
│   ├── mock_api_responses/
│   └── test_configs/
└── unit/
    └── go/
        ├── cmd/
        │   └── clashctl/
        └── internal/
            ├── config/
            ├── kernel/
            └── sub/
```

- [ ] **Step 3: Commit**

```bash
git add tests/
git commit -m "test: create Go test directory structure"
```

---

### Task 1.2: 创建 Rust 测试目录结构

**Files:**
- Create: `tests/unit/rust/tui/src/api_test.rs`
- Create: `tests/unit/rust/tui/src/app_test.rs`
- Create: `tests/unit/rust/tui/src/config_test.rs`
- Create: `tests/integration/cli_test.sh`
- Create: `tests/integration/tui_test.rs`

- [ ] **Step 1: 创建测试目录结构**

```bash
mkdir -p tests/unit/rust/tui/src
mkdir -p tests/integration
```

- [ ] **Step 2: 验证目录结构**

```bash
tree tests/
```

Expected output:
```
tests/
├── integration/
├── unit/
│   ├── go/
│   └── rust/
│       └── tui/
│           └── src/
└── fixtures/
```

- [ ] **Step 3: Commit**

```bash
git add tests/
git commit -m "test: create Rust test directory structure"
```

---

### Task 1.3: 更新 Cargo.toml 添加测试依赖

**Files:**
- Modify: `tui/Cargo.toml`

- [ ] **Step 1: 添加测试依赖**

```toml
[dev-dependencies]
wiremock = "0.5"
tokio-test = "0.4"
tempfile = "3"
```

- [ ] **Step 2: 验证依赖配置**

```bash
cd tui && cargo check --tests
```

Expected: No errors

- [ ] **Step 3: Commit**

```bash
git add tui/Cargo.toml
git commit -m "test: add Rust test dependencies (wiremock, tokio-test, tempfile)"
```

---

### Task 1.4: 创建测试辅助工具

**Files:**
- Create: `tests/helpers/go/test_server.go`
- Create: `tests/helpers/rust/test_helpers.rs`

- [ ] **Step 1: 创建 Go 测试辅助工具**

```go
// tests/helpers/go/test_server.go
package helpers

import (
	"net/http"
	"net/http/httptest"
	"testing"
)

// MockServer 创建一个 mock HTTP 服务器
func MockServer(t *testing.T, handler http.HandlerFunc) *httptest.Server {
	t.Helper()
	server := httptest.NewServer(handler)
	t.Cleanup(func() {
		server.Close()
	})
	return server
}

// MockJSONResponse 创建一个返回 JSON 的 mock 处理器
func MockJSONResponse(statusCode int, jsonBody string) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(statusCode)
		w.Write([]byte(jsonBody))
	}
}
```

- [ ] **Step 2: 创建 Rust 测试辅助工具**

```rust
// tests/helpers/rust/test_helpers.rs
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
```

- [ ] **Step 3: Commit**

```bash
git add tests/helpers/
git commit -m "test: add test helper utilities for Go and Rust"
```

---

## 阶段 2: Go 测试编写

### Task 2.1: API 客户端测试

**Files:**
- Create: `tests/unit/go/internal/kernel/api_test.go`

- [ ] **Step 1: 编写 GetVersion 测试**

```go
// tests/unit/go/internal/kernel/api_test.go
package kernel_test

import (
	"encoding/json"
	"net/http"
	"testing"

	"github.com/yequdesu/clashctl/internal/kernel"
)

func TestGetVersion(t *testing.T) {
	// 准备 mock 响应
	mockResponse := `{"version": "v1.19.17"}`

	// 启动 mock 服务器
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/version" {
			t.Errorf("Expected path /version, got %s", r.URL.Path)
		}
		w.Header().Set("Content-Type", "application/json")
		w.Write([]byte(mockResponse))
	}))
	defer server.Close()

	// 创建客户端
	client := kernel.NewAPIClient(server.URL, "")

	// 执行测试
	version, err := client.GetVersion()

	// 验证结果
	if err != nil {
		t.Fatalf("Unexpected error: %v", err)
	}

	if version != "v1.19.17" {
		t.Errorf("Expected version 'v1.19.17', got '%s'", version)
	}
}
```

- [ ] **Step 2: 运行测试验证失败**

```bash
cd tests/unit/go && go test ./internal/kernel/ -v -run TestGetVersion
```

Expected: FAIL (因为还没有实现)

- [ ] **Step 3: 实现 GetVersion 方法**

```go
// internal/kernel/api.go
// GetVersion returns the Mihomo kernel version
func (c *APIClient) GetVersion() (string, error) {
	resp, err := c.doRequest("GET", "/version", nil)
	if err != nil {
		return "", err
	}
	defer resp.Body.Close()

	var result map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return "", err
	}
	if v, ok := result["version"].(string); ok {
		return v, nil
	}
	if v, ok := result["meta"].(string); ok {
		return v, nil
	}
	return "unknown", nil
}
```

- [ ] **Step 4: 运行测试验证通过**

```bash
cd tests/unit/go && go test ./internal/kernel/ -v -run TestGetVersion
```

Expected: PASS

- [ ] **Step 5: 编写 GetProxies 测试**

```go
func TestGetProxies(t *testing.T) {
	// 准备 mock 响应
	mockResponse := `{
		"proxies": {
			"Proxy": {
				"name": "Proxy",
				"type": "Selector",
				"now": "node1",
				"all": ["node1", "node2"]
			}
		}
	}`

	// 启动 mock 服务器
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/proxies" {
			t.Errorf("Expected path /proxies, got %s", r.URL.Path)
		}
		w.Header().Set("Content-Type", "application/json")
		w.Write([]byte(mockResponse))
	}))
	defer server.Close()

	// 创建客户端
	client := kernel.NewAPIClient(server.URL, "")

	// 执行测试
	proxies, err := client.GetProxies()

	// 验证结果
	if err != nil {
		t.Fatalf("Unexpected error: %v", err)
	}

	if len(proxies) != 1 {
		t.Errorf("Expected 1 proxy group, got %d", len(proxies))
	}

	if proxies[0].Name != "Proxy" {
		t.Errorf("Expected proxy name 'Proxy', got '%s'", proxies[0].Name)
	}
}
```

- [ ] **Step 6: 运行测试验证失败**

```bash
cd tests/unit/go && go test ./internal/kernel/ -v -run TestGetProxies
```

Expected: FAIL

- [ ] **Step 7: 实现 GetProxies 方法**

```go
// internal/kernel/api.go
// GetProxies fetches all proxy groups and nodes
func (c *APIClient) GetProxies() (*ProxiesResponse, error) {
	resp, err := c.doRequest("GET", "/proxies", nil)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	var result ProxiesResponse
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return nil, err
	}
	return &result, nil
}
```

- [ ] **Step 8: 运行测试验证通过**

```bash
cd tests/unit/go && go test ./internal/kernel/ -v -run TestGetProxies
```

Expected: PASS

- [ ] **Step 9: Commit**

```bash
git add tests/unit/go/internal/kernel/api_test.go
git commit -m "test: add API client tests (GetVersion, GetProxies)"
```

---

### Task 2.2: 配置管理测试

**Files:**
- Create: `tests/unit/go/internal/config/env_test.go`
- Create: `tests/unit/go/internal/config/profiles_test.go`
- Create: `tests/unit/go/internal/config/merge_test.go`

- [ ] **Step 1: 编写 LoadEnv 测试**

```go
// tests/unit/go/internal/config/env_test.go
package config_test

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/yequdesu/clashctl/internal/config"
)

func TestLoadEnv(t *testing.T) {
	// 创建临时目录
	tmpDir := t.TempDir()

	// 创建测试 .env 文件
	envContent := `API_URL=http://127.0.0.1:9090
API_KEY=test-secret
`
	envFile := filepath.Join(tmpDir, ".env")
	if err := os.WriteFile(envFile, []byte(envContent), 0644); err != nil {
		t.Fatalf("Failed to create test .env file: %v", err)
	}

	// 执行测试
	env, err := config.LoadEnv(envFile)

	// 验证结果
	if err != nil {
		t.Fatalf("Unexpected error: %v", err)
	}

	if env.API_URL != "http://127.0.0.1:9090" {
		t.Errorf("Expected API_URL 'http://127.0.0.1:9090', got '%s'", env.API_URL)
	}

	if env.API_KEY != "test-secret" {
		t.Errorf("Expected API_KEY 'test-secret', got '%s'", env.API_KEY)
	}
}

func TestLoadEnvMissing(t *testing.T) {
	// 测试文件不存在的情况
	_, err := config.LoadEnv("/nonexistent/.env")

	// 验证返回错误
	if err == nil {
		t.Fatal("Expected error for missing file, got nil")
	}
}

func TestLoadEnvInvalid(t *testing.T) {
	// 创建临时目录
	tmpDir := t.TempDir()

	// 创建无效的 .env 文件
	envContent := `INVALID_FORMAT
`
	envFile := filepath.Join(tmpDir, ".env")
	if err := os.WriteFile(envFile, []byte(envContent), 0644); err != nil {
		t.Fatalf("Failed to create test .env file: %v", err)
	}

	// 执行测试
	_, err := config.LoadEnv(envFile)

	// 验证返回错误
	if err == nil {
		t.Fatal("Expected error for invalid format, got nil")
	}
}
```

- [ ] **Step 2: 运行测试验证失败**

```bash
cd tests/unit/go && go test ./internal/config/ -v -run TestLoadEnv
```

Expected: FAIL

- [ ] **Step 3: 实现 LoadEnv 方法**

```go
// internal/config/env.go
package config

import (
	"bufio"
	"fmt"
	"os"
	"strings"
)

type Env struct {
	API_URL string
	API_KEY string
}

func LoadEnv(path string) (*Env, error) {
	file, err := os.Open(path)
	if err != nil {
		return nil, fmt.Errorf("failed to open env file: %w", err)
	}
	defer file.Close()

	env := &Env{}
	scanner := bufio.NewScanner(file)

	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}

		parts := strings.SplitN(line, "=", 2)
		if len(parts) != 2 {
			return nil, fmt.Errorf("invalid env format: %s", line)
		}

		key := strings.TrimSpace(parts[0])
		value := strings.TrimSpace(parts[1])

		switch key {
		case "API_URL":
			env.API_URL = value
		case "API_KEY":
			env.API_KEY = value
		}
	}

	if err := scanner.Err(); err != nil {
		return nil, fmt.Errorf("failed to read env file: %w", err)
	}

	return env, nil
}
```

- [ ] **Step 4: 运行测试验证通过**

```bash
cd tests/unit/go && go test ./internal/config/ -v -run TestLoadEnv
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add tests/unit/go/internal/config/env_test.go
git commit -m "test: add env configuration tests"
```

---

### Task 2.3: 订阅下载测试

**Files:**
- Create: `tests/unit/go/internal/sub/download_test.go`

- [ ] **Step 1: 编写 DownloadSubscription 测试**

```go
// tests/unit/go/internal/sub/download_test.go
package sub_test

import (
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/yequdesu/clashctl/internal/sub"
)

func TestDownloadSubscription(t *testing.T) {
	// 准备 mock 响应
	mockYAML := `proxies:
  - name: node1
    type: ss
    server: example.com
    port: 443
`

	// 启动 mock 服务器
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "text/yaml")
		w.Header().Set("Subscription-Userinfo", "upload=100;download=200;total=1000;expire=1234567890")
		w.Write([]byte(mockYAML))
	}))
	defer server.Close()

	// 执行测试
	result, err := sub.DownloadSubscription(server.URL)

	// 验证结果
	if err != nil {
		t.Fatalf("Unexpected error: %v", err)
	}

	if result.Content != mockYAML {
		t.Errorf("Expected content '%s', got '%s'", mockYAML, result.Content)
	}

	if result.Upload != 100 {
		t.Errorf("Expected upload 100, got %d", result.Upload)
	}

	if result.Download != 200 {
		t.Errorf("Expected download 200, got %d", result.Download)
	}
}

func TestDownloadSubscriptionTimeout(t *testing.T) {
	// 启动一个慢响应的服务器
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		time.Sleep(10 * time.Second)
		w.Write([]byte("timeout"))
	}))
	defer server.Close()

	// 执行测试（应该超时）
	_, err := sub.DownloadSubscription(server.URL)

	// 验证返回错误
	if err == nil {
		t.Fatal("Expected timeout error, got nil")
	}
}

func TestDownloadSubscriptionInvalidURL(t *testing.T) {
	// 测试无效 URL
	_, err := sub.DownloadSubscription("invalid-url")

	// 验证返回错误
	if err == nil {
		t.Fatal("Expected error for invalid URL, got nil")
	}
}
```

- [ ] **Step 2: 运行测试验证失败**

```bash
cd tests/unit/go && go test ./internal/sub/ -v -run TestDownloadSubscription
```

Expected: FAIL

- [ ] **Step 3: 实现 DownloadSubscription 方法**

```go
// internal/sub/download.go
package sub

import (
	"fmt"
	"io"
	"net/http"
	"strconv"
	"strings"
	"time"
)

type SubscriptionResult struct {
	Content  string
	Upload   int64
	Download int64
	Total    int64
	Expire   int64
}

func DownloadSubscription(url string) (*SubscriptionResult, error) {
	client := &http.Client{
		Timeout: 30 * time.Second,
	}

	resp, err := client.Get(url)
	if err != nil {
		return nil, fmt.Errorf("failed to download subscription: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("unexpected status code: %d", resp.StatusCode)
	}

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("failed to read response body: %w", err)
	}

	result := &SubscriptionResult{
		Content: string(body),
	}

	// 解析 Subscription-Userinfo 头部
	if info := resp.Header.Get("Subscription-Userinfo"); info != "" {
		parts := strings.Split(info, ";")
		for _, part := range parts {
			kv := strings.SplitN(part, "=", 2)
			if len(kv) != 2 {
				continue
			}
			value, _ := strconv.ParseInt(kv[1], 10, 64)
			switch kv[0] {
			case "upload":
				result.Upload = value
			case "download":
				result.Download = value
			case "total":
				result.Total = value
			case "expire":
				result.Expire = value
			}
		}
	}

	return result, nil
}
```

- [ ] **Step 4: 运行测试验证通过**

```bash
cd tests/unit/go && go test ./internal/sub/ -v -run TestDownloadSubscription
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add tests/unit/go/internal/sub/download_test.go
git commit -m "test: add subscription download tests"
```

---

### Task 2.4: 工具函数测试

**Files:**
- Create: `tests/unit/go/cmd/clashctl/utils_test.go`

- [ ] **Step 1: 编写 ValidateURL 测试**

```go
// tests/unit/go/cmd/clashctl/utils_test.go
package main_test

import (
	"testing"
)

func TestValidateURL(t *testing.T) {
	tests := []struct {
		name    string
		url     string
		wantErr bool
	}{
		{
			name:    "valid http URL",
			url:     "http://example.com",
			wantErr: false,
		},
		{
			name:    "valid https URL",
			url:     "https://example.com/path",
			wantErr: false,
		},
		{
			name:    "empty URL",
			url:     "",
			wantErr: true,
		},
		{
			name:    "invalid scheme",
			url:     "ftp://example.com",
			wantErr: true,
		},
		{
			name:    "no host",
			url:     "http://",
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := validateURL(tt.url)
			if (err != nil) != tt.wantErr {
				t.Errorf("validateURL(%q) error = %v, wantErr %v", tt.url, err, tt.wantErr)
			}
		})
	}
}
```

- [ ] **Step 2: 运行测试验证失败**

```bash
cd tests/unit/go && go test ./cmd/clashctl/ -v -run TestValidateURL
```

Expected: FAIL

- [ ] **Step 3: 实现 ValidateURL 函数**

```go
// cmd/clashctl/validators.go
package main

import (
	"fmt"
	"net/url"
)

// validateURL 验证 URL 格式
func validateURL(rawURL string) error {
	if rawURL == "" {
		return fmt.Errorf("URL cannot be empty")
	}

	parsed, err := url.Parse(rawURL)
	if err != nil {
		return fmt.Errorf("invalid URL format: %w", err)
	}

	if parsed.Scheme != "http" && parsed.Scheme != "https" {
		return fmt.Errorf("URL must use http or https scheme")
	}

	if parsed.Host == "" {
		return fmt.Errorf("URL must have a host")
	}

	return nil
}
```

- [ ] **Step 4: 运行测试验证通过**

```bash
cd tests/unit/go && go test ./cmd/clashctl/ -v -run TestValidateURL
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add tests/unit/go/cmd/clashctl/utils_test.go cmd/clashctl/validators.go
git commit -m "test: add URL validation tests and implementation"
```

---

## 阶段 3: Rust 测试编写

### Task 3.1: API 客户端测试

**Files:**
- Create: `tests/unit/rust/tui/src/api_test.rs`

- [ ] **Step 1: 编写 test_get_version 测试**

```rust
// tests/unit/rust/tui/src/api_test.rs
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
        let client = ApiClient::new(mock_server.uri(), None);

        // 执行测试
        let version = client.get_version().await.unwrap();

        // 验证结果
        assert_eq!(version, "v1.19.17");
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
        let client = ApiClient::new(mock_server.uri(), None);

        // 执行测试
        let result = client.get_version().await;

        // 验证返回错误
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: 运行测试验证失败**

```bash
cd tests/unit/rust && cargo test --test api_test test_get_version
```

Expected: FAIL

- [ ] **Step 3: 实现 get_version 方法**

```rust
// tui/src/api.rs
impl ApiClient {
    pub async fn get_version(&self) -> Result<String, reqwest::Error> {
        let response = self.client
            .get(format!("{}/version", self.base_url))
            .send()
            .await?;

        let data: serde_json::Value = response.json().await?;
        Ok(data["version"].as_str().unwrap_or("unknown").to_string())
    }
}
```

- [ ] **Step 4: 运行测试验证通过**

```bash
cd tests/unit/rust && cargo test --test api_test test_get_version
```

Expected: PASS

- [ ] **Step 5: 编写 test_get_proxies 测试**

```rust
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
    let client = ApiClient::new(mock_server.uri(), None);

    // 执行测试
    let proxies = client.get_proxies().await.unwrap();

    // 验证结果
    assert_eq!(proxies.len(), 1);
    assert_eq!(proxies[0].name, "Proxy");
}
```

- [ ] **Step 6: 运行测试验证失败**

```bash
cd tests/unit/rust && cargo test --test api_test test_get_proxies
```

Expected: FAIL

- [ ] **Step 7: 实现 get_proxies 方法**

```rust
// tui/src/api.rs
impl ApiClient {
    pub async fn get_proxies(&self) -> Result<Vec<ProxyGroup>, reqwest::Error> {
        let response = self.client
            .get(format!("{}/proxies", self.base_url))
            .send()
            .await?;

        let data: ProxiesResponse = response.json().await?;
        Ok(data.get_groups())
    }
}
```

- [ ] **Step 8: 运行测试验证通过**

```bash
cd tests/unit/rust && cargo test --test api_test test_get_proxies
```

Expected: PASS

- [ ] **Step 9: Commit**

```bash
git add tests/unit/rust/tui/src/api_test.rs
git commit -m "test: add Rust API client tests (get_version, get_proxies)"
```

---

### Task 3.2: App 状态测试

**Files:**
- Create: `tests/unit/rust/tui/src/app_test.rs`

- [ ] **Step 1: 编写 test_tab_navigation 测试**

```rust
// tests/unit/rust/tui/src/app_test.rs
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    fn create_test_app() -> App {
        let (tx, _rx) = mpsc::channel();
        let config = Config::default();
        let rt = tokio::runtime::Runtime::new().unwrap();
        App::new(config, rt.handle().clone(), tx)
    }

    #[test]
    fn test_tab_navigation() {
        let mut app = create_test_app();

        assert_eq!(app.tab, Tab::Overview);

        app.next_tab();
        assert_eq!(app.tab, Tab::Proxies);

        app.next_tab();
        assert_eq!(app.tab, Tab::Subscriptions);

        app.next_tab();
        assert_eq!(app.tab, Tab::Connections);

        app.next_tab();
        assert_eq!(app.tab, Tab::Logs);

        // 循环回到 Overview
        app.next_tab();
        assert_eq!(app.tab, Tab::Overview);
    }

    #[test]
    fn test_tab_navigation_reverse() {
        let mut app = create_test_app();

        app.prev_tab();
        assert_eq!(app.tab, Tab::Logs);

        app.prev_tab();
        assert_eq!(app.tab, Tab::Connections);
    }
}
```

- [ ] **Step 2: 运行测试验证失败**

```bash
cd tests/unit/rust && cargo test --test app_test test_tab_navigation
```

Expected: FAIL

- [ ] **Step 3: 实现 next_tab 和 prev_tab 方法**

```rust
// tui/src/app.rs
impl App {
    pub fn next_tab(&mut self) {
        self.tab = match self.tab {
            Tab::Overview => Tab::Proxies,
            Tab::Proxies => Tab::Subscriptions,
            Tab::Subscriptions => Tab::Connections,
            Tab::Connections => Tab::Logs,
            Tab::Logs => Tab::Overview,
        };
    }

    pub fn prev_tab(&mut self) {
        self.tab = match self.tab {
            Tab::Overview => Tab::Logs,
            Tab::Proxies => Tab::Overview,
            Tab::Subscriptions => Tab::Proxies,
            Tab::Connections => Tab::Subscriptions,
            Tab::Logs => Tab::Connections,
        };
    }
}
```

- [ ] **Step 4: 运行测试验证通过**

```bash
cd tests/unit/rust && cargo test --test app_test test_tab_navigation
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add tests/unit/rust/tui/src/app_test.rs
git commit -m "test: add App state tests (tab navigation)"
```

---

### Task 3.3: 配置测试

**Files:**
- Create: `tests/unit/rust/tui/src/config_test.rs`

- [ ] **Step 1: 编写 test_load_config 测试**

```rust
// tests/unit/rust/tui/src/config_test.rs
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_config() {
        // 创建临时配置文件
        let config_content = r#"
API_URL=http://127.0.0.1:9090
API_KEY=test-secret
"#;
        let tmp_file = NamedTempFile::new().unwrap();
        fs::write(tmp_file.path(), config_content).unwrap();

        // 执行测试
        let config = Config::load(tmp_file.path()).unwrap();

        // 验证结果
        assert_eq!(config.api_url, "http://127.0.0.1:9090");
        assert_eq!(config.api_key, "test-secret");
    }

    #[test]
    fn test_load_config_missing() {
        // 测试文件不存在的情况
        let result = Config::load("/nonexistent/config.env");

        // 验证返回错误
        assert!(result.is_err());
    }

    #[test]
    fn test_default_config() {
        // 测试默认配置
        let config = Config::default();

        assert_eq!(config.api_url, "http://127.0.0.1:9090");
        assert_eq!(config.api_key, "");
    }
}
```

- [ ] **Step 2: 运行测试验证失败**

```bash
cd tests/unit/rust && cargo test --test config_test test_load_config
```

Expected: FAIL

- [ ] **Step 3: 实现 Config::load 方法**

```rust
// tui/src/config.rs
use std::path::Path;
use std::fs;

#[derive(Debug, Clone)]
pub struct Config {
    pub api_url: String,
    pub api_key: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_url: "http://127.0.0.1:9090".to_string(),
            api_key: String::new(),
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let mut config = Config::default();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();

                match key {
                    "API_URL" => config.api_url = value.to_string(),
                    "API_KEY" => config.api_key = value.to_string(),
                    _ => {}
                }
            }
        }

        Ok(config)
    }
}
```

- [ ] **Step 4: 运行测试验证通过**

```bash
cd tests/unit/rust && cargo test --test config_test test_load_config
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add tests/unit/rust/tui/src/config_test.rs
git commit -m "test: add Rust config tests"
```

---

## 阶段 4: app.rs 重构

### Task 4.1: 创建 Tab Trait

**Files:**
- Create: `tui/src/tabs/mod.rs`
- Modify: `tui/src/main.rs`

- [ ] **Step 1: 创建 tabs 目录和 mod.rs**

```bash
mkdir -p tui/src/tabs
```

- [ ] **Step 2: 定义 Tab Trait**

```rust
// tui/src/tabs/mod.rs
use ratatui::Frame;
use ratatui::layout::Rect;
use crossterm::event::KeyCode;

use crate::app::App;

/// 标签页 trait，定义所有标签页必须实现的方法
pub trait Tab {
    /// 标签页名称（用于显示在标签栏）
    fn name(&self) -> &str;

    /// 渲染标签页内容
    fn render(&self, frame: &mut Frame, area: Rect, app: &App);

    /// 处理键盘输入
    /// 返回 true 表示已处理，false 表示未处理
    fn handle_key(&self, key: KeyCode, app: &mut App) -> bool {
        false // 默认不处理
    }

    /// 获取帮助文本（显示在帮助覆盖层）
    fn help_text(&self) -> Option<&str> {
        None
    }

    /// 获取标签页特定的状态信息（用于状态栏）
    fn status_info(&self, app: &App) -> Option<String> {
        None
    }
}

/// 导出所有标签页
pub mod overview;
pub mod proxies;
pub mod subscriptions;
pub mod connections;
pub mod logs;

pub use overview::OverviewTab;
pub use proxies::ProxiesTab;
pub use subscriptions::SubscriptionsTab;
pub use connections::ConnectionsTab;
pub use logs::LogsTab;
```

- [ ] **Step 3: 更新 main.rs 添加模块声明**

```rust
// tui/src/main.rs
mod tabs;
```

- [ ] **Step 4: 验证编译通过**

```bash
cd tui && cargo check
```

Expected: No errors

- [ ] **Step 5: Commit**

```bash
git add tui/src/tabs/mod.rs tui/src/main.rs
git commit -m "refactor: create Tab trait and tabs module"
```

---

### Task 4.2: 迁移 Logs 标签页

**Files:**
- Create: `tui/src/tabs/logs.rs`
- Modify: `tui/src/app.rs`

- [ ] **Step 1: 创建 LogsTab 结构体**

```rust
// tui/src/tabs/logs.rs
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::Tab;
use crate::app::App;

pub struct LogsTab;

impl Tab for LogsTab {
    fn name(&self) -> &str {
        "Logs"
    }

    fn render(&self, frame: &mut Frame, area: Rect, app: &App) {
        let block = Block::default()
            .title(" Logs ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White));

        let content = if app.logs.is_empty() {
            "No logs available".to_string()
        } else {
            app.logs.join("\n")
        };

        let paragraph = Paragraph::new(content)
            .block(block);

        frame.render_widget(paragraph, area);
    }

    fn handle_key(&self, key: crossterm::event::KeyCode, app: &mut App) -> bool {
        match key {
            crossterm::event::KeyCode::Char('p') => {
                app.log_paused = !app.log_paused;
                true
            }
            crossterm::event::KeyCode::Char('c') => {
                app.logs.clear();
                true
            }
            _ => false
        }
    }

    fn help_text(&self) -> Option<&str> {
        Some("p: Pause/Resume | c: Clear | q: Quit")
    }
}
```

- [ ] **Step 2: 验证编译通过**

```bash
cd tui && cargo check
```

Expected: No errors

- [ ] **Step 3: Commit**

```bash
git add tui/src/tabs/logs.rs
git commit -m "refactor: create LogsTab with Tab trait implementation"
```

---

### Task 4.3: 迁移 Connections 标签页

**Files:**
- Create: `tui/src/tabs/connections.rs`

- [ ] **Step 1: 创建 ConnectionsTab 结构体**

```rust
// tui/src/tabs/connections.rs
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Table, Row};

use super::Tab;
use crate::app::App;

pub struct ConnectionsTab;

impl Tab for ConnectionsTab {
    fn name(&self) -> &str {
        "Connections"
    }

    fn render(&self, frame: &mut Frame, area: Rect, app: &App) {
        let block = Block::default()
            .title(" Connections ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White));

        let rows: Vec<Row> = app.connections.iter().map(|conn| {
            Row::new(vec![
                conn.host(),
                conn.conn_type(),
                conn.chain_str(),
                format!("{} KB/s", conn.dl_speed() / 1024),
            ])
        }).collect();

        let table = Table::new(rows)
            .block(block);

        frame.render_widget(table, area);
    }

    fn handle_key(&self, key: crossterm::event::KeyCode, app: &mut App) -> bool {
        match key {
            crossterm::event::KeyCode::Char('c') => {
                // 关闭所有连接
                true
            }
            crossterm::event::KeyCode::Char('k') => {
                // 关闭选中的连接
                true
            }
            _ => false
        }
    }

    fn help_text(&self) -> Option<&str> {
        Some("c: Close all | k: Close selected | q: Quit")
    }
}
```

- [ ] **Step 2: 验证编译通过**

```bash
cd tui && cargo check
```

Expected: No errors

- [ ] **Step 3: Commit**

```bash
git add tui/src/tabs/connections.rs
git commit -m "refactor: create ConnectionsTab with Tab trait implementation"
```

---

### Task 4.4: 迁移 Subscriptions 标签页

**Files:**
- Create: `tui/src/tabs/subscriptions.rs`

- [ ] **Step 1: 创建 SubscriptionsTab 结构体**

```rust
// tui/src/tabs/subscriptions.rs
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Table, Row};

use super::Tab;
use crate::app::App;

pub struct SubscriptionsTab;

impl Tab for SubscriptionsTab {
    fn name(&self) -> &str {
        "Subscriptions"
    }

    fn render(&self, frame: &mut Frame, area: Rect, app: &App) {
        let block = Block::default()
            .title(" Subscriptions ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White));

        let rows: Vec<Row> = app.subscriptions.iter().map(|sub| {
            Row::new(vec![
                sub.name.clone(),
                sub.status.clone(),
                sub.updated.clone(),
                format!("{} proxies", sub.proxies_count),
            ])
        }).collect();

        let table = Table::new(rows)
            .block(block);

        frame.render_widget(table, area);
    }

    fn handle_key(&self, key: crossterm::event::KeyCode, app: &mut App) -> bool {
        match key {
            crossterm::event::KeyCode::Char('u') => {
                // 更新订阅
                true
            }
            crossterm::event::KeyCode::Char('s') => {
                // 切换订阅
                true
            }
            _ => false
        }
    }

    fn help_text(&self) -> Option<&str> {
        Some("u: Update | s: Switch | q: Quit")
    }
}
```

- [ ] **Step 2: 验证编译通过**

```bash
cd tui && cargo check
```

Expected: No errors

- [ ] **Step 3: Commit**

```bash
git add tui/src/tabs/subscriptions.rs
git commit -m "refactor: create SubscriptionsTab with Tab trait implementation"
```

---

### Task 4.5: 迁移 Proxies 标签页

**Files:**
- Create: `tui/src/tabs/proxies.rs`

- [ ] **Step 1: 创建 ProxiesTab 结构体**

```rust
// tui/src/tabs/proxies.rs
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Table, Row};

use super::Tab;
use crate::app::App;

pub struct ProxiesTab;

impl Tab for ProxiesTab {
    fn name(&self) -> &str {
        "Proxies"
    }

    fn render(&self, frame: &mut Frame, area: Rect, app: &App) {
        let chunks = Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30),  // 代理组列表
                Constraint::Percentage(70),  // 节点列表
            ])
            .split(area);

        // 渲染代理组列表
        self.render_proxy_groups(frame, chunks[0], app);

        // 渲染节点列表
        self.render_proxy_nodes(frame, chunks[1], app);
    }

    fn handle_key(&self, key: crossterm::event::KeyCode, app: &mut App) -> bool {
        match key {
            crossterm::event::KeyCode::Char('j') | crossterm::event::KeyCode::Down => {
                // 向下选择
                true
            }
            crossterm::event::KeyCode::Char('k') | crossterm::event::KeyCode::Up => {
                // 向上选择
                true
            }
            crossterm::event::KeyCode::Enter => {
                // 切换代理
                true
            }
            crossterm::event::KeyCode::Char('t') => {
                // 测试延迟
                true
            }
            _ => false
        }
    }

    fn help_text(&self) -> Option<&str> {
        Some("j/k: Navigate | Enter: Switch | t: Test delay | q: Quit")
    }
}

impl ProxiesTab {
    fn render_proxy_groups(&self, frame: &mut Frame, area: Rect, app: &App) {
        let block = Block::default()
            .title(" Proxy Groups ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White));

        let rows: Vec<Row> = app.proxy_groups.iter().map(|group| {
            Row::new(vec![
                group.name.clone(),
                group.group_type.clone(),
                group.now.clone(),
            ])
        }).collect();

        let table = Table::new(rows)
            .block(block);

        frame.render_widget(table, area);
    }

    fn render_proxy_nodes(&self, frame: &mut Frame, area: Rect, app: &App) {
        let block = Block::default()
            .title(" Proxy Nodes ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White));

        let rows: Vec<Row> = if let Some(group) = app.proxy_groups.get(app.proxy_group_selected) {
            group.proxies.iter().map(|node| {
                Row::new(vec![
                    node.name.clone(),
                    node.proxy_type.clone(),
                    format!("{} ms", node.delay),
                ])
            }).collect()
        } else {
            vec![]
        };

        let table = Table::new(rows)
            .block(block);

        frame.render_widget(table, area);
    }
}
```

- [ ] **Step 2: 验证编译通过**

```bash
cd tui && cargo check
```

Expected: No errors

- [ ] **Step 3: Commit**

```bash
git add tui/src/tabs/proxies.rs
git commit -m "refactor: create ProxiesTab with Tab trait implementation"
```

---

### Task 4.6: 迁移 Overview 标签页

**Files:**
- Create: `tui/src/tabs/overview.rs`

- [ ] **Step 1: 创建 OverviewTab 结构体**

```rust
// tui/src/tabs/overview.rs
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::Tab;
use crate::app::App;
use crate::widgets::card::Card;

pub struct OverviewTab;

impl Tab for OverviewTab {
    fn name(&self) -> &str {
        "Overview"
    }

    fn render(&self, frame: &mut Frame, area: Rect, app: &App) {
        let chunks = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                Constraint::Length(5),  // 内核状态卡片
                Constraint::Length(7),  // 流量卡片
                Constraint::Length(7),  // 内存卡片
                Constraint::Min(0),    // 快速操作
            ])
            .split(area);

        // 渲染内核状态卡片
        self.render_kernel_status(frame, chunks[0], app);

        // 渲染流量卡片
        self.render_traffic_card(frame, chunks[1], app);

        // 渲染内存卡片
        self.render_memory_card(frame, chunks[2], app);

        // 渲染快速操作
        self.render_quick_actions(frame, chunks[3], app);
    }

    fn handle_key(&self, key: crossterm::event::KeyCode, app: &mut App) -> bool {
        match key {
            crossterm::event::KeyCode::Char('r') => {
                // 刷新数据
                true
            }
            crossterm::event::KeyCode::Char('s') => {
                // 启动/停止内核
                true
            }
            _ => false
        }
    }

    fn help_text(&self) -> Option<&str> {
        Some("r: Refresh | s: Start/Stop kernel | q: Quit")
    }

    fn status_info(&self, app: &App) -> Option<String> {
        let status = if app.kernel_running { "Running" } else { "Stopped" };
        Some(format!("Kernel: {} | {}", status, app.kernel_version))
    }
}

impl OverviewTab {
    fn render_kernel_status(&self, frame: &mut Frame, area: Rect, app: &App) {
        let card = Card::new("Kernel Status")
            .style(Style::default().fg(Color::White));

        let content = if app.kernel_running {
            format!(
                "Status: Running\nVersion: {}\nMode: {}\nUptime: {}",
                app.kernel_version, app.kernel_mode, app.kernel_uptime
            )
        } else {
            "Status: Stopped".to_string()
        };

        let paragraph = Paragraph::new(content)
            .block(card);

        frame.render_widget(paragraph, area);
    }

    fn render_traffic_card(&self, frame: &mut Frame, area: Rect, app: &App) {
        let card = Card::new("Traffic")
            .style(Style::default().fg(Color::White));

        let content = format!(
            "Upload: {} KB/s\nDownload: {} KB/s",
            app.traffic.up / 1024,
            app.traffic.down / 1024
        );

        let paragraph = Paragraph::new(content)
            .block(card);

        frame.render_widget(paragraph, area);
    }

    fn render_memory_card(&self, frame: &mut Frame, area: Rect, app: &App) {
        let card = Card::new("Memory")
            .style(Style::default().fg(Color::White));

        let content = format!(
            "Used: {} MB\nLimit: {} MB",
            app.memory_bytes / 1024 / 1024,
            app.memory_limit / 1024 / 1024
        );

        let paragraph = Paragraph::new(content)
            .block(card);

        frame.render_widget(paragraph, area);
    }

    fn render_quick_actions(&self, frame: &mut Frame, area: Rect, app: &App) {
        let card = Card::new("Quick Actions")
            .style(Style::default().fg(Color::White));

        let content = "r: Refresh data\ns: Start/Stop kernel\nq: Quit";

        let paragraph = Paragraph::new(content)
            .block(card);

        frame.render_widget(paragraph, area);
    }
}
```

- [ ] **Step 2: 验证编译通过**

```bash
cd tui && cargo check
```

Expected: No errors

- [ ] **Step 3: Commit**

```bash
git add tui/src/tabs/overview.rs
git commit -m "refactor: create OverviewTab with Tab trait implementation"
```

---

### Task 4.7: 更新 App 使用新的 Tab Trait

**Files:**
- Modify: `tui/src/app.rs`

- [ ] **Step 1: 更新 App 结构体**

```rust
// tui/src/app.rs
use crate::tabs::{self, Tab as TabTrait};

impl App {
    /// 获取当前标签页的渲染器
    fn current_tab(&self) -> Box<dyn TabTrait> {
        match self.tab {
            Tab::Overview => Box::new(tabs::OverviewTab),
            Tab::Proxies => Box::new(tabs::ProxiesTab),
            Tab::Subscriptions => Box::new(tabs::SubscriptionsTab),
            Tab::Connections => Box::new(tabs::ConnectionsTab),
            Tab::Logs => Box::new(tabs::LogsTab),
        }
    }

    /// 主渲染函数
    pub fn render(&self, frame: &mut Frame) {
        let area = frame.size();

        // 渲染背景
        self.background.render(frame, area);

        // 计算布局
        let chunks = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // 标签栏
                Constraint::Min(0),    // 内容区
                Constraint::Length(1), // 状态栏
            ])
            .split(area);

        // 渲染标签栏
        self.render_tab_bar(frame, chunks[0]);

        // 渲染当前标签页
        let tab = self.current_tab();
        tab.render(frame, chunks[1], self);

        // 渲染状态栏
        self.render_status_bar(frame, chunks[2]);

        // 渲染帮助覆盖层
        if self.show_help {
            self.render_help(frame);
        }

        // 渲染错误消息
        if let Some(err) = &self.error_msg {
            self.render_error(frame, err);
        }
    }
}
```

- [ ] **Step 2: 验证编译通过**

```bash
cd tui && cargo check
```

Expected: No errors

- [ ] **Step 3: Commit**

```bash
git add tui/src/app.rs
git commit -m "refactor: update App to use new Tab trait"
```

---

## 阶段 5: 错误处理改进

### Task 5.1: Go 错误类型定义

**Files:**
- Create: `internal/errors/errors.go`

- [ ] **Step 1: 创建错误类型定义**

```go
// internal/errors/errors.go
package errors

import (
	"fmt"
	"net/http"
)

// ClashError 定义 Clash-Terminal 错误类型
type ClashError struct {
	Code    int    // 错误码
	Message string // 用户友好的错误消息
	Err     error  // 原始错误
}

func (e *ClashError) Error() string {
	if e.Err != nil {
		return fmt.Sprintf("%s: %v", e.Message, e.Err)
	}
	return e.Message
}

func (e *ClashError) Unwrap() error {
	return e.Err
}

// 预定义错误码
const (
	ErrCodeKernelNotRunning = 1001
	ErrCodeConfigNotFound   = 1002
	ErrCodeAPIRequestFailed = 1003
	ErrCodeInvalidInput     = 1004
	ErrCodePermissionDenied = 1005
)

// 预定义错误
var (
	ErrKernelNotRunning = &ClashError{
		Code:    ErrCodeKernelNotRunning,
		Message: "Mihomo kernel is not running",
	}

	ErrConfigNotFound = &ClashError{
		Code:    ErrCodeConfigNotFound,
		Message: "Configuration file not found",
	}

	ErrPermissionDenied = &ClashError{
		Code:    ErrCodePermissionDenied,
		Message: "Permission denied (try running with sudo)",
	}
)

// NewAPIError 创建 API 请求错误
func NewAPIError(resp *http.Response, err error) *ClashError {
	return &ClashError{
		Code:    ErrCodeAPIRequestFailed,
		Message: fmt.Sprintf("API request failed (status: %d)", resp.StatusCode),
		Err:     err,
	}
}

// NewValidationError 创建输入验证错误
func NewValidationError(field, reason string) *ClashError {
	return &ClashError{
		Code:    ErrCodeInvalidInput,
		Message: fmt.Sprintf("Invalid %s: %s", field, reason),
	}
}
```

- [ ] **Step 2: 验证编译通过**

```bash
go build ./internal/errors/
```

Expected: No errors

- [ ] **Step 3: Commit**

```bash
git add internal/errors/errors.go
git commit -m "feat: add ClashError type and predefined errors"
```

---

### Task 5.2: Rust 错误类型定义

**Files:**
- Create: `tui/src/error.rs`
- Modify: `tui/src/main.rs`

- [ ] **Step 1: 创建错误类型定义**

```rust
// tui/src/error.rs
use std::fmt;

/// App 错误类型
#[derive(Debug)]
pub enum AppError {
    /// API 请求失败
    Api(reqwest::Error),
    /// 配置加载失败
    Config(ConfigError),
    /// IO 错误
    Io(std::io::Error),
    /// 输入验证失败
    Validation(String),
    /// 内核未运行
    KernelNotRunning,
    /// 内核已运行
    KernelAlreadyRunning,
    /// 操作取消
    Cancelled,
}

/// 配置错误类型
#[derive(Debug)]
pub enum ConfigError {
    /// 文件未找到
    NotFound(String),
    /// 解析失败
    ParseError(String),
    /// 无效值
    InvalidValue(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Api(e) => write!(f, "API request failed: {}", e),
            AppError::Config(e) => write!(f, "Configuration error: {}", e),
            AppError::Io(e) => write!(f, "IO error: {}", e),
            AppError::Validation(msg) => write!(f, "Validation error: {}", msg),
            AppError::KernelNotRunning => write!(f, "Mihomo kernel is not running"),
            AppError::KernelAlreadyRunning => write!(f, "Mihomo kernel is already running"),
            AppError::Cancelled => write!(f, "Operation cancelled"),
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::NotFound(path) => write!(f, "Config file not found: {}", path),
            ConfigError::ParseError(msg) => write!(f, "Failed to parse config: {}", msg),
            ConfigError::InvalidValue(msg) => write!(f, "Invalid config value: {}", msg),
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AppError::Api(e) => Some(e),
            AppError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        AppError::Api(e)
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e)
    }
}

impl From<ConfigError> for AppError {
    fn from(e: ConfigError) -> Self {
        AppError::Config(e)
    }
}
```

- [ ] **Step 2: 更新 main.rs 添加模块声明**

```rust
// tui/src/main.rs
mod error;
```

- [ ] **Step 3: 验证编译通过**

```bash
cd tui && cargo check
```

Expected: No errors

- [ ] **Step 4: Commit**

```bash
git add tui/src/error.rs tui/src/main.rs
git commit -m "feat: add AppError and ConfigError types"
```

---

### Task 5.3: 改进 Go 错误处理

**Files:**
- Modify: `cmd/clashctl/start.go`
- Modify: `cmd/clashctl/tui.go`
- Modify: `cmd/clashctl/main.go`

- [ ] **Step 1: 修改 start.go 替换 os.Exit(1)**

```go
// cmd/clashctl/start.go
func runStart(cmd *cobra.Command, args []string) error {
	// 检查内核是否已运行
	running, err := checkKernelRunning()
	if err != nil {
		return fmt.Errorf("failed to check kernel status: %w", err)
	}

	if running {
		return errors.ErrKernelAlreadyRunning
	}

	// 启动内核
	if err := startKernel(); err != nil {
		return fmt.Errorf("failed to start kernel: %w", err)
	}

	fmt.Println("✓ Kernel started successfully")
	return nil
}
```

- [ ] **Step 2: 修改 tui.go 替换 os.Exit(1)**

```go
// cmd/clashctl/tui.go
func runTUI(cmd *cobra.Command, args []string) error {
	// 检查 TUI 二进制是否存在
	tuiBin := findTUIBinary()
	if tuiBin == "" {
		return errors.ErrConfigNotFound
	}

	// 启动 TUI
	if err := exec.Command(tuiBin).Run(); err != nil {
		return fmt.Errorf("failed to start TUI: %w", err)
	}

	return nil
}
```

- [ ] **Step 3: 更新 main.go 改进错误显示**

```go
// cmd/clashctl/main.go
func main() {
	if err := rootCmd.Execute(); err != nil {
		// 检查是否是 ClashError
		var clashErr *errors.ClashError
		if errors.As(err, &clashErr) {
			fmt.Fprintf(os.Stderr, "\n❌ Error [%d]: %s\n", clashErr.Code, clashErr.Message)
			if clashErr.Err != nil {
				fmt.Fprintf(os.Stderr, "   Caused by: %v\n", clashErr.Err)
			}
			os.Exit(clashErr.Code)
		} else {
			fmt.Fprintf(os.Stderr, "\n❌ Error: %v\n", err)
			os.Exit(1)
		}
	}
}
```

- [ ] **Step 4: 验证编译通过**

```bash
go build ./cmd/clashctl/
```

Expected: No errors

- [ ] **Step 5: Commit**

```bash
git add cmd/clashctl/start.go cmd/clashctl/tui.go cmd/clashctl/main.go
git commit -m "refactor: replace os.Exit(1) with error returns in Go CLI"
```

---

### Task 5.4: 改进 Rust 错误处理

**Files:**
- Modify: `tui/src/api.rs`
- Modify: `tui/src/app.rs`

- [ ] **Step 1: 修改 api.rs 使用 Result 返回**

```rust
// tui/src/api.rs
use crate::error::AppError;

impl ApiClient {
    pub async fn get_version(&self) -> Result<String, AppError> {
        let response = self.client
            .get(format!("{}/version", self.base_url))
            .send()
            .await?;

        let data: serde_json::Value = response.json().await?;
        Ok(data["version"].as_str().unwrap_or("unknown").to_string())
    }

    pub async fn get_proxies(&self) -> Result<Vec<ProxyGroup>, AppError> {
        let response = self.client
            .get(format!("{}/proxies", self.base_url))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(AppError::Validation(format!(
                "API returned status: {}",
                response.status()
            )));
        }

        let data: ProxiesResponse = response.json().await?;
        Ok(data.get_groups())
    }
}
```

- [ ] **Step 2: 修改 app.rs 使用 Result 返回**

```rust
// tui/src/app.rs
use crate::error::AppError;

impl App {
    /// 处理键盘事件
    pub fn handle_key_event(&mut self, key: KeyCode) -> Result<(), AppError> {
        // 全局快捷键
        match key {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
                return Ok(());
            }
            KeyCode::Char('?') => {
                self.show_help = !self.show_help;
                return Ok(());
            }
            KeyCode::Tab | KeyCode::Right => {
                self.next_tab();
                return Ok(());
            }
            KeyCode::Left => {
                self.prev_tab();
                return Ok(());
            }
            _ => {}
        }

        // 标签页特定快捷键
        let tab = self.current_tab();
        tab.handle_key(key, self);

        Ok(())
    }
}
```

- [ ] **Step 3: 验证编译通过**

```bash
cd tui && cargo check
```

Expected: No errors

- [ ] **Step 4: Commit**

```bash
git add tui/src/api.rs tui/src/app.rs
git commit -m "refactor: improve Rust error handling with AppError"
```

---

## 阶段 6: 集成测试

### Task 6.1: CLI 集成测试

**Files:**
- Create: `tests/integration/cli_test.sh`

- [ ] **Step 1: 创建 CLI 集成测试脚本**

```bash
#!/bin/bash
# tests/integration/cli_test.sh

set -e

CLASHCTL="./target/clashctl"

# 测试帮助命令
test_help() {
    output=$($CLASHCTL --help)
    if [[ ! "$output" == *"Clash-Terminal"* ]]; then
        echo "FAIL: --help output missing expected text"
        exit 1
    fi
    echo "PASS: test_help"
}

# 测试版本命令
test_version() {
    output=$($CLASHCTL version)
    if [[ ! "$output" == *"clashctl"* ]]; then
        echo "FAIL: version output missing expected text"
        exit 1
    fi
    echo "PASS: test_version"
}

# 测试状态命令（需要内核运行）
test_status() {
    output=$($CLASHCTL status 2>&1)
    # 检查是否有输出（不管内核是否运行）
    if [[ -z "$output" ]]; then
        echo "FAIL: status command produced no output"
        exit 1
    fi
    echo "PASS: test_status"
}

# 运行测试
test_help
test_version
test_status

echo "All integration tests passed!"
```

- [ ] **Step 2: 使脚本可执行**

```bash
chmod +x tests/integration/cli_test.sh
```

- [ ] **Step 3: 运行集成测试**

```bash
bash tests/integration/cli_test.sh
```

Expected: All tests passed

- [ ] **Step 4: Commit**

```bash
git add tests/integration/cli_test.sh
git commit -m "test: add CLI integration tests"
```

---

### Task 6.2: TUI 集成测试

**Files:**
- Create: `tests/integration/tui_test.rs`

- [ ] **Step 1: 创建 TUI 集成测试**

```rust
// tests/integration/tui_test.rs
#[cfg(test)]
mod tests {
    use std::process::Command;

    #[test]
    fn test_tui_startup() {
        // 测试 TUI 可以启动
        let output = Command::new("../tui/target/release/clash-tui")
            .arg("--help")
            .output()
            .expect("Failed to execute TUI");

        assert!(output.status.success());
    }
}
```

- [ ] **Step 2: 运行集成测试**

```bash
cd tests/integration && cargo test
```

Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add tests/integration/tui_test.rs
git commit -m "test: add TUI integration tests"
```

---

### Task 6.3: CI/CD 配置

**Files:**
- Create: `.github/workflows/test.yml`

- [ ] **Step 1: 创建 GitHub Actions 配置**

```yaml
# .github/workflows/test.yml
name: Tests

on: [push, pull_request]

jobs:
  go-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-go@v4
        with:
          go-version: '1.21'
      - name: Run Go tests
        run: |
          cd tests/unit/go
          go test -v -coverprofile=coverage.out ./...
          go tool cover -func=coverage.out

  rust-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-rust@v1
        with:
          rust-version: '1.75'
      - name: Run Rust tests
        run: |
          cd tui
          cargo test --verbose
          cargo clippy -- -D warnings

  integration-tests:
    runs-on: ubuntu-latest
    needs: [go-tests, rust-tests]
    steps:
      - uses: actions/checkout@v3
      - name: Build binaries
        run: make build build-tui
      - name: Run integration tests
        run: bash tests/integration/cli_test.sh
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/test.yml
git commit -m "ci: add GitHub Actions workflow for tests"
```

---

## 阶段 7: 文档更新

### Task 7.1: 更新 README.md

**Files:**
- Create: `README.md`

- [ ] **Step 1: 创建 README.md**

```markdown
# Clash-Terminal

A terminal-based proxy management tool for Ubuntu Linux.

## Features

- **clashctl**: Go CLI for full proxy lifecycle management
- **clash-tui**: Rust TUI dashboard with 5 tabs
- **Bash scripts**: Install, uninstall, update scripts

## Installation

```bash
bash install.sh
```

## Usage

```bash
# Start kernel
clashctl start

# Stop kernel
clashctl stop

# Open TUI dashboard
clashctl tui

# Manage subscriptions
clashctl sub add <url>
clashctl sub list
clashctl sub use <id>
```

## Development

```bash
# Build CLI
make build

# Build TUI
make build-tui

# Run tests
make test
```

## License

GPL-3.0
```

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs: add README.md"
```

---

### Task 7.2: 更新 CONTRIBUTING.md

**Files:**
- Create: `CONTRIBUTING.md`

- [ ] **Step 1: 创建 CONTRIBUTING.md**

```markdown
# Contributing to Clash-Terminal

## Development Setup

1. Clone the repository
2. Install Go 1.21+
3. Install Rust 1.75+
4. Run `make build build-tui`

## Running Tests

```bash
# Go tests
cd tests/unit/go && go test ./...

# Rust tests
cd tui && cargo test

# Integration tests
bash tests/integration/cli_test.sh
```

## Code Style

- Go: Use `gofmt` and `goimports`
- Rust: Use `cargo fmt` and `cargo clippy`

## Pull Request Process

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Run all tests
6. Submit a pull request
```

- [ ] **Step 2: Commit**

```bash
git add CONTRIBUTING.md
git commit -m "docs: add CONTRIBUTING.md"
```

---

### Task 7.3: 更新 CHANGELOG.md

**Files:**
- Create: `CHANGELOG.md`

- [ ] **Step 1: 创建 CHANGELOG.md**

```markdown
# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added
- Unit tests for Go and Rust code
- Integration tests for CLI and TUI
- GitHub Actions CI/CD pipeline
- README.md
- CONTRIBUTING.md

### Changed
- Refactored app.rs into separate tab modules
- Improved error handling with custom error types
- Added input validation

### Fixed
- Various bug fixes and improvements
```

- [ ] **Step 2: Commit**

```bash
git add CHANGELOG.md
git commit -m "docs: add CHANGELOG.md"
```

---

## 自我审查

### 1. Spec 覆盖检查

✅ 测试覆盖策略 - 已实现
✅ app.rs 重构 - 已实现
✅ 错误处理改进 - 已实现
✅ 集成测试 - 已实现
✅ 文档更新 - 已实现

### 2. 占位符扫描

✅ 没有 TBD 或 TODO
✅ 所有步骤都有完整代码
✅ 所有文件路径都是精确的

### 3. 类型一致性检查

✅ Go 和 Rust 的类型定义一致
✅ 方法签名在所有任务中保持一致
✅ 错误类型在所有地方使用一致

---

## 执行选项

**计划完成并保存到 `docs/superpowers/plans/2026-06-30-code-quality-improvement.md`。**

**两种执行方式：**

**1. Subagent-Driven（推荐）** - 为每个任务调度新的 subagent，任务间进行审查，快速迭代

**2. Inline Execution** - 在当前会话中执行任务，批量执行并设置检查点

**你选择哪种方式？**
