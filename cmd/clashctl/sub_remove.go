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
		id, err := parseID(args)
		if err != nil {
			ilog.Fatal("%v", err)
		}
		removed, err := removeSubscription(cfg, id)
		if err != nil {
			ilog.Fatal("%v", err)
		}
		logSub(fmt.Sprintf("removed: [%d] %s", id, removed.URL))
		ilog.Ok("subscription removed: [%d]", id)
	},
}

func removeSubscription(cfg *config.EnvConfig, id int) (config.Profile, error) {
	var removed config.Profile
	if err := config.WithProfilesLock(cfg, func() error {
		meta, err := config.LoadProfiles(cfg.ProfilesMeta())
		if err != nil {
			return err
		}
		p := meta.FindByID(id)
		if p == nil {
			return fmt.Errorf("subscription id %d not found", id)
		}
		if meta.Use == id {
			return fmt.Errorf("cannot remove active subscription, switch first")
		}
		removed = *p
		snapshots, err := snapshotFiles(p.Path)
		if err != nil {
			return fmt.Errorf("snapshot profile: %w", err)
		}
		if err := os.Remove(p.Path); err != nil && !os.IsNotExist(err) {
			return fmt.Errorf("remove profile file: %w", err)
		}
		meta.RemoveByID(id)
		if err := saveProfiles(cfg.ProfilesMeta(), meta); err != nil {
			restoreSnapshots(snapshots)
			return fmt.Errorf("save profiles metadata failed, restored profile file: %w", err)
		}
		return nil
	}); err != nil {
		return config.Profile{}, err
	}
	return removed, nil
}

func parseID(args []string) (int, error) {
	if len(args) == 0 {
		fmt.Print("Enter subscription id: ")
		var sid string
		fmt.Scanln(&sid)
		args = []string{sid}
	}
	id, err := strconv.Atoi(args[0])
	if err != nil || id <= 0 {
		return 0, fmt.Errorf("invalid subscription id")
	}
	return id, nil
}
