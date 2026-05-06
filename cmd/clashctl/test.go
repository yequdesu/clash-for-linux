package main

import (
	"fmt"
	"net/http"
	"time"

	"github.com/spf13/cobra"
	"github.com/yequdesu/clashctl/internal/config"
	"github.com/yequdesu/clashctl/internal/kernel"
)

var testCmd = &cobra.Command{
	Use:   "test",
	Short: "Latency test",
	Long:  "Test URL connectivity or proxy node latency.",
}

var testURLCmd = &cobra.Command{
	Use:   "<url>",
	Short: "Test a URL directly",
	Args:  cobra.ExactArgs(1),
	Run:   runTestURL,
}

var testProxyCmd = &cobra.Command{
	Use:   "proxy <name>",
	Short: "Test via proxy node",
	Args:  cobra.ExactArgs(1),
	Run:   runTestProxy,
}

func init() {
	testCmd.AddCommand(testURLCmd)
	testCmd.AddCommand(testProxyCmd)
	rootCmd.AddCommand(testCmd)
}

func runTestURL(cmd *cobra.Command, args []string) {
	urlStr := args[0]
	start := time.Now()

	client := &http.Client{Timeout: 10 * time.Second}
	resp, err := client.Get(urlStr)
	if err != nil {
		fmt.Printf("%s %s — %s\n", red("✗"), urlStr, err.Error())
		return
	}
	defer resp.Body.Close()

	elapsed := time.Since(start)
	delay := elapsed.Milliseconds()

	if resp.StatusCode >= 200 && resp.StatusCode < 400 {
		fmt.Printf("%s %s — %s\n", green("✓"), cyan(urlStr), cyan(fmt.Sprintf("%dms", delay)))
	} else {
		fmt.Printf("%s %s — %s (HTTP %d)\n", yellow("⚠"), urlStr, cyan(fmt.Sprintf("%dms", delay)), resp.StatusCode)
	}
}

func runTestProxy(cmd *cobra.Command, args []string) {
	nodeName := args[0]
	testURL := "http://www.gstatic.com/generate_204"

	envConfig, _ := config.LoadEnv(getEnvFilePath())
	port := envConfig.GetPort()

	apiClient := kernel.NewAPIClient(fmt.Sprintf("http://127.0.0.1:%d", port), readSecret())

	delay, err := apiClient.TestDelay(nodeName, testURL, 5000)
	if err != nil {
		fmt.Printf("%s %s — %s\n", red("✗"), cyan(nodeName), err.Error())
		return
	}

	fmt.Printf("%s %s — %s\n", green("✓"), cyan(nodeName), cyan(fmt.Sprintf("%dms", delay)))
}
