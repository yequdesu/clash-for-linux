package main

import (
	"fmt"

	"github.com/spf13/cobra"
)

var appVersion = "v0.2.0-dev"

var versionCmd = &cobra.Command{
	Use:   "version",
	Short: "Show version",
	Run: func(cmd *cobra.Command, args []string) {
		fmt.Printf("clashctl %s\n", appVersion)
	},
}
