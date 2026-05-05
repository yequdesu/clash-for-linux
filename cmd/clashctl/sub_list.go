package main

import (
	"fmt"

	"github.com/spf13/cobra"

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
			ilog.Warn("cannot load profiles")
			return
		}
		if len(meta.Profiles) == 0 {
			ilog.Info("no subscriptions")
			return
		}
		for _, p := range meta.Profiles {
			marker := " "
			if p.ID == meta.Use {
				marker = "*"
			}
			fmt.Printf(" %s [%d] %s\n", marker, p.ID, p.URL)
		}
	},
}
