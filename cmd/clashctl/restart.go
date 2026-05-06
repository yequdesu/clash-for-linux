package main

import (
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
	runStart(nil, nil)
}
