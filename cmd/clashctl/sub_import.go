package main

import (
	"os"
	"path/filepath"
	"strings"

	"github.com/spf13/cobra"

	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

var subImportCmd = &cobra.Command{
	Use:   "import [directory]",
	Short: "Import all .yaml configs from a directory",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		dir := cfg.ConfigsDir()
		if len(args) > 0 {
			dir = args[0]
		}
		if fi, err := os.Stat(dir); err != nil || !fi.IsDir() {
			ilog.Warn("directory not found: %s", dir)
			return
		}
		entries, _ := os.ReadDir(dir)
		count := 0
		for _, e := range entries {
			if e.IsDir() {
				continue
			}
			name := e.Name()
			if !strings.HasSuffix(name, ".yaml") && !strings.HasSuffix(name, ".yml") {
				continue
			}
			fullPath := filepath.Join(dir, name)
			ilog.Info("importing: %s", name)
			subAddCmd.Run(nil, []string{"file://" + fullPath})
			count++
		}
		if count == 0 {
			ilog.Warn("no .yaml/.yml files found in %s", dir)
		} else {
			ilog.Ok("processed %d config file(s) — use 'clashctl sub list' to check", count)
		}
	},
}
