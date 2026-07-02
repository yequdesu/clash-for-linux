package main

import (
	"fmt"
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
			ilog.Fatal("directory not found: %s", dir)
		}
		entries, err := os.ReadDir(dir)
		if err != nil {
			ilog.Fatal("cannot read directory %s: %v", dir, err)
		}
		imported := 0
		failed := 0
		for _, e := range entries {
			if e.IsDir() {
				continue
			}
			name := e.Name()
			lowerName := strings.ToLower(name)
			if !strings.HasSuffix(lowerName, ".yaml") && !strings.HasSuffix(lowerName, ".yml") {
				continue
			}
			fullPath := filepath.Join(dir, name)
			ilog.Info("importing: %s", name)
			result, err := addSubscription(cfg, "file://"+fullPath)
			if err != nil {
				failed++
				ilog.Warn("import failed for %s: %v", name, err)
				continue
			}
			imported++
			logSub(fmt.Sprintf("imported: [%d] %s", result.ID, result.Source))
			ilog.Ok("imported: [%d] %s", result.ID, result.Source)
			if result.ActivateFirst {
				ilog.Info("activating first subscription...")
				if err := switchSubscription(cfg, result.ID); err != nil {
					ilog.Fatal("%v", err)
				}
			}
		}
		if imported == 0 && failed == 0 {
			ilog.Fatal("no .yaml/.yml files found in %s", dir)
		} else if failed > 0 {
			ilog.Fatal("imported %d config file(s), failed %d — run 'clashctl sub list' to check", imported, failed)
		} else {
			ilog.Ok("imported %d config file(s) — use 'clashctl sub list' to check", imported)
		}
	},
}
