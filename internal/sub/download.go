package sub

import (
	"fmt"
	"io"
	"net/http"
	"net/url"
	"os"
	"path/filepath"
	"strings"
	"time"

	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

const (
	ProxyModeSystem = "system"
	ProxyModeDirect = "direct"
	ProxyModeCore   = "core"
)

type DownloadOptions struct {
	ProxyMode string
	ProxyURL  string
}

func Download(url, dest, userAgent string) error {
	return DownloadWithOptions(url, dest, userAgent, DownloadOptions{ProxyMode: ProxyModeSystem})
}

func DownloadWithOptions(url, dest, userAgent string, opts DownloadOptions) error {
	url = strings.TrimSpace(url)

	if strings.HasPrefix(url, "file://") {
		return copyFile(strings.TrimPrefix(url, "file://"), dest)
	}
	if !strings.HasPrefix(url, "http://") && !strings.HasPrefix(url, "https://") {
		if strings.HasPrefix(url, "~/") {
			home, _ := os.UserHomeDir()
			url = filepath.Join(home, url[2:])
		}
		if fi, err := os.Stat(url); err == nil && !fi.IsDir() {
			return copyFile(url, dest)
		}
	}

	ilog.Info("downloading...")
	return httpDownloadWithOptions(url, dest, userAgent, opts)
}

func copyFile(src, dst string) error {
	s, err := os.Open(src)
	if err != nil {
		return fmt.Errorf("open source: %w", err)
	}
	defer s.Close()
	d, err := os.Create(dst)
	if err != nil {
		return fmt.Errorf("create dest: %w", err)
	}
	defer d.Close()
	if _, err := io.Copy(d, s); err != nil {
		return fmt.Errorf("copy: %w", err)
	}
	return nil
}

func httpDownload(url, dest, userAgent string) error {
	return httpDownloadWithOptions(url, dest, userAgent, DownloadOptions{ProxyMode: ProxyModeSystem})
}

func httpDownloadWithOptions(rawURL, dest, userAgent string, opts DownloadOptions) error {
	client, err := httpClientForDownload(opts)
	if err != nil {
		return err
	}
	req, err := http.NewRequest("GET", rawURL, nil)
	if err != nil {
		return fmt.Errorf("request: %w", err)
	}
	if userAgent != "" {
		req.Header.Set("User-Agent", userAgent)
	}
	resp, err := client.Do(req)
	if err != nil {
		return fmt.Errorf("download: %w", err)
	}
	defer resp.Body.Close()
	if resp.StatusCode != 200 {
		return fmt.Errorf("HTTP %d", resp.StatusCode)
	}
	f, err := os.Create(dest)
	if err != nil {
		return fmt.Errorf("create file: %w", err)
	}
	defer f.Close()
	if _, err := io.Copy(f, resp.Body); err != nil {
		return fmt.Errorf("write: %w", err)
	}
	return nil
}

func httpClientForDownload(opts DownloadOptions) (*http.Client, error) {
	mode := strings.ToLower(strings.TrimSpace(opts.ProxyMode))
	if mode == "" {
		mode = ProxyModeSystem
	}
	transport := http.DefaultTransport.(*http.Transport).Clone()
	switch mode {
	case ProxyModeSystem:
		transport.Proxy = http.ProxyFromEnvironment
	case ProxyModeDirect:
		transport.Proxy = nil
	case ProxyModeCore:
		proxyURL, err := url.Parse(strings.TrimSpace(opts.ProxyURL))
		if err != nil || proxyURL.Scheme == "" || proxyURL.Host == "" {
			return nil, fmt.Errorf("invalid core proxy URL: %s", opts.ProxyURL)
		}
		transport.Proxy = http.ProxyURL(proxyURL)
	default:
		return nil, fmt.Errorf("unsupported download proxy mode: %s", opts.ProxyMode)
	}
	return &http.Client{Timeout: 15 * time.Second, Transport: transport}, nil
}
