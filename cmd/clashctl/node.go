package main

import (
	"fmt"
	"net/url"
	"strings"

	"github.com/spf13/cobra"
	"github.com/yequdesu/clashctl/internal/config"
	"github.com/yequdesu/clashctl/internal/kernel"
)

var nodeCmd = &cobra.Command{
	Use:   "node",
	Short: "Proxy node management",
	Long:  "List proxy groups, switch nodes, and test delays.",
}

var nodeListCmd = &cobra.Command{
	Use:   "list",
	Short: "List all proxy groups and nodes",
	Long:  "List all proxy groups and their nodes with current selection.",
	Run:   runNodeList,
}

var nodeSwitchCmd = &cobra.Command{
	Use:   "switch <group> <node>",
	Short: "Switch proxy node in a group",
	Long:  "Switch the selected proxy node for a group.",
	Args:  cobra.ExactArgs(2),
	Run:   runNodeSwitch,
}

var nodeDelayCmd = &cobra.Command{
	Use:   "delay [group]",
	Short: "Test proxy node delays",
	Long:  "Test latency of proxy nodes in a group or all groups.",
	Run:   runNodeDelay,
}

var (
	nodeGroup  string
	nodeFilter string
	delayURL   string
	delayTimeout int
)

func init() {
	nodeListCmd.Flags().StringVar(&nodeGroup, "group", "", "Only show specified group")
	nodeListCmd.Flags().StringVar(&nodeFilter, "filter", "", "Filter by name")
	nodeDelayCmd.Flags().StringVar(&delayURL, "url", "http://www.gstatic.com/generate_204", "Test URL")
	nodeDelayCmd.Flags().IntVar(&delayTimeout, "timeout", 5000, "Timeout in ms")

	nodeCmd.AddCommand(nodeListCmd)
	nodeCmd.AddCommand(nodeSwitchCmd)
	nodeCmd.AddCommand(nodeDelayCmd)
	rootCmd.AddCommand(nodeCmd)
}

func runNodeList(cmd *cobra.Command, args []string) {
	envConfig, _ := config.LoadEnv(getEnvFilePath())
	port := envConfig.GetPort()

	apiClient := kernel.NewAPIClient(fmt.Sprintf("http://127.0.0.1:%d", port), readSecret())

	proxies, err := apiClient.GetProxies()
	if err != nil {
		fmt.Printf("%s Failed to get proxies: %v\n", red("✗"), err)
		return
	}

	allGroups := proxies.GetGroups()

	separator := strings.Repeat("─", 65)
	fmt.Printf("┌%s┐\n", separator)
	fmt.Printf("│ %-4s %-35s %-10s %-10s %-6s │\n", "♯", "Proxy Node", "Type", "Delay", "Strategy")
	fmt.Printf("├%s┤\n", separator)

	for _, g := range allGroups {
		if nodeGroup != "" && g.Name != nodeGroup {
			continue
		}

		fmt.Printf("│ [♯ %s]\n", bold(cutStr(g.Name, 50)))

		for _, pn := range g.Proxies {
			if nodeFilter != "" {
				lowerPN := strings.ToLower(pn.Name)
				lowerFilter := strings.ToLower(nodeFilter)
				if !strings.Contains(lowerPN, lowerFilter) {
					continue
				}
			}

			marker := "○"
			if pn.Name == g.Now {
				marker = green("●")
			}

			delayStr := formatDelayDisplay(pn.Delay)
			typeStr := cutStr(pn.Type, 8)

			fmt.Printf("│   %s %-33s %-10s %-10s %-6s │\n",
				marker, cutStr(pn.Name, 32), typeStr, delayStr, "")
		}
	}
	fmt.Printf("└%s┘\n", separator)
	fmt.Printf("  %s = current selected  %s = available\n", green("●"), gray("○"))
}

func runNodeSwitch(cmd *cobra.Command, args []string) {
	group := args[0]
	node := args[1]
	// support URL encoded group name
	if decoded, err := url.QueryUnescape(group); err == nil {
		group = decoded
	}
	if decoded, err := url.QueryUnescape(node); err == nil {
		node = decoded
	}

	envConfig, _ := config.LoadEnv(getEnvFilePath())
	port := envConfig.GetPort()

	apiClient := kernel.NewAPIClient(fmt.Sprintf("http://127.0.0.1:%d", port), readSecret())

	err := apiClient.SwitchProxy(group, node)
	if err != nil {
		fmt.Printf("%s Switch failed: %v\n", red("✗"), err)
		return
	}

	// Test delay
	delay, err := apiClient.TestDelay(node, delayURL, delayTimeout)
	if err != nil {
		fmt.Printf("%s %s → %s\n", green("✓"), cyan(group), bold(node))
	} else {
		fmt.Printf("%s %s → %s  (delay: %s)\n", green("✓"), cyan(group), bold(node), cyan(fmt.Sprintf("%dms", delay)))
	}
}

func runNodeDelay(cmd *cobra.Command, args []string) {
	groupName := ""
	if len(args) > 0 {
		groupName = args[0]
	}

	envConfig, _ := config.LoadEnv(getEnvFilePath())
	port := envConfig.GetPort()

	apiClient := kernel.NewAPIClient(fmt.Sprintf("http://127.0.0.1:%d", port), readSecret())

	proxies, err := apiClient.GetProxies()
	if err != nil {
		fmt.Printf("%s Failed to get proxies: %v\n", red("✗"), err)
		return
	}

	allGroups := proxies.GetGroups()

	for _, g := range allGroups {
		if groupName != "" && g.Name != groupName {
			continue
		}

		fmt.Printf("\n[♯ %s]\n", bold(g.Name))
		fmt.Println(strings.Repeat("─", 50))

		for _, pn := range g.Proxies {
			if pn.Type == "Direct" || pn.Type == "Reject" || pn.Type == "Compatible" {
				fmt.Printf("  %-30s %-10s %s\n", cutStr(pn.Name, 29), pn.Type, gray("—"))
				continue
			}

			fmt.Printf("  %-30s ", cutStr(pn.Name, 29))
			delay, err := apiClient.TestDelay(pn.Name, delayURL, delayTimeout)
			if err != nil {
				fmt.Printf("%s %s\n", red("✗"), err.Error())
			} else {
				bar := delayBar(delay)
				fmt.Printf("%s %s\n", cyan(fmt.Sprintf("%dms", delay)), bar)
			}
		}
	}
}

func formatDelayDisplay(delay int) string {
	if delay == 0 {
		return gray("—")
	}
	if delay > 0 {
		return fmt.Sprintf("%dms", delay)
	}
	return gray("—")
}

func delayBar(delay int) string {
	if delay <= 50 {
		return green(strings.Repeat("█", 4) + strings.Repeat("░", 6))
	} else if delay <= 100 {
		return cyan(strings.Repeat("█", 6) + strings.Repeat("░", 4))
	} else if delay <= 200 {
		return yellow(strings.Repeat("█", 8) + strings.Repeat("░", 2))
	}
	return red(strings.Repeat("█", 10))
}

func cutStr(s string, max int) string {
	runes := []rune(s)
	if len(runes) <= max {
		return s
	}
	return string(runes[:max])
}
