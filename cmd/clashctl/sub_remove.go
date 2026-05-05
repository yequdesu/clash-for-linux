package main

import (
	"fmt"
	"os"
	"strconv"

	"github.com/spf13/cobra"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

var subRemoveCmd = &cobra.Command{
	Use:   "remove <id>",
	Short: "Remove a subscription",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		id := parseID(args)
		if id == 0 {
			return
		}
		meta, _ := config.LoadProfiles(cfg.ProfilesMeta())
		p := meta.FindByID(id)
		if p == nil {
			ilog.Warn("subscription id %d not found", id)
			return
		}
		if meta.Use == id {
			ilog.Warn("cannot remove active subscription, switch first")
			return
		}
		os.Remove(p.Path)
		meta.RemoveByID(id)
		config.SaveProfiles(cfg.ProfilesMeta(), meta)
		logSub(fmt.Sprintf("removed: [%d] %s", id, p.URL))
		ilog.Ok("subscription removed: [%d]", id)
	},
}

func parseID(args []string) int {
	if len(args) == 0 {
		fmt.Print("Enter subscription id: ")
		var sid string
		fmt.Scanln(&sid)
		args = []string{sid}
	}
	id, err := strconv.Atoi(args[0])
	if err != nil || id <= 0 {
		ilog.Warn("invalid subscription id")
		return 0
	}
	return id
}
