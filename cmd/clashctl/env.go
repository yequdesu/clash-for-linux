package main

import (
	"fmt"

	"github.com/spf13/cobra"
)

var envCmd = &cobra.Command{
	Use:   "env",
	Short: "Print proxy environment variables (for shell eval)",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		info := readRuntimeInfo(cfg)
		port := info.proxyPort
		if port == "" {
			port = "7890"
		}
		fmt.Printf("export http_proxy=http://127.0.0.1:%s\n", port)
		fmt.Printf("export HTTP_PROXY=http://127.0.0.1:%s\n", port)
		fmt.Printf("export https_proxy=http://127.0.0.1:%s\n", port)
		fmt.Printf("export HTTPS_PROXY=http://127.0.0.1:%s\n", port)
		fmt.Printf("export all_proxy=socks5h://127.0.0.1:%s\n", port)
		fmt.Printf("export ALL_PROXY=socks5h://127.0.0.1:%s\n", port)
		fmt.Printf("export no_proxy=localhost,127.0.0.1,::1\n")
		fmt.Printf("export NO_PROXY=localhost,127.0.0.1,::1\n")
	},
}
