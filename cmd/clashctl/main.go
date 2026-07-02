package main

import (
	"os"
	"os/exec"

	"github.com/spf13/cobra"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

var (
	cfg         *config.EnvConfig
	rootCmd     *cobra.Command
	exitProcess = os.Exit
)

func main() {
	cfg = config.LoadEnv()
	rootCmd = &cobra.Command{
		Use:   "clashctl",
		Short: "Clash proxy manager for Linux",
		Long:  "clashctl - Manage Clash/Mihomo proxy on Linux. CLI + TUI dashboard.",
		Run: func(cmd *cobra.Command, args []string) {
			showHelpAndExit(cmd)
		},
	}
	rootCmd.AddCommand(startCmd)
	rootCmd.AddCommand(stopCmd)
	rootCmd.AddCommand(restartCmd)
	rootCmd.AddCommand(statusCmd)
	rootCmd.AddCommand(logCmd)
	rootCmd.AddCommand(proxyCmd)
	rootCmd.AddCommand(tunCmd)
	rootCmd.AddCommand(secretCmd)
	rootCmd.AddCommand(upgradeCmd)
	rootCmd.AddCommand(upgradeKernelCmd)
	rootCmd.AddCommand(configCmd)
	rootCmd.AddCommand(subCmd)
	rootCmd.AddCommand(nodeCmd)
	rootCmd.AddCommand(envCmd)
	rootCmd.AddCommand(testCmd)
	rootCmd.AddCommand(tuiCmd)
	rootCmd.AddCommand(doctorCmd)
	rootCmd.AddCommand(geodataCmd)
	rootCmd.AddCommand(versionCmd)

	if err := rootCmd.Execute(); err != nil {
		exitProcess(1)
	}
}

func showHelpAndExit(cmd *cobra.Command) {
	_ = cmd.Help()
	exitProcess(1)
}

func requireInstall() {
	if _, err := os.Stat(cfg.ClashBaseDir); os.IsNotExist(err) {
		ilog.Fatal("clashctl not installed. Run install.sh first.")
	}
	if _, err := os.Stat(cfg.ResourcesDir()); os.IsNotExist(err) {
		ilog.Fatal("clashctl not installed. Run install.sh first.")
	}
}

func launchEditor(path string) error {
	editor := os.Getenv("EDITOR")
	if editor == "" {
		editor = "vim"
	}
	cmd := exec.Command(editor, path)
	cmd.Stdin = os.Stdin
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	return cmd.Run()
}
