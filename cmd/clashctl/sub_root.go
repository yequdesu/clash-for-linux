package main

import (
	"github.com/spf13/cobra"
)

var subCmd = &cobra.Command{
	Use:   "sub",
	Short: "Manage subscriptions",
	Long:  "Add, list, remove, use, update, import, or view subscription logs.",
}

func init() {
	subCmd.AddCommand(subAddCmd)
	subCmd.AddCommand(subListCmd)
	subCmd.AddCommand(subRemoveCmd)
	subCmd.AddCommand(subUseCmd)
	subCmd.AddCommand(subUpdateCmd)
	subCmd.AddCommand(subImportCmd)
	subCmd.AddCommand(subLogCmd)
}
