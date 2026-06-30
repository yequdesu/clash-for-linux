package main

import (
	"fmt"
	"os"
	"path/filepath"

	"github.com/spf13/cobra"
	"github.com/yequdesu/clashctl/internal/config"
)

var proxyCmd = &cobra.Command{
	Use:   "proxy",
	Short: "System proxy switch",
	Long:  "Enable or disable system proxy environment variables.",
}

var proxyOnCmd = &cobra.Command{
	Use:   "on",
	Short: "Enable system proxy",
	Long:  "Set system proxy environment variables in current shell and shell RC files.",
	Run:   runProxyOn,
}

var proxyOffCmd = &cobra.Command{
	Use:   "off",
	Short: "Disable system proxy",
	Long:  "Clear system proxy environment variables.",
	Run:   runProxyOff,
}

var proxyStatusCmd = &cobra.Command{
	Use:   "status",
	Short: "Show system proxy status",
	Long:  "Show current system proxy settings.",
	Run:   runProxyStatus,
}

func init() {
	proxyCmd.AddCommand(proxyOnCmd)
	proxyCmd.AddCommand(proxyOffCmd)
	proxyCmd.AddCommand(proxyStatusCmd)
	rootCmd.AddCommand(proxyCmd)
}

func runProxyOn(cmd *cobra.Command, args []string) {
	envConfig, err := config.LoadEnv(getEnvFilePath())
	if err != nil {
		fmt.Printf("%s Failed to load config: %v\n", red("✗"), err)
		return
	}

	port := envConfig.MixedPort

	// Inject into shell RC files for persistence
	home, err := os.UserHomeDir()
	modified := 0
	if err == nil {
		for _, rc := range []string{".bashrc", ".zshrc"} {
			rcPath := filepath.Join(home, rc)
			if fileExists(rcPath) {
				if err := config.InjectShellRC(rcPath, port); err == nil {
					modified++
				}
			}
		}
	}

	fmt.Printf("%s System proxy enabled\n", green("✓"))
	fmt.Printf("  http_proxy=%s\n", cyan(fmt.Sprintf("http://127.0.0.1:%d", port)))
	fmt.Printf("  https_proxy=%s\n", cyan(fmt.Sprintf("http://127.0.0.1:%d", port)))
	fmt.Printf("  all_proxy=%s\n", cyan(fmt.Sprintf("socks5h://127.0.0.1:%d", port)))
	fmt.Println()
	if modified > 0 {
		fmt.Printf("%s Proxy added to %d shell RC file(s) (persistent across new shells)\n", green("✓"), modified)
	}
	fmt.Println("  Current shell only: eval $(clashctl env)")
	fmt.Println(yellow("  Remember: proxy only works when kernel is running (clashctl start)"))
}

func runProxyOff(cmd *cobra.Command, args []string) {
	home, _ := os.UserHomeDir()
	for _, rc := range []string{".bashrc", ".zshrc"} {
		rcPath := filepath.Join(home, rc)
		if fileExists(rcPath) {
			config.RemoveShellRC(rcPath)
		}
	}
	fmt.Printf("%s System proxy disabled\n", green("✓"))
}

func runProxyStatus(cmd *cobra.Command, args []string) {
	httpProxy := os.Getenv("http_proxy")
	if httpProxy != "" {
		fmt.Printf("%s System proxy: %s\n", green("●"), cyan(httpProxy))
	} else {
		fmt.Printf("%s System proxy: %s\n", gray("○"), "not set")
	}
}
