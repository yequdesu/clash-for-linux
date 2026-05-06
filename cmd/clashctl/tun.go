package main

import (
	"fmt"
	"path/filepath"

	"github.com/spf13/cobra"
	"github.com/yequdesu/clashctl/internal/config"
)

var tunCmd = &cobra.Command{
	Use:   "tun",
	Short: "TUN mode switch",
	Long: `TUN mode routes all system traffic through the proxy using a virtual network interface.

Note: TUN mode requires sudo/root privileges.`,
}

var tunOnCmd = &cobra.Command{
	Use:   "on",
	Short: "Enable TUN mode",
	Long: `Enable TUN mode - requires sudo/root privileges.

This will modify mixin.yaml to set tun.enable=true, then restart the kernel.
After enabling, all system traffic will be routed through the proxy.`,
	Run: runTUNOn,
}

var tunOffCmd = &cobra.Command{
	Use:   "off",
	Short: "Disable TUN mode",
	Long:  "Disable TUN mode - requires sudo/root privileges.",
	Run:   runTUNOff,
}

var tunStatusCmd = &cobra.Command{
	Use:   "status",
	Short: "Show TUN mode status",
	Long:  "Show whether TUN mode is enabled.",
	Run:   runTUNStatus,
}

func init() {
	tunCmd.AddCommand(tunOnCmd)
	tunCmd.AddCommand(tunOffCmd)
	tunCmd.AddCommand(tunStatusCmd)
	rootCmd.AddCommand(tunCmd)
}

func runTUNOn(cmd *cobra.Command, args []string) {
	mixinPath := filepath.Join(clashResourcesDir, "mixin.yaml")

	if !fileExists(mixinPath) {
		fmt.Printf("%s mixin.yaml not found\n", red("✗"))
		return
	}

	fmt.Println(yellow("⚠ TUN mode requires sudo/root privileges to modify network settings."))
	fmt.Println(yellow("⚠ This will restart the proxy kernel."))
	fmt.Print("Continue? [y/N] ")

	var answer string
	fmt.Scanln(&answer)
	if answer != "y" && answer != "Y" && answer != "yes" {
		fmt.Println("Cancelled.")
		return
	}

	fmt.Print("↓ Enabling TUN mode... ")
	if err := config.SetTUNMode(mixinPath, true); err != nil {
		fmt.Printf("\n%s Failed to enable TUN: %v\n", red("✗"), err)
		return
	}
	fmt.Println(green("✓ Done"))

	fmt.Println()
	fmt.Println("TUN mode enabled. Run 'clashctl restart' to apply changes.")
}

func runTUNOff(cmd *cobra.Command, args []string) {
	mixinPath := filepath.Join(clashResourcesDir, "mixin.yaml")

	if !fileExists(mixinPath) {
		fmt.Printf("%s mixin.yaml not found\n", red("✗"))
		return
	}

	fmt.Print("↓ Disabling TUN mode... ")
	if err := config.SetTUNMode(mixinPath, false); err != nil {
		fmt.Printf("\n%s Failed to disable TUN: %v\n", red("✗"), err)
		return
	}
	fmt.Println(green("✓ Done"))

	fmt.Println()
	fmt.Println("TUN mode disabled. Run 'clashctl restart' to apply changes.")
}

func runTUNStatus(cmd *cobra.Command, args []string) {
	mixinPath := filepath.Join(clashResourcesDir, "mixin.yaml")

	enabled, err := config.GetTUNMode(mixinPath)
	if err != nil {
		fmt.Printf("%s Cannot read TUN status: %v\n", red("✗"), err)
		return
	}

	if enabled {
		fmt.Printf("%s TUN mode: %s\n", green("●"), green("Enabled"))
	} else {
		fmt.Printf("%s TUN mode: %s\n", gray("○"), "Disabled")
	}
}
