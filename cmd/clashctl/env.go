package main

import (
	"fmt"

	"github.com/spf13/cobra"
	"github.com/yequdesu/clashctl/internal/config"
)

var envCmd = &cobra.Command{
	Use:   "env",
	Short: "Output proxy environment variables for eval",
	Long: `Print proxy environment variables for use with eval.

Example:
  eval $(clashctl env)`,
	Run: runEnv,
}

func init() {
	rootCmd.AddCommand(envCmd)
}

func runEnv(cmd *cobra.Command, args []string) {
	envConfig, err := config.LoadEnv(getEnvFilePath())
	if err != nil {
		envConfig = config.DefaultEnvConfig()
	}

	port := envConfig.MixedPort

	fmt.Printf("export http_proxy=http://127.0.0.1:%d\n", port)
	fmt.Printf("export https_proxy=http://127.0.0.1:%d\n", port)
	fmt.Printf("export all_proxy=socks5h://127.0.0.1:%d\n", port)
	fmt.Printf("export HTTP_PROXY=http://127.0.0.1:%d\n", port)
	fmt.Printf("export HTTPS_PROXY=http://127.0.0.1:%d\n", port)
	fmt.Printf("export ALL_PROXY=socks5h://127.0.0.1:%d\n", port)
	fmt.Printf("export no_proxy=localhost,127.0.0.0/8,::1\n")
	fmt.Printf("export NO_PROXY=localhost,127.0.0.0/8,::1\n")
}
