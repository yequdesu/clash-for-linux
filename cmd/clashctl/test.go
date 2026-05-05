package main

import (
	"fmt"
	"io"
	"net/http"
	"net/url"
	"time"

	"github.com/spf13/cobra"
	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

var testCmd = &cobra.Command{
	Use:   "test [url]",
	Short: "Test proxy connectivity",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		info := readRuntimeInfo(cfg)
		port := info.proxyPort
		if port == "" {
			port = "7890"
		}
		if !portOpen(port) {
			ilog.Fatal("proxy port :%s not listening — run 'clashctl start' first", port)
		}

		proxyURL, _ := url.Parse(fmt.Sprintf("http://127.0.0.1:%s", port))
		client := &http.Client{
			Transport: &http.Transport{Proxy: http.ProxyURL(proxyURL)},
			Timeout:   10 * time.Second,
		}

		ilog.Info("testing proxy...")
		testURL := "http://www.google.com"
		if len(args) > 0 && len(args[0]) > 4 {
			testURL = args[0]
		}

		resp, err := client.Get(testURL)
		if err != nil {
			ilog.Fatal("proxy test failed: %v", err)
		}
		defer resp.Body.Close()

		body, _ := io.ReadAll(io.LimitReader(resp.Body, 500))
		ilog.Ok("proxy test OK (%d) — %.100s", resp.StatusCode, string(body))

		ilog.Info("to use proxy in shell: eval $(clashctl env)")
	},
}
