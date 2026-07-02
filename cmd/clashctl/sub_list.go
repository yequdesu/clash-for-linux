package main

import (
	"fmt"
	"os"
	"strings"

	"github.com/spf13/cobra"
	"gopkg.in/yaml.v3"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

var subListCmd = &cobra.Command{
	Use:   "list",
	Short: "List subscriptions",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		meta, err := config.LoadProfiles(cfg.ProfilesMeta())
		if err != nil {
			ilog.Fatal("cannot load profiles: %v", err)
		}
		if len(meta.Profiles) == 0 {
			ilog.Info("no subscriptions")
			return
		}

		fmt.Println()
		printTableHeader([]string{"", "ID", "Name", "Status", "Updated", "Interval", "Next", "Proxies", "URL"})
		for _, p := range meta.Profiles {
			marker := " "
			status := "ready"
			if p.ID == meta.Use {
				marker = "*"
				status = "active"
			}
			if p.LastError != "" {
				status = "error"
			}
			name := p.Name
			if name == "" {
				name = shortenURL(p.URL)
			}
			updated := p.Updated
			if updated == "" {
				updated = "—"
			}
			interval := profileIntervalLabel(p)
			next := p.NextUpdate
			if next == "" {
				next = nextProfileUpdateString(p, timeNow())
			}
			if next == "" {
				next = "—"
			}
			proxies := countProxies(p.Path)
			fmt.Printf(" %s  %-3d %-20s %-8s %-14s %-9s %-14s %-7s %s\n",
				marker, p.ID, truncStr(name, 20), status, truncStr(updated, 14), interval, truncStr(next, 14), proxies, shortenURL(p.URL))
		}
		fmt.Println()
		fmt.Println(" * = currently active")
	},
}

func printTableHeader(columns []string) {
	for i, c := range columns {
		if i == 0 {
			fmt.Printf(" %s ", c)
		} else if i == len(columns)-1 {
			fmt.Printf("%s", c)
		} else {
			fmt.Printf("%-*s ", len(c)+4, c)
		}
	}
	fmt.Println()
	for i, c := range columns {
		w := len(c) + 4
		if i == 0 {
			fmt.Print("──")
		} else if i == len(columns)-1 {
			fmt.Print(strings.Repeat("─", 30))
		} else {
			fmt.Print(strings.Repeat("─", w))
		}
		if i < len(columns)-1 {
			fmt.Print("")
		}
	}
	fmt.Println()
}

func truncStr(s string, max int) string {
	if len(s) > max {
		return s[:max-3] + "..."
	}
	return s
}

func countProxies(path string) string {
	data, err := os.ReadFile(path)
	if err != nil {
		return "—"
	}
	var parsed struct {
		Proxies []yaml.Node `yaml:"proxies"`
	}
	if err := yaml.Unmarshal(data, &parsed); err != nil {
		return "unknown"
	}
	if len(parsed.Proxies) == 0 {
		return "—"
	}
	return fmt.Sprintf("%d", len(parsed.Proxies))
}
