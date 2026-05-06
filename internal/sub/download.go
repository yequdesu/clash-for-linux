package sub

import (
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"time"
)

// SubscriptionInfo holds subscription user info from headers
type SubscriptionInfo struct {
	Upload   int64
	Download int64
	Total    int64
	Expire   int64
}

// DownloadResult contains the result of a subscription download
type DownloadResult struct {
	Name       string
	ProxyCount int
	Interval   int
	Info       *SubscriptionInfo
}

// DownloadSubscription downloads a subscription config from a URL
func DownloadSubscription(url string, targetPath string) (*DownloadResult, error) {
	client := &http.Client{
		Timeout: 30 * time.Second,
	}

	req, err := http.NewRequest("GET", url, nil)
	if err != nil {
		return nil, fmt.Errorf("failed to create request: %v", err)
	}

	req.Header.Set("User-Agent", "ClashTerminal/0.1.0")
	req.Header.Set("Accept", "application/yaml, text/yaml, */*")

	resp, err := client.Do(req)
	if err != nil {
		return nil, fmt.Errorf("download failed: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("HTTP %d: %s", resp.StatusCode, resp.Status)
	}

	data, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("failed to read response: %v", err)
	}

	result := &DownloadResult{}

	// Parse subscription user info from headers
	result.Info = parseSubscriptionUserInfo(resp.Header.Get("subscription-userinfo"))

	// Parse update interval
	if interval := resp.Header.Get("profile-update-interval"); interval != "" {
		if v, err := strconv.Atoi(interval); err == nil {
			result.Interval = v
		}
	}

	// Parse content-disposition for filename
	if cd := resp.Header.Get("content-disposition"); cd != "" {
		result.Name = parseFilename(cd)
	}

	// Count proxies (simple count of "name:" entries)
	result.ProxyCount = strings.Count(string(data), "name:")

	// Save to target
	os.MkdirAll(filepath.Dir(targetPath), 0755)
	if err := os.WriteFile(targetPath, data, 0644); err != nil {
		return nil, fmt.Errorf("failed to save config: %v", err)
	}

	return result, nil
}

// parseSubscriptionUserInfo parses the subscription-userinfo header
func parseSubscriptionUserInfo(header string) *SubscriptionInfo {
	if header == "" {
		return nil
	}

	info := &SubscriptionInfo{}
	parts := strings.Split(header, ";")
	for _, part := range parts {
		part = strings.TrimSpace(part)
		kv := strings.SplitN(part, "=", 2)
		if len(kv) != 2 {
			continue
		}
		val, err := strconv.ParseInt(strings.TrimSpace(kv[1]), 10, 64)
		if err != nil {
			continue
		}
		switch strings.TrimSpace(kv[0]) {
		case "upload":
			info.Upload = val
		case "download":
			info.Download = val
		case "total":
			info.Total = val
		case "expire":
			info.Expire = val
		}
	}
	return info
}

// parseFilename extracts filename from content-disposition header
func parseFilename(cd string) string {
	if idx := strings.Index(cd, "filename="); idx >= 0 {
		name := cd[idx+9:]
		name = strings.Trim(name, "\"'; ")
		if dotIdx := strings.LastIndex(name, "."); dotIdx > 0 {
			name = name[:dotIdx]
		}
		return name
	}
	return ""
}
