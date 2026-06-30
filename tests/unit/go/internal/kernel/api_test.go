package kernel_test

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
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

func TestGetVersionWithMeta(t *testing.T) {
	// 测试 meta 字段
	mockResponse := `{"meta": "v1.19.18"}`

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.Write([]byte(mockResponse))
	}))
	defer server.Close()

	client := kernel.NewAPIClient(server.URL, "")
	version, err := client.GetVersion()

	if err != nil {
		t.Fatalf("Unexpected error: %v", err)
	}

	if version != "v1.19.18" {
		t.Errorf("Expected version 'v1.19.18', got '%s'", version)
	}
}

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

	if proxies == nil {
		t.Fatal("Expected proxies response, got nil")
	}

	if len(proxies.Proxies) != 1 {
		t.Errorf("Expected 1 proxy, got %d", len(proxies.Proxies))
	}

	proxy, exists := proxies.Proxies["Proxy"]
	if !exists {
		t.Fatal("Expected 'Proxy' to exist")
	}

	if proxy.Name != "Proxy" {
		t.Errorf("Expected proxy name 'Proxy', got '%s'", proxy.Name)
	}

	if proxy.Type != "Selector" {
		t.Errorf("Expected proxy type 'Selector', got '%s'", proxy.Type)
	}
}

func TestGetGroups(t *testing.T) {
	// 测试 GetGroups 方法
	proxies := &kernel.ProxiesResponse{
		Proxies: map[string]kernel.ProxyDetail{
			"Proxy": {
				Type: "Selector",
				Now:  "node1",
				Name: "Proxy",
				All:  []string{"node1", "node2"},
			},
			"node1": {
				Type: "ss",
				Name: "node1",
			},
			"node2": {
				Type: "ss",
				Name: "node2",
			},
		},
	}

	groups := proxies.GetGroups()

	if len(groups) != 1 {
		t.Fatalf("Expected 1 group, got %d", len(groups))
	}

	group := groups[0]
	if group.Name != "Proxy" {
		t.Errorf("Expected group name 'Proxy', got '%s'", group.Name)
	}

	if len(group.Proxies) != 2 {
		t.Errorf("Expected 2 proxies in group, got %d", len(group.Proxies))
	}
}

func TestGetConnections(t *testing.T) {
	// 准备 mock 响应
	mockResponse := `{
		"connections": [
			{
				"id": "conn1",
				"host": "example.com",
				"network": "tcp",
				"type": "HTTP",
				"chain": ["Proxy", "node1"],
				"downloadSpeed": 1024,
				"uploadSpeed": 512
			}
		]
	}`

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/connections" {
			t.Errorf("Expected path /connections, got %s", r.URL.Path)
		}
		w.Header().Set("Content-Type", "application/json")
		w.Write([]byte(mockResponse))
	}))
	defer server.Close()

	client := kernel.NewAPIClient(server.URL, "")
	connections, err := client.GetConnections()

	if err != nil {
		t.Fatalf("Unexpected error: %v", err)
	}

	if len(connections) != 1 {
		t.Fatalf("Expected 1 connection, got %d", len(connections))
	}

	conn := connections[0]
	if conn.ID != "conn1" {
		t.Errorf("Expected connection ID 'conn1', got '%s'", conn.ID)
	}

	if conn.Host != "example.com" {
		t.Errorf("Expected host 'example.com', got '%s'", conn.Host)
	}
}

func TestGetTraffic(t *testing.T) {
	// 准备 mock 响应
	mockResponse := `{"up": 1024, "down": 2048}`

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/traffic" {
			t.Errorf("Expected path /traffic, got %s", r.URL.Path)
		}
		w.Header().Set("Content-Type", "application/json")
		w.Write([]byte(mockResponse))
	}))
	defer server.Close()

	client := kernel.NewAPIClient(server.URL, "")
	traffic, err := client.GetTraffic()

	if err != nil {
		t.Fatalf("Unexpected error: %v", err)
	}

	if traffic.Up != 1024 {
		t.Errorf("Expected upload 1024, got %d", traffic.Up)
	}

	if traffic.Down != 2048 {
		t.Errorf("Expected download 2048, got %d", traffic.Down)
	}
}

func TestGetMemory(t *testing.T) {
	// 准备 mock 响应
	mockResponse := `{"inuse": 1048576, "oslimit": 2097152}`

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/memory" {
			t.Errorf("Expected path /memory, got %s", r.URL.Path)
		}
		w.Header().Set("Content-Type", "application/json")
		w.Write([]byte(mockResponse))
	}))
	defer server.Close()

	client := kernel.NewAPIClient(server.URL, "")
	memory, err := client.GetMemory()

	if err != nil {
		t.Fatalf("Unexpected error: %v", err)
	}

	if memory.InUse != 1048576 {
		t.Errorf("Expected inuse 1048576, got %d", memory.InUse)
	}

	if memory.OSLimit != 2097152 {
		t.Errorf("Expected oslimit 2097152, got %d", memory.OSLimit)
	}
}

func TestGetConfig(t *testing.T) {
	// 准备 mock 响应
	mockResponse := `{
		"mode": "Rule",
		"mixed-port": 7890,
		"socks-port": 7891,
		"port": 7892,
		"tun": {"enable": true}
	}`

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/configs" {
			t.Errorf("Expected path /configs, got %s", r.URL.Path)
		}
		w.Header().Set("Content-Type", "application/json")
		w.Write([]byte(mockResponse))
	}))
	defer server.Close()

	client := kernel.NewAPIClient(server.URL, "")
	config, err := client.GetConfig()

	if err != nil {
		t.Fatalf("Unexpected error: %v", err)
	}

	if config.Mode != "Rule" {
		t.Errorf("Expected mode 'Rule', got '%s'", config.Mode)
	}

	if config.MixedPort != 7890 {
		t.Errorf("Expected mixed-port 7890, got %d", config.MixedPort)
	}

	if config.TUN == nil || !config.TUN.Enable {
		t.Error("Expected TUN to be enabled")
	}
}

func TestSwitchProxy(t *testing.T) {
	// 启动 mock 服务器
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "PUT" {
			t.Errorf("Expected PUT method, got %s", r.Method)
		}

		if r.URL.Path != "/proxies/Proxy" {
			t.Errorf("Expected path /proxies/Proxy, got %s", r.URL.Path)
		}

		// 验证请求体
		var body map[string]string
		if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
			t.Fatalf("Failed to decode request body: %v", err)
		}

		if body["name"] != "node1" {
			t.Errorf("Expected proxy name 'node1', got '%s'", body["name"])
		}

		w.WriteHeader(http.StatusNoContent)
	}))
	defer server.Close()

	client := kernel.NewAPIClient(server.URL, "")
	err := client.SwitchProxy("Proxy", "node1")

	if err != nil {
		t.Fatalf("Unexpected error: %v", err)
	}
}

func TestTestDelay(t *testing.T) {
	// 启动 mock 服务器
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/proxies/node1/delay" {
			t.Errorf("Expected path /proxies/node1/delay, got %s", r.URL.Path)
		}

		// 验证查询参数
		query := r.URL.Query()
		if query.Get("url") == "" {
			t.Error("Expected url parameter")
		}

		if query.Get("timeout") == "" {
			t.Error("Expected timeout parameter")
		}

		w.Header().Set("Content-Type", "application/json")
		w.Write([]byte(`{"delay": 100}`))
	}))
	defer server.Close()

	client := kernel.NewAPIClient(server.URL, "")
	delay, err := client.TestDelay("node1", "https://www.google.com", 5000)

	if err != nil {
		t.Fatalf("Unexpected error: %v", err)
	}

	if delay != 100 {
		t.Errorf("Expected delay 100, got %d", delay)
	}
}

func TestSetMode(t *testing.T) {
	// 启动 mock 服务器
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "PATCH" {
			t.Errorf("Expected PATCH method, got %s", r.Method)
		}

		if r.URL.Path != "/configs" {
			t.Errorf("Expected path /configs, got %s", r.URL.Path)
		}

		// 验证请求体
		var body map[string]string
		if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
			t.Fatalf("Failed to decode request body: %v", err)
		}

		if body["mode"] != "Global" {
			t.Errorf("Expected mode 'Global', got '%s'", body["mode"])
		}

		w.WriteHeader(http.StatusNoContent)
	}))
	defer server.Close()

	client := kernel.NewAPIClient(server.URL, "")
	err := client.SetMode("Global")

	if err != nil {
		t.Fatalf("Unexpected error: %v", err)
	}
}

func TestCloseConnection(t *testing.T) {
	// 启动 mock 服务器
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "DELETE" {
			t.Errorf("Expected DELETE method, got %s", r.Method)
		}

		if r.URL.Path != "/connections/conn1" {
			t.Errorf("Expected path /connections/conn1, got %s", r.URL.Path)
		}

		w.WriteHeader(http.StatusNoContent)
	}))
	defer server.Close()

	client := kernel.NewAPIClient(server.URL, "")
	err := client.CloseConnection("conn1")

	if err != nil {
		t.Fatalf("Unexpected error: %v", err)
	}
}

func TestCloseAllConnections(t *testing.T) {
	// 启动 mock 服务器
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "DELETE" {
			t.Errorf("Expected DELETE method, got %s", r.Method)
		}

		if r.URL.Path != "/connections" {
			t.Errorf("Expected path /connections, got %s", r.URL.Path)
		}

		w.WriteHeader(http.StatusNoContent)
	}))
	defer server.Close()

	client := kernel.NewAPIClient(server.URL, "")
	err := client.CloseAllConnections()

	if err != nil {
		t.Fatalf("Unexpected error: %v", err)
	}
}

func TestGetLogs(t *testing.T) {
	// 准备 mock 响应
	mockResponse := `{"logs": ["log1", "log2", "log3"]}`

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/logs" {
			t.Errorf("Expected path /logs, got %s", r.URL.Path)
		}
		w.Header().Set("Content-Type", "application/json")
		w.Write([]byte(mockResponse))
	}))
	defer server.Close()

	client := kernel.NewAPIClient(server.URL, "")
	logs, err := client.GetLogs()

	if err != nil {
		t.Fatalf("Unexpected error: %v", err)
	}

	if len(logs) != 3 {
		t.Errorf("Expected 3 logs, got %d", len(logs))
	}

	if logs[0] != "log1" {
		t.Errorf("Expected first log 'log1', got '%s'", logs[0])
	}
}

func TestAPIError(t *testing.T) {
	// 测试 API 错误响应
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusInternalServerError)
		w.Write([]byte(`{"error": "internal server error"}`))
	}))
	defer server.Close()

	client := kernel.NewAPIClient(server.URL, "")
	_, err := client.GetVersion()

	// 应该返回错误
	if err == nil {
		t.Fatal("Expected error for 500 status code, got nil")
	}
}

func TestAPIConnectionError(t *testing.T) {
	// 测试连接错误
	client := kernel.NewAPIClient("http://localhost:1", "")
	_, err := client.GetVersion()

	// 应该返回错误
	if err == nil {
		t.Fatal("Expected connection error, got nil")
	}
}
