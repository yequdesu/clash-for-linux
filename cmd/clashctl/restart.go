package main

import (
	"time"

	"github.com/spf13/cobra"
)

var restartCmd = &cobra.Command{
	Use:   "restart",
	Short: "Restart Mihomo proxy kernel",
	Long:  "Restart the Mihomo proxy kernel.",
	Run:   runRestart,
}

func init() {
	rootCmd.AddCommand(restartCmd)
}

func runRestart(cmd *cobra.Command, args []string) {
	runStop(nil, nil)
	// Wait for port release (TCP TIME_WAIT) before starting
	time.Sleep(1 * time.Second)
	runStart(nil, nil)
}
