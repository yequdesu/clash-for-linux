package main

import (
	"fmt"
	"os"
	"os/exec"

	"github.com/spf13/cobra"
)

var tuiCmd = &cobra.Command{
	Use:   "tui",
	Short: "Launch TUI dashboard",
	Long:  "Start the clash-tui terminal dashboard.",
	Run:   runTUI,
}

func init() {
	rootCmd.AddCommand(tuiCmd)
}

func runTUI(cmd *cobra.Command, args []string) {
	tuiPath := "/usr/local/bin/clash-tui"
	if !fileExists(tuiPath) {
		fmt.Printf("%s clash-tui binary not found at %s\n", yellow("⚠"), tuiPath)
		fmt.Println("To install TUI, run: bash install.sh")
		fmt.Println("Or compile with: cd tui && cargo build --release")
		os.Exit(1)
	}

	tuiCmd := exec.Command(tuiPath, args...)
	tuiCmd.Stdin = os.Stdin
	tuiCmd.Stdout = os.Stdout
	tuiCmd.Stderr = os.Stderr

	if err := tuiCmd.Run(); err != nil {
		fmt.Printf("%s TUI exited with error: %v\n", red("✗"), err)
	}
}
