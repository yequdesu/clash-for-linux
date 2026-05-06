package kernel

import (
	"encoding/json"
	"fmt"
	"net/http"
	"strings"
	"time"
)

type APIClient struct {
	BaseURL string
	Secret  string
	client  *http.Client
}

func NewAPIClient(baseURL, secret string) *APIClient {
	return &APIClient{
		BaseURL: baseURL,
		Secret:  secret,
		client: &http.Client{
			Timeout: 10 * time.Second,
		},
	}
}

func (c *APIClient) doRequest(method, path string, body interface{}) (*http.Response, error) {
	url := strings.TrimRight(c.BaseURL, "/") + path
	req, err := http.NewRequest(method, url, nil)
	if err != nil {
		return nil, err
	}
	if c.Secret != "" {
		req.Header.Set("Authorization", "Bearer "+c.Secret)
	}
	return c.client.Do(req)
}

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

// ProxyInfo represents a proxy node's information
type ProxyInfo struct {
	Name  string
	Type  string
	Now   string
	Delay int
}

// ProxyGroup represents a proxy group with its nodes
type ProxyGroup struct {
	Name    string
	Type    string
	Now     string
	Proxies []ProxyNode
}

// ProxyNode represents a proxy node within a group
type ProxyNode struct {
	Name  string
	Type  string
	Delay int
	Now   string
}

// ProxiesResponse holds the response from GET /proxies
type ProxiesResponse struct {
	Proxies map[string]ProxyDetail `json:"proxies"`
}

// ProxyDetail holds details of a proxy group or node
type ProxyDetail struct {
	Type    string   `json:"type"`
	Now     string   `json:"now"`
	Name    string   `json:"name"`
	All     []string `json:"all"`
	History []DelayHistory `json:"history"`
}

// DelayHistory holds delay history for a proxy
type DelayHistory struct {
	Time  string `json:"time"`
	Delay int    `json:"delay"`
}

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

// GetGroups extracts proxy groups from the response
func (pr *ProxiesResponse) GetGroups() []ProxyGroup {
	var groups []ProxyGroup
	for name, detail := range pr.Proxies {
		if detail.Type == "Selector" || detail.Type == "URLTest" || detail.Type == "Fallback" || detail.Type == "LoadBalance" {
			group := ProxyGroup{
				Name: name,
				Type: detail.Type,
				Now:  detail.Now,
			}
			for _, proxyName := range detail.All {
				node := ProxyNode{
					Name: proxyName,
				}
				if p, ok := pr.Proxies[proxyName]; ok {
					node.Type = p.Type
					if len(p.History) > 0 {
						node.Delay = p.History[len(p.History)-1].Delay
					}
				}
				group.Proxies = append(group.Proxies, node)
			}
			groups = append(groups, group)
		}
	}
	return groups
}

// SwitchProxy switches the selected proxy in a group
func (c *APIClient) SwitchProxy(group, proxy string) error {
	body := fmt.Sprintf(`{"name":"%s"}`, proxy)
	resp, err := c.doRequest("PUT", "/proxies/"+group, nil)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	// Actually send the PUT with body
	url := strings.TrimRight(c.BaseURL, "/") + "/proxies/" + group
	req, err := http.NewRequest("PUT", url, strings.NewReader(body))
	if err != nil {
		return err
	}
	if c.Secret != "" {
		req.Header.Set("Authorization", "Bearer "+c.Secret)
	}
	req.Header.Set("Content-Type", "application/json")
	resp2, err := c.client.Do(req)
	if err != nil {
		return err
	}
	defer resp2.Body.Close()
	return nil
}

// TestDelay tests the delay of a proxy node
func (c *APIClient) TestDelay(proxyName, testURL string, timeoutMs int) (int, error) {
	url := fmt.Sprintf("%s/proxies/%s/delay?url=%s&timeout=%d",
		strings.TrimRight(c.BaseURL, "/"), proxyName, testURL, timeoutMs)
	req, err := http.NewRequest("GET", url, nil)
	if err != nil {
		return 0, err
	}
	if c.Secret != "" {
		req.Header.Set("Authorization", "Bearer "+c.Secret)
	}

	resp, err := c.client.Do(req)
	if err != nil {
		return 0, err
	}
	defer resp.Body.Close()

	var result struct {
		Delay int `json:"delay"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return 0, fmt.Errorf("failed to parse delay response: %v", err)
	}
	return result.Delay, nil
}

// TrafficInfo holds upload/download traffic data
type TrafficInfo struct {
	Up   uint64 `json:"up"`
	Down uint64 `json:"down"`
}

// GetTraffic fetches current traffic stats
func (c *APIClient) GetTraffic() (*TrafficInfo, error) {
	resp, err := c.doRequest("GET", "/traffic", nil)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	var result TrafficInfo
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return nil, err
	}
	return &result, nil
}

// Connection represents an active or closed connection
type Connection struct {
	ID             string   `json:"id"`
	Host           string   `json:"host"`
	Network        string   `json:"network"`
	Type           string   `json:"type"`
	Chain          []string `json:"chain"`
	DownloadSpeed  uint64   `json:"downloadSpeed"`
	UploadSpeed    uint64   `json:"uploadSpeed"`
	Download       uint64   `json:"download"`
	Upload         uint64   `json:"upload"`
	Start          string   `json:"start"`
	UploadClosed   bool     `json:"uploadClosed"`
	DownloadClosed bool     `json:"downloadClosed"`
}

// GetConnections fetches all connections
func (c *APIClient) GetConnections() ([]Connection, error) {
	resp, err := c.doRequest("GET", "/connections", nil)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	var result struct {
		Connections []Connection `json:"connections"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return nil, err
	}
	return result.Connections, nil
}

// CloseConnection closes a specific connection
func (c *APIClient) CloseConnection(id string) error {
	_, err := c.doRequest("DELETE", "/connections/"+id, nil)
	return err
}

// CloseAllConnections closes all connections
func (c *APIClient) CloseAllConnections() error {
	_, err := c.doRequest("DELETE", "/connections", nil)
	return err
}

// MemoryInfo holds memory usage info
type MemoryInfo struct {
	InUse   uint64 `json:"inuse"`
	OSLimit uint64 `json:"oslimit"`
}

// GetMemory fetches memory stats
func (c *APIClient) GetMemory() (*MemoryInfo, error) {
	resp, err := c.doRequest("GET", "/memory", nil)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	var result MemoryInfo
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return nil, err
	}
	return &result, nil
}

// RuntimeConfig holds runtime config info
type RuntimeConfig struct {
	Mode      string    `json:"mode"`
	MixedPort int       `json:"mixed-port"`
	SocksPort int       `json:"socks-port"`
	Port      int       `json:"port"`
	TUN       *TUNConfig `json:"tun"`
}

// TUNConfig holds TUN settings
type TUNConfig struct {
	Enable bool `json:"enable"`
}

// GetConfig fetches the current config
func (c *APIClient) GetConfig() (*RuntimeConfig, error) {
	resp, err := c.doRequest("GET", "/configs", nil)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	var result RuntimeConfig
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return nil, err
	}
	return &result, nil
}

// GetLogs fetches recent logs
func (c *APIClient) GetLogs() ([]string, error) {
	resp, err := c.doRequest("GET", "/logs", nil)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	var result struct {
		Logs []string `json:"logs"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return nil, err
	}
	return result.Logs, nil
}

// SetMode sets the proxy mode (Rule/Global/Direct)
func (c *APIClient) SetMode(mode string) error {
	body := fmt.Sprintf(`{"mode":"%s"}`, mode)
	url := strings.TrimRight(c.BaseURL, "/") + "/configs"
	req, err := http.NewRequest("PATCH", url, strings.NewReader(body))
	if err != nil {
		return err
	}
	if c.Secret != "" {
		req.Header.Set("Authorization", "Bearer "+c.Secret)
	}
	req.Header.Set("Content-Type", "application/json")
	resp, err := c.client.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	return nil
}
