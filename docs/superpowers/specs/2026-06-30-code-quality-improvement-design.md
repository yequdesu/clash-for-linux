# Clash-Terminal 代码质量改进设计文档

> **日期**: 2026-06-30
> **目标**: 提升代码质量、可维护性和可靠性
> **范围**: 测试覆盖、架构重构、错误处理

---

## 目录

1. [概述](#1-概述)
2. [测试覆盖策略](#2-测试覆盖策略)
3. [app.rs 重构方案](#3-apprs-重构方案)
4. [错误处理改进](#4-错误处理改进)
5. [实施路线图](#5-实施路线图)
6. [风险和缓解](#6-风险和缓解)

---

## 1. 概述

### 1.1 当前问题

| 问题 | 严重程度 | 影响 |
|------|----------|------|
| 零测试覆盖 | 🔴 高 | 重构风险高，回归 bug 难以发现 |
| app.rs 过于庞大（1329行） | 🟡 中 | 难以维护，职责不清 |
| 错误处理不一致 | 🟡 中 | 用户体验差，调试困难 |

### 1.2 改进目标

1. **测试覆盖**: 核心模块达到 70%+ 测试覆盖率
2. **架构优化**: app.rs 拆分为独立的标签页模块
3. **错误处理**: 统一错误类型，优雅的错误传播和显示

### 1.3 设计原则

- **渐进式改进**: 不破坏现有功能
- **向后兼容**: 保持 CLI/TUI 接口不变
- **可测试性**: 新代码必须易于测试
- **单一职责**: 每个模块只做一件事

---

## 2. 测试覆盖策略

### 2.1 测试架构

```
tests/
├── unit/                    # 单元测试（快速，无外部依赖）
│   ├── go/
│   │   ├── internal/
│   │   │   ├── kernel/
│   │   │   │   ├── api_test.go
│   │   │   │   └── service_test.go
│   │   │   ├── config/
│   │   │   │   ├── env_test.go
│   │   │   │   ├── profiles_test.go
│   │   │   │   └── merge_test.go
│   │   │   └── sub/
│   │   │       └── download_test.go
│   │   └── cmd/clashctl/
│   │       └── utils_test.go
│   └── rust/
│       └── tui/src/
│           ├── api_test.rs
│           ├── app_test.rs
│           └── config_test.rs
├── integration/             # 集成测试（需要真实/模拟服务）
│   ├── cli_test.sh
│   └── tui_test.rs
└── fixtures/                # 测试数据
    ├── mock_api_responses/
    └── test_configs/
```

### 2.2 Go 测试策略

#### 2.2.1 API 客户端测试 (internal/kernel/api_test.go)

**测试方法**: 使用 `net/http/httptest` 创建 mock HTTP 服务器

**测试用例**:
- `TestGetVersion` - 测试获取内核版本
- `TestGetProxies` - 测试获取代理列表
- `TestGetProxyDelay` - 测试延迟测试
- `TestSwitchProxy` - 测试切换代理
- `TestGetConnections` - 测试获取连接列表
- `TestCloseConnection` - 测试关闭连接
- `TestGetTraffic` - 测试流量监控
- `TestGetMemory` - 测试内存使用
- `TestGetConfigs` - 测试获取配置
- `TestPatchConfigs` - 测试更新配置

**示例代码**:
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
    client := NewApiClient(server.URL, "")
    
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

#### 2.2.2 配置管理测试 (internal/config/)

**env_test.go**:
- `TestLoadEnv` - 测试加载 .env 文件
- `TestLoadEnvMissing` - 测试文件不存在的情况
- `TestLoadEnvInvalid` - 测试无效格式

**profiles_test.go**:
- `TestLoadProfiles` - 测试加载 profiles.yaml
- `TestAddProfile` - 测试添加订阅
- `TestRemoveProfile` - 测试删除订阅
- `TestSwitchProfile` - 测试切换订阅
- `TestGetActiveProfile` - 测试获取当前订阅

**merge_test.go**:
- `TestMergeConfigs` - 测试配置合并
- `TestMergeWithOverride` - 测试覆盖合并
- `TestMergeInvalidYAML` - 测试无效 YAML

#### 2.2.3 订阅下载测试 (internal/sub/download_test.go)

**测试方法**: 使用 `httptest` 模拟 HTTP 服务器

**测试用例**:
- `TestDownloadSubscription` - 测试正常下载
- `TestDownloadWithHeaders` - 测试带头部的下载
- `TestDownloadTimeout` - 测试超时处理
- `TestDownloadInvalidURL` - 测试无效 URL
- `TestParseSubscriptionHeader` - 测试头部解析

#### 2.2.4 工具函数测试 (cmd/clashctl/utils_test.go)

**测试用例**:
- `TestValidateURL` - 测试 URL 验证
- `TestValidatePath` - 测试路径验证
- `TestFormatBytes` - 测试字节格式化
- `TestFormatDuration` - 测试时间格式化

### 2.3 Rust 测试策略

#### 2.3.1 API 客户端测试 (tui/src/api_test.rs)

**测试方法**: 使用 `wiremock` 或 `mockall` mock HTTP 请求

**依赖添加** (Cargo.toml):
```toml
[dev-dependencies]
wiremock = "0.5"
tokio-test = "0.4"
```

**测试用例**:
- `test_get_version` - 测试获取版本
- `test_get_proxies` - 测试获取代理
- `test_get_proxy_delay` - 测试延迟测试
- `test_switch_proxy` - 测试切换代理
- `test_get_connections` - 测试获取连接
- `test_get_traffic` - 测试流量监控
- `test_get_memory` - 测试内存使用

**示例代码**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path};
    
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
    
    #[tokio::test]
    async fn test_get_proxies_error() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("GET"))
            .and(path("/proxies"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;
        
        let client = ApiClient::new(mock_server.uri(), None);
        let result = client.get_proxies().await;
        
        assert!(result.is_err());
    }
}
```

#### 2.3.2 App 状态测试 (tui/src/app_test.rs)

**测试用例**:
- `test_app_initialization` - 测试 App 初始化
- `test_tab_navigation` - 测试标签页切换
- `test_proxy_selection` - 测试代理选择
- `test_connection_selection` - 测试连接选择
- `test_log_filtering` - 测试日志过滤

**示例代码**:
```rust
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

#### 2.3.3 配置测试 (tui/src/config_test.rs)

**测试用例**:
- `test_load_config` - 测试加载配置
- `test_load_config_missing` - 测试配置文件不存在
- `test_load_config_invalid` - 测试无效配置
- `test_default_config` - 测试默认配置

### 2.4 集成测试策略

#### 2.4.1 CLI 集成测试 (tests/integration/cli_test.sh)

**测试方法**: 使用 Bash 脚本测试 CLI 命令

**测试用例**:
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

#### 2.4.2 TUI 集成测试 (tests/integration/tui_test.rs)

**测试方法**: 使用 `rexpect` 或 `expectrl` 测试 TUI 交互

**测试用例**:
- `test_tui_startup` - 测试 TUI 启动
- `test_tui_tab_switch` - 测试标签页切换
- `test_tui_quit` - 测试退出

### 2.5 测试覆盖率目标

| 模块 | 目标覆盖率 | 优先级 |
|------|------------|--------|
| internal/kernel/api.go | 80% | 🔴 高 |
| internal/config/*.go | 70% | 🟡 中 |
| internal/sub/download.go | 70% | 🟡 中 |
| tui/src/api.rs | 80% | 🔴 高 |
| tui/src/app.rs | 60% | 🟡 中 |
| cmd/clashctl/utils.go | 70% | 🟡 中 |

### 2.6 CI/CD 集成

**GitHub Actions 配置** (`.github/workflows/test.yml`):
```yaml
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
          cd cmd/clashctl
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

---

## 3. app.rs 重构方案

### 3.1 当前问题

**app.rs 文件分析**:
- **总行数**: 1329 行
- **职责**:
  - App 状态定义（~100行）
  - 5个标签页渲染（~800行）
  - 事件处理（~400行）
  - 辅助方法（~29行）

**问题**:
1. 违反单一职责原则
2. 难以定位和修改特定标签页的代码
3. 新增标签页需要修改 app.rs
4. 测试困难

### 3.2 目标结构

```
tui/src/
├── app.rs                  # App 状态定义 + 核心方法（~300行）
├── tabs/
│   ├── mod.rs              # Tab trait 定义 + 模块导出（~50行）
│   ├── overview.rs         # Overview 标签页（~200行）
│   ├── proxies.rs          # Proxies 标签页（~250行）
│   ├── subscriptions.rs    # Subscriptions 标签页（~200行）
│   ├── connections.rs      # Connections 标签页（~200行）
│   └── logs.rs             # Logs 标签页（~150行）
├── api.rs                  # API 客户端（保持不变）
├── config.rs               # 配置加载（保持不变）
├── event.rs                # 事件系统（保持不变）
├── theme.rs                # 主题定义（保持不变）
├── window.rs               # 窗口状态（保持不变）
├── background.rs           # 背景效果（保持不变）
└── widgets/                # 自定义控件（保持不变）
```

### 3.3 Tab Trait 设计

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

### 3.4 App 状态精简

```rust
// tui/src/app.rs
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use std::sync::mpsc;

use crate::api::{ApiClient, Connection, ProxyGroup, TrafficInfo};
use crate::background::{BackgroundEffect, ParticleField};
use crate::config::Config;
use crate::event::DataEvent;
use crate::theme::CLASH_THEME;
use crate::window::WindowState;
use crate::widgets::tab_bar::Tab;
use crate::tabs::{self, Tab as TabTrait};

pub struct App {
    // ===== 核心状态 =====
    pub current_tab: Tab,
    pub should_quit: bool,
    pub show_help: bool,
    pub error_msg: Option<String>,
    pub tick_count: u64,
    pub last_refresh: i64,
    
    // ===== 内核状态 =====
    pub kernel_running: bool,
    pub kernel_version: String,
    pub kernel_mode: String,
    pub kernel_uptime: String,
    pub tun_enabled: bool,
    
    // ===== 数据状态 =====
    pub traffic: TrafficInfo,
    pub traffic_history: Vec<u64>,
    
    pub proxy_groups: Vec<ProxyGroup>,
    pub proxy_selected: usize,
    pub proxy_group_selected: usize,
    pub proxy_scroll_offset: usize,
    
    pub connections: Vec<Connection>,
    pub connections_active: usize,
    pub connections_total: usize,
    pub connection_selected: usize,
    
    pub logs: Vec<String>,
    pub log_paused: bool,
    pub log_level_filter: String,
    pub log_scroll: usize,
    
    pub subscriptions: Vec<crate::api::SubscriptionInfo>,
    pub sub_selected: usize,
    pub sub_active_id: usize,
    
    pub memory_bytes: u64,
    pub memory_limit: u64,
    
    // ===== UI 状态 =====
    pub confirm_action: bool,
    pub confirm_timer: u16,
    pub window: WindowState,
    pub background: Box<dyn BackgroundEffect>,
    
    // ===== 依赖 =====
    pub api: ApiClient,
    pub config: Config,
    pub rt: tokio::runtime::Handle,
    pub data_tx: mpsc::Sender<DataEvent>,
}

impl App {
    pub fn new(config: Config, rt: tokio::runtime::Handle, data_tx: mpsc::Sender<DataEvent>) -> Self {
        let api = ApiClient::new(config.api_url.clone(), config.api_key.clone());
        
        Self {
            current_tab: Tab::Overview,
            should_quit: false,
            show_help: false,
            error_msg: None,
            tick_count: 0,
            last_refresh: 0,
            
            kernel_running: false,
            kernel_version: String::new(),
            kernel_mode: String::from("rule"),
            kernel_uptime: String::new(),
            tun_enabled: false,
            
            traffic: TrafficInfo { up: 0, down: 0 },
            traffic_history: Vec::new(),
            
            proxy_groups: Vec::new(),
            proxy_selected: 0,
            proxy_group_selected: 0,
            proxy_scroll_offset: 0,
            
            connections: Vec::new(),
            connections_active: 0,
            connections_total: 0,
            connection_selected: 0,
            
            logs: Vec::new(),
            log_paused: false,
            log_level_filter: String::from("info"),
            log_scroll: 0,
            
            subscriptions: Vec::new(),
            sub_selected: 0,
            sub_active_id: 0,
            
            memory_bytes: 0,
            memory_limit: 0,
            
            confirm_action: false,
            confirm_timer: 0,
            window: WindowState::default(),
            background: Box::new(ParticleField::new()),
            
            api,
            config,
            rt,
            data_tx,
        }
    }
    
    /// 获取当前标签页的渲染器
    fn current_tab(&self) -> Box<dyn TabTrait> {
        match self.current_tab {
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
    
    /// 处理键盘事件
    pub fn handle_key_event(&mut self, key: KeyCode) {
        // 全局快捷键
        match key {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
                return;
            }
            KeyCode::Char('?') => {
                self.show_help = !self.show_help;
                return;
            }
            KeyCode::Tab | KeyCode::Right => {
                self.next_tab();
                return;
            }
            KeyCode::Left => {
                self.prev_tab();
                return;
            }
            KeyCode::Char('1') => {
                self.current_tab = Tab::Overview;
                return;
            }
            KeyCode::Char('2') => {
                self.current_tab = Tab::Proxies;
                return;
            }
            KeyCode::Char('3') => {
                self.current_tab = Tab::Subscriptions;
                return;
            }
            KeyCode::Char('4') => {
                self.current_tab = Tab::Connections;
                return;
            }
            KeyCode::Char('5') => {
                self.current_tab = Tab::Logs;
                return;
            }
            _ => {}
        }
        
        // 标签页特定快捷键
        let tab = self.current_tab();
        if !tab.handle_key(key, self) {
            // 未处理的按键
        }
    }
    
    /// 切换到下一个标签页
    pub fn next_tab(&mut self) {
        self.current_tab = match self.current_tab {
            Tab::Overview => Tab::Proxies,
            Tab::Proxies => Tab::Subscriptions,
            Tab::Subscriptions => Tab::Connections,
            Tab::Connections => Tab::Logs,
            Tab::Logs => Tab::Overview,
        };
    }
    
    /// 切换到上一个标签页
    pub fn prev_tab(&mut self) {
        self.current_tab = match self.current_tab {
            Tab::Overview => Tab::Logs,
            Tab::Proxies => Tab::Overview,
            Tab::Subscriptions => Tab::Proxies,
            Tab::Connections => Tab::Subscriptions,
            Tab::Logs => Tab::Connections,
        };
    }
    
    /// 定时更新
    pub fn tick(&mut self) {
        self.tick_count += 1;
        
        // 每 30 秒刷新一次数据
        if self.tick_count % 30 == 0 {
            self.refresh_data();
        }
        
        // 更新确认计时器
        if self.confirm_action {
            self.confirm_timer += 1;
            if self.confirm_timer >= 60 {
                self.confirm_action = false;
                self.confirm_timer = 0;
            }
        }
    }
    
    /// 刷新数据
    fn refresh_data(&mut self) {
        // 这里会调用 API 获取最新数据
        // 具体实现会在事件处理中
    }
    
    /// 应用数据事件
    pub fn apply_data_event(&mut self, event: DataEvent) {
        match event {
            DataEvent::Traffic(info) => {
                self.traffic = info;
                self.traffic_history.push(info.up + info.down);
                if self.traffic_history.len() > 60 {
                    self.traffic_history.remove(0);
                }
            }
            DataEvent::Proxies(groups) => {
                self.proxy_groups = groups;
            }
            DataEvent::Connections(conns) => {
                self.connections = conns.clone();
                self.connections_active = conns.iter().filter(|c| c.active()).count();
                self.connections_total = conns.len();
            }
            DataEvent::Logs(logs) => {
                self.logs.extend(logs);
                // 保持日志数量在合理范围
                if self.logs.len() > 1000 {
                    self.logs.drain(0..500);
                }
            }
            DataEvent::Memory(bytes, limit) => {
                self.memory_bytes = bytes;
                self.memory_limit = limit;
            }
            DataEvent::Version(version) => {
                self.kernel_version = version;
                self.kernel_running = true;
            }
            DataEvent::Error(err) => {
                self.error_msg = Some(err);
            }
            _ => {}
        }
    }
    
    // ===== 渲染辅助方法 =====
    
    fn render_tab_bar(&self, frame: &mut Frame, area: Rect) {
        // 使用现有的 tab_bar widget
        // ...
    }
    
    fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        // 渲染状态栏
        // ...
    }
    
    fn render_help(&self, frame: &mut Frame) {
        // 渲染帮助覆盖层
        // ...
    }
    
    fn render_error(&self, frame: &mut Frame, error: &str) {
        // 渲染错误消息
        // ...
    }
}
```

### 3.5 标签页实现示例

#### 3.5.1 Overview 标签页

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
        // 渲染流量卡片
        // ...
    }
    
    fn render_memory_card(&self, frame: &mut Frame, area: Rect, app: &App) {
        // 渲染内存卡片
        // ...
    }
    
    fn render_quick_actions(&self, frame: &mut Frame, area: Rect, app: &App) {
        // 渲染快速操作按钮
        // ...
    }
}
```

#### 3.5.2 Proxies 标签页

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
        // 渲染代理组列表
        // ...
    }
    
    fn render_proxy_nodes(&self, frame: &mut Frame, area: Rect, app: &App) {
        // 渲染节点列表
        // ...
    }
}
```

### 3.6 重构步骤

#### 阶段 1: 准备工作（1天）
1. 创建 `tui/src/tabs/` 目录
2. 创建 `tui/src/tabs/mod.rs`，定义 Tab trait
3. 更新 `tui/src/main.rs`，添加 `mod tabs;`

#### 阶段 2: 迁移标签页（3-4天）
1. **Logs 标签页**（最简单，先迁移）
   - 从 app.rs 提取 Logs 渲染代码
   - 创建 `tui/src/tabs/logs.rs`
   - 实现 Tab trait
   - 测试
   
2. **Connections 标签页**
   - 从 app.rs 提取 Connections 渲染代码
   - 创建 `tui/src/tabs/connections.rs`
   - 实现 Tab trait
   - 测试
   
3. **Subscriptions 标签页**
   - 从 app.rs 提取 Subscriptions 渲染代码
   - 创建 `tui/src/tabs/subscriptions.rs`
   - 实现 Tab trait
   - 测试
   
4. **Proxies 标签页**
   - 从 app.rs 提取 Proxies 渲染代码
   - 创建 `tui/src/tabs/proxies.rs`
   - 实现 Tab trait
   - 测试
   
5. **Overview 标签页**（最复杂，最后迁移）
   - 从 app.rs 提取 Overview 渲染代码
   - 创建 `tui/src/tabs/overview.rs`
   - 实现 Tab trait
   - 测试

#### 阶段 3: 精简 app.rs（1天）
1. 删除 app.rs 中已迁移的渲染代码
2. 更新 App::render() 使用新的 Tab trait
3. 整理和优化 app.rs

#### 阶段 4: 测试和验证（1天）
1. 运行所有现有测试
2. 手动测试所有标签页功能
3. 修复发现的问题
4. 性能测试

### 3.7 迁移检查清单

对于每个标签页迁移，检查：

- [ ] 渲染代码正确提取
- [ ] 键盘快捷键正确处理
- [ ] 帮助文本正确显示
- [ ] 状态信息正确传递
- [ ] 与 App 状态正确交互
- [ ] 无回归 bug
- [ ] 代码风格一致

---

## 4. 错误处理改进

### 4.1 Go 错误处理改进

#### 4.1.1 当前问题

**硬退出示例** (cmd/clashctl/start.go):
```go
// ❌ 当前代码
if err != nil {
    fmt.Fprintf(os.Stderr, "Error: %v\n", err)
    os.Exit(1)  // 硬退出，不给调用者处理机会
}
```

**问题**:
1. 无法被测试捕获
2. 不利于错误传播
3. 用户体验差（突然退出）

#### 4.1.2 改进方案

**定义错误类型** (internal/errors/errors.go):
```go
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

**改进后的代码** (cmd/clashctl/start.go):
```go
// ✅ 改进后
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

**主函数错误处理** (cmd/clashctl/main.go):
```go
func main() {
    if err := rootCmd.Execute(); err != nil {
        // 检查是否是 ClashError
        var clashErr *errors.ClashError
        if errors.As(err, &clashErr) {
            fmt.Fprintf(os.Stderr, "❌ Error [%d]: %s\n", clashErr.Code, clashErr.Message)
            if clashErr.Err != nil {
                fmt.Fprintf(os.Stderr, "   Caused by: %v\n", clashErr.Err)
            }
            os.Exit(clashErr.Code)
        } else {
            fmt.Fprintf(os.Stderr, "❌ Error: %v\n", err)
            os.Exit(1)
        }
    }
}
```

#### 4.1.3 需要修改的文件

| 文件 | 当前 os.Exit(1) | 改进方式 |
|------|-----------------|----------|
| cmd/clashctl/start.go | 8处 | 返回 error |
| cmd/clashctl/tui.go | 1处 | 返回 error |
| cmd/clashctl/main.go | 1处 | 保留（程序入口） |

#### 4.1.4 输入验证

**添加验证函数** (cmd/clashctl/validators.go):
```go
package main

import (
    "fmt"
    "net/url"
    "os"
    "path/filepath"
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

// validatePath 验证路径是否存在
func validatePath(path string) error {
    if path == "" {
        return fmt.Errorf("path cannot be empty")
    }
    
    // 转换为绝对路径
    absPath, err := filepath.Abs(path)
    if err != nil {
        return fmt.Errorf("invalid path: %w", err)
    }
    
    // 检查路径是否存在
    if _, err := os.Stat(absPath); os.IsNotExist(err) {
        return fmt.Errorf("path does not exist: %s", absPath)
    }
    
    return nil
}

// validatePort 验证端口号
func validatePort(port int) error {
    if port < 1 || port > 65535 {
        return fmt.Errorf("port must be between 1 and 65535")
    }
    return nil
}
```

### 4.2 Rust 错误处理改进

#### 4.2.1 定义错误类型

**创建错误模块** (tui/src/error.rs):
```rust
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

#### 4.2.2 改进 API 客户端

**改进 api.rs**:
```rust
use crate::error::AppError;

impl ApiClient {
    /// 获取代理列表
    pub async fn get_proxies(&self) -> Result<Vec<ProxyGroup>, AppError> {
        let response = self.client
            .get(format!("{}/proxies", self.base_url))
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Api(
                reqwest::Error::from(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("API returned status {}: {}", status, body),
                ))
            ));
        }
        
        let data: ProxiesResponse = response.json().await?;
        Ok(data.proxies)
    }
    
    /// 测试代理延迟
    pub async fn test_proxy_delay(&self, name: &str, timeout: u64) -> Result<u64, AppError> {
        let response = self.client
            .get(format!("{}/proxies/{}/delay", self.base_url, name))
            .query(&[("timeout", timeout)])
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(AppError::Validation(format!(
                "Failed to test delay for proxy '{}'",
                name
            )));
        }
        
        let data: DelayResponse = response.json().await?;
        Ok(data.delay)
    }
}
```

#### 4.2.3 改进 App 状态管理

**改进 app.rs**:
```rust
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
    
    /// 刷新数据
    pub async fn refresh_data(&mut self) -> Result<(), AppError> {
        if !self.kernel_running {
            return Err(AppError::KernelNotRunning);
        }
        
        // 并发获取数据
        let (proxies, connections, traffic, memory) = tokio::join!(
            self.api.get_proxies(),
            self.api.get_connections(),
            self.api.get_traffic(),
            self.api.get_memory(),
        );
        
        // 处理结果
        if let Ok(proxies) = proxies {
            self.proxy_groups = proxies;
        }
        
        if let Ok(connections) = connections {
            self.connections = connections;
        }
        
        if let Ok(traffic) = traffic {
            self.traffic = traffic;
        }
        
        if let Ok(memory) = memory {
            self.memory_bytes = memory;
        }
        
        Ok(())
    }
    
    /// 显示错误消息
    pub fn show_error(&mut self, err: AppError) {
        let message = err.to_string();
        let suggestion = match &err {
            AppError::Api(e) if e.is_connect() => {
                Some("💡 Check if Mihomo kernel is running on the correct port")
            }
            AppError::KernelNotRunning => {
                Some("💡 Run 'clashctl start' to start the kernel")
            }
            AppError::KernelAlreadyRunning => {
                Some("💡 Kernel is already running")
            }
            _ => None,
        };
        
        self.error_msg = Some(ErrorMessage {
            message,
            suggestion: suggestion.map(String::from),
        });
    }
}
```

#### 4.2.4 输入验证

**创建验证模块** (tui/src/validation.rs):
```rust
use crate::error::AppError;
use url::Url;

/// 验证 URL 格式
pub fn validate_url(url: &str) -> Result<(), AppError> {
    if url.is_empty() {
        return Err(AppError::Validation("URL cannot be empty".to_string()));
    }
    
    Url::parse(url)
        .map_err(|e| AppError::Validation(format!("Invalid URL: {}", e)))?;
    
    Ok(())
}

/// 验证路径是否存在
pub fn validate_path(path: &str) -> Result<(), AppError> {
    if path.is_empty() {
        return Err(AppError::Validation("Path cannot be empty".to_string()));
    }
    
    if !std::path::Path::new(path).exists() {
        return Err(AppError::Validation(format!("Path does not exist: {}", path)));
    }
    
    Ok(())
}

/// 验证端口号
pub fn validate_port(port: u16) -> Result<(), AppError> {
    if port == 0 {
        return Err(AppError::Validation("Port cannot be 0".to_string()));
    }
    Ok(())
}
```

### 4.3 错误显示改进

#### 4.3.1 统一错误消息格式

**格式规范**:
```
❌ Error: <用户友好的错误消息>
   Caused by: <原始错误（如果有）>
   
💡 Suggestion: <修复建议（如果有）>
```

**示例**:
```
❌ Error: API request failed (status: 500)
   Caused by: Connection refused (os error 111)
   
💡 Suggestion: Check if Mihomo kernel is running on the correct port
```

#### 4.3.2 实现错误显示

**Go 实现** (cmd/clashctl/utils.go):
```go
// printError 打印格式化的错误消息
func printError(err error) {
    var clashErr *errors.ClashError
    
    fmt.Fprintf(os.Stderr, "\n❌ Error: %v\n", err)
    
    if errors.As(err, &clashErr) {
        if clashErr.Err != nil {
            fmt.Fprintf(os.Stderr, "   Caused by: %v\n", clashErr.Err)
        }
        
        // 根据错误码提供建议
        switch clashErr.Code {
        case errors.ErrCodeKernelNotRunning:
            fmt.Fprintf(os.Stderr, "\n💡 Suggestion: Run 'clashctl start' to start the kernel\n")
        case errors.ErrCodePermissionDenied:
            fmt.Fprintf(os.Stderr, "\n💡 Suggestion: Try running with sudo\n")
        case errors.ErrCodeConfigNotFound:
            fmt.Fprintf(os.Stderr, "\n💡 Suggestion: Run 'clashctl sub add <url>' to add a subscription\n")
        }
    }
    
    fmt.Fprintln(os.Stderr)
}
```

**Rust 实现** (tui/src/app.rs):
```rust
impl App {
    fn render_error(&self, frame: &mut Frame, error: &ErrorMessage) {
        let area = frame.size();
        
        // 计算错误消息框大小
        let width = 60.min(area.width - 4);
        let height = 6;
        let x = (area.width - width) / 2;
        let y = (area.height - height) / 2;
        
        let error_area = Rect::new(x, y, width, height);
        
        // 创建错误消息内容
        let mut lines = vec![
            Line::from(vec![
                Span::styled("❌ Error: ", Style::default().fg(Color::Red)),
                Span::raw(&error.message),
            ]),
        ];
        
        if let Some(suggestion) = &error.suggestion {
            lines.push(Line::from(vec![
                Span::styled("💡 ", Style::default().fg(Color::Yellow)),
                Span::raw(suggestion),
            ]));
        }
        
        // 渲染错误框
        let block = Block::default()
            .title(" Error ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red));
        
        let paragraph = Paragraph::new(lines)
            .block(block)
            .alignment(ratatui::layout::Alignment::Left);
        
        frame.render_widget(Clear, error_area);
        frame.render_widget(paragraph, error_area);
    }
}
```

### 4.4 改进步骤

#### 阶段 1: Go 错误处理改进（2天）
1. 创建 `internal/errors/errors.go`
2. 定义错误类型和预定义错误
3. 修改 `cmd/clashctl/start.go`，替换 `os.Exit(1)`
4. 修改 `cmd/clashctl/tui.go`，替换 `os.Exit(1)`
5. 更新 `cmd/clashctl/main.go`，改进错误显示
6. 添加输入验证函数
7. 测试所有修改

#### 阶段 2: Rust 错误处理改进（2天）
1. 创建 `tui/src/error.rs`
2. 定义 AppError 和 ConfigError
3. 修改 `tui/src/api.rs`，使用 Result 返回
4. 修改 `tui/src/app.rs`，使用 Result 返回
5. 添加输入验证函数
6. 改进错误显示
7. 测试所有修改

#### 阶段 3: 错误显示优化（1天）
1. 统一错误消息格式
2. 添加修复建议
3. 改进错误框样式
4. 测试错误显示

---

## 5. 实施路线图

### 5.1 总体时间线

| 阶段 | 任务 | 时间 | 依赖 |
|------|------|------|------|
| 阶段 1 | 测试基础设施搭建 | 2天 | 无 |
| 阶段 2 | Go 测试编写 | 3天 | 阶段 1 |
| 阶段 3 | Rust 测试编写 | 3天 | 阶段 1 |
| 阶段 4 | app.rs 重构 | 5天 | 阶段 3 |
| 阶段 5 | 错误处理改进 | 4天 | 阶段 2, 3 |
| 阶段 6 | 集成测试 | 2天 | 阶段 4, 5 |
| 阶段 7 | 文档更新 | 1天 | 阶段 6 |

**总计**: 20 天

### 5.2 详细任务分解

#### 阶段 1: 测试基础设施搭建（2天）

**任务 1.1: Go 测试基础设施**
- 创建测试目录结构
- 配置测试工具（go test, go vet）
- 创建 mock 服务器工具
- 准备测试数据

**任务 1.2: Rust 测试基础设施**
- 更新 Cargo.toml 添加测试依赖
- 创建测试目录结构
- 配置测试工具（cargo test, clippy）
- 创建 mock 工具

#### 阶段 2: Go 测试编写（3天）

**任务 2.1: API 客户端测试**
- 编写 api_test.go
- 测试所有 API 方法
- 达到 80% 覆盖率

**任务 2.2: 配置管理测试**
- 编写 env_test.go
- 编写 profiles_test.go
- 编写 merge_test.go
- 达到 70% 覆盖率

**任务 2.3: 订阅下载测试**
- 编写 download_test.go
- 测试各种场景
- 达到 70% 覆盖率

#### 阶段 3: Rust 测试编写（3天）

**任务 3.1: API 客户端测试**
- 编写 api_test.rs
- 测试所有 API 方法
- 达到 80% 覆盖率

**任务 3.2: App 状态测试**
- 编写 app_test.rs
- 测试状态管理
- 达到 60% 覆盖率

**任务 3.3: 配置测试**
- 编写 config_test.rs
- 测试配置加载
- 达到 70% 覆盖率

#### 阶段 4: app.rs 重构（5天）

**任务 4.1: 准备工作**
- 创建 tabs 目录
- 定义 Tab trait
- 更新模块结构

**任务 4.2: 迁移标签页**
- 迁移 Logs 标签页（0.5天）
- 迁移 Connections 标签页（0.5天）
- 迁移 Subscriptions 标签页（1天）
- 迁移 Proxies 标签页（1天）
- 迁移 Overview 标签页（1天）

**任务 4.3: 精简 app.rs**
- 删除已迁移代码
- 更新渲染逻辑
- 整理和优化

**任务 4.4: 测试验证**
- 运行所有测试
- 手动测试
- 修复问题

#### 阶段 5: 错误处理改进（4天）

**任务 5.1: Go 错误处理**
- 创建错误类型
- 替换 os.Exit(1)
- 添加输入验证
- 改进错误显示

**任务 5.2: Rust 错误处理**
- 创建错误类型
- 改进 API 客户端
- 改进 App 状态管理
- 添加输入验证

**任务 5.3: 错误显示优化**
- 统一错误格式
- 添加修复建议
- 改进错误框样式

#### 阶段 6: 集成测试（2天）

**任务 6.1: CLI 集成测试**
- 编写 cli_test.sh
- 测试所有命令
- 验证错误处理

**任务 6.2: TUI 集成测试**
- 编写 tui_test.rs
- 测试启动和退出
- 测试标签页切换

**任务 6.3: CI/CD 配置**
- 创建 GitHub Actions 配置
- 配置自动测试
- 配置代码覆盖率

#### 阶段 7: 文档更新（1天）

**任务 7.1: 更新文档**
- 更新 README.md
- 更新 CONTRIBUTING.md
- 更新 CHANGELOG.md
- 更新 API 文档

### 5.3 里程碑

| 里程碑 | 完成日期 | 交付物 |
|--------|----------|--------|
| M1: 测试基础设施 | 第2天 | 测试框架、mock 工具 |
| M2: Go 测试完成 | 第5天 | Go 测试套件、覆盖率报告 |
| M3: Rust 测试完成 | 第8天 | Rust 测试套件、覆盖率报告 |
| M4: app.rs 重构完成 | 第13天 | 新的模块结构、所有标签页迁移 |
| M5: 错误处理完成 | 第17天 | 统一的错误处理、输入验证 |
| M6: 集成测试完成 | 第19天 | 集成测试套件、CI/CD 配置 |
| M7: 项目完成 | 第20天 | 所有文档更新、发布准备 |

### 5.4 风险管理

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|----------|
| 测试发现隐藏 bug | 中 | 高 | 预留修复时间，优先修复关键 bug |
| 重构引入回归 | 中 | 高 | 充分测试，逐步迁移 |
| 时间超支 | 中 | 中 | 优先完成核心功能，非核心功能可延后 |
| 依赖版本不兼容 | 低 | 中 | 提前测试依赖兼容性 |

---

## 6. 风险和缓解

### 6.1 技术风险

#### 6.1.1 测试发现隐藏 bug

**风险**: 测试过程中可能发现之前未发现的 bug

**影响**: 高 - 可能需要额外时间修复

**缓解措施**:
1. 预留 20% 的时间用于 bug 修复
2. 优先修复关键功能 bug
3. 非关键 bug 可以记录为 issue，延后修复

#### 6.1.2 重构引入回归

**风险**: app.rs 重构可能引入新的 bug

**影响**: 高 - 影响现有功能

**缓解措施**:
1. 充分测试每个迁移的标签页
2. 逐步迁移，每次只迁移一个标签页
3. 保持 Git 提交历史清晰，便于回滚
4. 迁移前后运行完整测试套件

#### 6.1.3 依赖版本不兼容

**风险**: 更新依赖可能导致编译或运行时错误

**影响**: 中 - 可能需要调试和修复

**缓解措施**:
1. 提前测试依赖兼容性
2. 使用语义化版本控制
3. 锁定依赖版本（Cargo.lock, go.sum）

### 6.2 项目风险

#### 6.2.1 时间超支

**风险**: 实际开发时间可能超过预期

**影响**: 中 - 延后项目完成时间

**缓解措施**:
1. 优先完成核心功能（测试、重构）
2. 非核心功能（文档、CI/CD）可延后
3. 每周检查进度，及时调整计划
4. 必要时缩减范围

#### 6.2.2 需求变更

**风险**: 开发过程中需求可能变更

**影响**: 中 - 需要重新设计和实现

**缓解措施**:
1. 明确需求范围，避免范围蔓延
2. 定期与用户沟通，确认需求
3. 设计时考虑扩展性

### 6.3 缓解策略总结

1. **渐进式改进**: 不一次性重构所有代码
2. **充分测试**: 每个改动都要有测试覆盖
3. **版本控制**: 保持清晰的 Git 历史
4. **定期检查**: 每周检查进度和风险
5. **灵活调整**: 根据实际情况调整计划

---

## 附录

### A. 术语表

| 术语 | 定义 |
|------|------|
| 单元测试 | 测试独立函数或方法的测试 |
| 集成测试 | 测试多个组件协同工作的测试 |
| 测试覆盖率 | 被测试代码占总代码的比例 |
| Mock | 模拟外部依赖的测试替身 |
| 回归 | 之前工作的功能现在不工作了 |
| 重构 | 改进代码结构而不改变其行为 |

### B. 参考资料

1. [Go Testing Package](https://pkg.go.dev/testing)
2. [Rust Testing Guide](https://doc.rust-lang.org/book/ch11-00-testing.html)
3. [Ratatui Documentation](https://docs.rs/ratatui/)
4. [Cobra Documentation](https://cobra.dev/)
5. [Mihomo API Documentation](https://wiki.metacubex.one/api/)

### C. 变更历史

| 日期 | 版本 | 变更 |
|------|------|------|
| 2026-06-30 | 1.0 | 初始设计文档 |
