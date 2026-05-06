package main

import (
	"fmt"

	"github.com/spf13/cobra"
)

var versionCmd = &cobra.Command{
	Use:   "version",
	Short: "Show version information",
	Run:   runVersion,
}

func init() {
	rootCmd.AddCommand(versionCmd)
}

func runVersion(cmd *cobra.Command, args []string) {
	fmt.Printf("clashctl version %s\n", cyan("0.1.0"))
	fmt.Printf("  Clash-Terminal — Terminal proxy management tool\n")
	fmt.Printf("  License: GPL-3.0\n")
}
