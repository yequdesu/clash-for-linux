package main

import (
	"fmt"
	"sort"
	"strings"

	"github.com/spf13/cobra"

	"github.com/yequdesu/linux-cli-tui-clash/internal/kernel"
	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

var nodeCmd = &cobra.Command{
	Use:   "node",
	Short: "Proxy node management",
	Long:  "List proxy groups/nodes, switch nodes, test delays.",
}

var nodeListCmd = &cobra.Command{
	Use:   "list",
	Short: "List all proxy groups and nodes",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		info := readRuntimeInfo(cfg)
		apiPort := info.apiPort
		if apiPort == "" {
			apiPort = "9090"
		}
		if !portOpen(apiPort) {
			ilog.Fatal("kernel API not available on :%s — run 'clashctl start' first", apiPort)
		}

		api := kernel.NewClient(fmt.Sprintf("http://127.0.0.1:%s", apiPort), info.secret)
		proxies, err := api.GetProxies()
		if err != nil {
			ilog.Warn("failed to get proxies: %v", err)
			return
		}

		groups := make([]string, 0)
		for name, raw := range proxies {
			if m, ok := raw.(map[string]any); ok {
				t, _ := m["type"].(string)
				if t == "Selector" || t == "Fallback" || t == "URLTest" || t == "LoadBalance" {
					groups = append(groups, name)
				}
			}
		}
		sort.Strings(groups)

		for _, groupName := range groups {
			raw := proxies[groupName]
			m, ok := raw.(map[string]any)
			if !ok {
				continue
			}

			now, _ := m["now"].(string)
			allRaw, _ := m["all"].([]any)

			ilog.Section(fmt.Sprintf("Group: %s", groupName))
			for _, nodeRaw := range allRaw {
				node, _ := nodeRaw.(string)
				marker := " "
				if node == now {
					marker = "*"
				}

				delayStr := "—"
				if delay, err := api.TestDelay(node, "http://www.gstatic.com/generate_204", 3000); err == nil {
					delayStr = fmt.Sprintf("%dms", delay)
				}

				fmt.Printf("  %s %s  (%s)\n", marker, node, delayStr)
			}
			fmt.Println()
		}
	},
}

var nodeSwitchCmd = &cobra.Command{
	Use:   "switch <group> <node>",
	Short: "Switch proxy node in a group",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		if len(args) < 2 {
			ilog.Warn("usage: clashctl node switch <group> <node>")
			return
		}
		info := readRuntimeInfo(cfg)
		apiPort := info.apiPort
		if apiPort == "" {
			apiPort = "9090"
		}
		if !portOpen(apiPort) {
			ilog.Fatal("kernel API not available on :%s", apiPort)
		}

		group := args[0]
		node := args[1]
		api := kernel.NewClient(fmt.Sprintf("http://127.0.0.1:%s", apiPort), info.secret)
		if err := api.SwitchProxy(group, node); err != nil {
			ilog.Warn("switch failed: %v", err)
			return
		}

		delayStr := ""
		if delay, err := api.TestDelay(node, "http://www.gstatic.com/generate_204", 3000); err == nil {
			delayStr = fmt.Sprintf(" (delay: %dms)", delay)
		}
		ilog.Ok("%s → %s%s", group, node, delayStr)
	},
}

var nodeDelayCmd = &cobra.Command{
	Use:   "delay [group]",
	Short: "Test delay of all nodes in a group",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		info := readRuntimeInfo(cfg)
		apiPort := info.apiPort
		if apiPort == "" {
			apiPort = "9090"
		}
		if !portOpen(apiPort) {
			ilog.Fatal("kernel API not available on :%s", apiPort)
		}

		api := kernel.NewClient(fmt.Sprintf("http://127.0.0.1:%s", apiPort), info.secret)
		proxies, err := api.GetProxies()
		if err != nil {
			ilog.Warn("failed to get proxies: %v", err)
			return
		}

		groups := make([]string, 0)
		for name, raw := range proxies {
			if m, ok := raw.(map[string]any); ok {
				t, _ := m["type"].(string)
				if t == "Selector" || t == "Fallback" || t == "URLTest" || t == "LoadBalance" {
					if len(args) == 0 || name == args[0] {
						groups = append(groups, name)
					}
				}
			}
		}

		if len(args) > 0 && len(groups) == 0 {
			ilog.Warn("group '%s' not found", args[0])
			return
		}

		for _, groupName := range groups {
			raw := proxies[groupName]
			m := raw.(map[string]any)
			allRaw, _ := m["all"].([]any)

			ilog.Section(fmt.Sprintf("Delay Test: %s", groupName))
			for _, nodeRaw := range allRaw {
				node, _ := nodeRaw.(string)
				if node == "DIRECT" || node == "REJECT" || node == "REJECT-DROP" {
					fmt.Printf("  %s  (built-in)\n", node)
					continue
				}

				result := "TIMEOUT"
				if delay, err := api.TestDelay(node, "http://www.gstatic.com/generate_204", 5000); err == nil {
					result = fmt.Sprintf("%dms", delay)
				} else if strings.Contains(err.Error(), "timeout") {
					result = "TIMEOUT"
				} else {
					result = "ERROR"
				}
				fmt.Printf("  %s  %s\n", node, result)
			}
			fmt.Println()
		}
	},
}

func init() {
	nodeCmd.AddCommand(nodeListCmd)
	nodeCmd.AddCommand(nodeSwitchCmd)
	nodeCmd.AddCommand(nodeDelayCmd)
}
