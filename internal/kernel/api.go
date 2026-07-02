package kernel

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strings"
	"time"

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
		client:  &http.Client{Timeout: 8 * time.Second},
	}
}

func (c *Client) do(method, path string, body io.Reader) (*http.Response, error) {
	reqURL := strings.TrimRight(c.BaseURL, "/") + path
	req, err := http.NewRequest(method, reqURL, body)
	if err != nil {
		return nil, err
	}
	if c.Secret != "" {
		req.Header.Set("Authorization", "Bearer "+c.Secret)
	}
	return c.client.Do(req)
}

func responseError(resp *http.Response) error {
	body, _ := io.ReadAll(io.LimitReader(resp.Body, 4096))
	msg := strings.TrimSpace(string(body))
	if msg == "" {
		return fmt.Errorf("HTTP %d", resp.StatusCode)
	}
	return fmt.Errorf("HTTP %d: %s", resp.StatusCode, msg)
}

func (c *Client) HealthCheck() error {
	resp, err := c.do("GET", "/", nil)
	if err != nil {
		return fmt.Errorf("health check: %w", err)
	}
	defer resp.Body.Close()
	if resp.StatusCode != 200 {
		return responseError(resp)
	}
	return nil
}

func (c *Client) GetProxies() (map[string]any, error) {
	resp, err := c.do("GET", "/proxies", nil)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return nil, responseError(resp)
	}
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
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return "", responseError(resp)
	}
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
		path += "?channel=" + url.QueryEscape(channel)
	}
	resp, err := c.do("POST", path, nil)
	if err != nil {
		return fmt.Errorf("upgrade request: %w", err)
	}
	defer resp.Body.Close()
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return responseError(resp)
	}
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
	path := fmt.Sprintf("/proxies/%s/delay?url=%s&timeout=%d",
		url.PathEscape(proxyName),
		url.QueryEscape(testURL),
		timeout,
	)
	resp, err := c.do("GET", path, nil)
	if err != nil {
		return 0, err
	}
	defer resp.Body.Close()
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return 0, responseError(resp)
	}
	var result struct {
		Delay int `json:"delay"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return 0, err
	}
	return result.Delay, nil
}

func (c *Client) SwitchProxy(group, proxy string) error {
	path := fmt.Sprintf("/proxies/%s", url.PathEscape(group))
	body, err := json.Marshal(struct {
		Name string `json:"name"`
	}{Name: proxy})
	if err != nil {
		return err
	}
	reqURL := strings.TrimRight(c.BaseURL, "/") + path
	req, err := http.NewRequest("PUT", reqURL, bytes.NewReader(body))
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
		return responseError(resp)
	}
	return nil
}
