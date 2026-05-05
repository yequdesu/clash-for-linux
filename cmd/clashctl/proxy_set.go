package main

import (
	"fmt"
	"os"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

func setSystemProxy(cfg *config.EnvConfig) {
	info := readRuntimeInfo(cfg)
	port := info.proxyPort
	if port == "" {
		port = "7890"
	}
	addr := fmt.Sprintf("http://127.0.0.1:%s", port)
	os.Setenv("http_proxy", addr)
	os.Setenv("HTTP_PROXY", addr)
	os.Setenv("https_proxy", addr)
	os.Setenv("HTTPS_PROXY", addr)
	os.Setenv("all_proxy", fmt.Sprintf("socks5h://127.0.0.1:%s", port))
	os.Setenv("ALL_PROXY", fmt.Sprintf("socks5h://127.0.0.1:%s", port))
	os.Setenv("no_proxy", "localhost,127.0.0.1,::1")
	os.Setenv("NO_PROXY", "localhost,127.0.0.1,::1")
}

func unsetSystemProxy() {
	for _, k := range []string{"http_proxy", "HTTP_PROXY", "https_proxy", "HTTPS_PROXY", "all_proxy", "ALL_PROXY", "no_proxy", "NO_PROXY"} {
		os.Unsetenv(k)
	}
}

func showProxyStatus() {
	hp := os.Getenv("http_proxy")
	if hp != "" {
		ilog.Ok("system proxy: on")
		fmt.Printf("  http_proxy=%s\n", hp)
		fmt.Printf("  https_proxy=%s\n", os.Getenv("https_proxy"))
		fmt.Printf("  all_proxy=%s\n", os.Getenv("all_proxy"))
	} else {
		ilog.Warn("system proxy: off")
	}
}
