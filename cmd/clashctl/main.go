package main

import (
	"fmt"
	"os"

	"github.com/spf13/cobra"
)

var rootCmd = &cobra.Command{
	Use:   "clashctl",
	Short: "Clash-Terminal CLI — terminal proxy management tool",
	Long: `Clash-Terminal CLI provides complete control over Mihomo proxy kernel.
Manage subscriptions, switch proxy nodes, monitor connections, and more.`,
	Version: "0.1.0",
	Run: func(cmd *cobra.Command, args []string) {
		cmd.Help()
	},
}

func Execute() {
	if err := rootCmd.Execute(); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

func main() {
	Execute()
}
