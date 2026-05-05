package kernel

import (
	"encoding/json"
	"fmt"
	"net/http"
	"strings"

	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

type Client struct {
	BaseURL string
	Secret  string
	client  *http.Client
}

func NewClient(baseURL, secret string) *Client {
	return &Client{
		BaseURL: baseURL,
		Secret:  secret,
		client:  &http.Client{},
	}
}

func (c *Client) do(method, path string, body any) (*http.Response, error) {
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

func (c *Client) HealthCheck() error {
	resp, err := c.do("GET", "/", nil)
	if err != nil {
		return fmt.Errorf("health check: %w", err)
	}
	defer resp.Body.Close()
	if resp.StatusCode != 200 {
		return fmt.Errorf("kernel returned status %d", resp.StatusCode)
	}
	return nil
}

func (c *Client) GetProxies() (map[string]any, error) {
	resp, err := c.do("GET", "/proxies", nil)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	var result map[string]any
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return nil, err
	}
	return result, nil
}

func (c *Client) GetVersion() (string, error) {
	resp, err := c.do("GET", "/version", nil)
	if err != nil {
		return "", err
	}
	defer resp.Body.Close()
	var v struct {
		Version string `json:"version"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&v); err != nil {
		return "", err
	}
	return v.Version, nil
}

func (c *Client) Upgrade(channel string) error {
	path := "/upgrade"
	if channel != "" {
		path += "?channel=" + channel
	}
	resp, err := c.do("POST", path, nil)
	if err != nil {
		return fmt.Errorf("upgrade request: %w", err)
	}
	defer resp.Body.Close()
	var result struct {
		Status string `json:"status"`
		Error  string `json:"error"`
	}
	json.NewDecoder(resp.Body).Decode(&result)
	if result.Status != "ok" {
		if strings.Contains(result.Error, "already using latest") {
			ilog.Info("already running latest version")
			return nil
		}
		return fmt.Errorf("upgrade failed: %s", result.Error)
	}
	ilog.Ok("kernel upgraded successfully")
	return nil
}

func (c *Client) TestDelay(proxyName string, testURL string, timeout int) (int, error) {
	path := fmt.Sprintf("/proxies/%s/delay?url=%s&timeout=%d", proxyName, testURL, timeout)
	resp, err := c.do("GET", path, nil)
	if err != nil {
		return 0, err
	}
	defer resp.Body.Close()
	var result struct {
		Delay int `json:"delay"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return 0, err
	}
	return result.Delay, nil
}

func (c *Client) SwitchProxy(group, proxy string) error {
	path := fmt.Sprintf("/proxies/%s", group)
	body := strings.NewReader(fmt.Sprintf(`{"name":"%s"}`, proxy))
	url := strings.TrimRight(c.BaseURL, "/") + path
	req, err := http.NewRequest("PUT", url, body)
	if err != nil {
		return err
	}
	req.Header.Set("Content-Type", "application/json")
	if c.Secret != "" {
		req.Header.Set("Authorization", "Bearer "+c.Secret)
	}
	resp, err := c.client.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	if resp.StatusCode != 204 {
		return fmt.Errorf("switch proxy returned %d", resp.StatusCode)
	}
	return nil
}
