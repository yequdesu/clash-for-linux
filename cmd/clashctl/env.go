package main

import "github.com/spf13/cobra"

var envCmd = &cobra.Command{
	Use:   "env",
	Short: "Print proxy environment variables (for shell eval)",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		printProxyExports(cfg)
	},
}
