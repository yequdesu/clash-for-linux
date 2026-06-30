package main

import (
	"fmt"
	"path/filepath"
	"strings"

	"github.com/spf13/cobra"
	"github.com/yequdesu/clashctl/internal/config"
	"github.com/yequdesu/clashctl/internal/kernel"
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

This will modify mixin.yaml to set tun.enable=true, merge config, then restart the kernel.
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

func applyTUNMode(enable bool) error {
	mixinPath := filepath.Join(clashResourcesDir, "mixin.yaml")
	configPath := filepath.Join(clashResourcesDir, "config.yaml")
	runtimePath := filepath.Join(clashResourcesDir, "runtime.yaml")

	if !fileExists(mixinPath) {
		return fmt.Errorf("mixin.yaml not found")
	}

	if err := config.SetTUNMode(mixinPath, enable); err != nil {
		return fmt.Errorf("failed to set TUN mode: %v", err)
	}

	fmt.Print("↓ Merging configuration... ")
	if err := config.MergeConfig(configPath, mixinPath, runtimePath); err != nil {
		return fmt.Errorf("\nconfig merge failed: %v", err)
	}
	fmt.Println(green("✓ Done"))

	fmt.Print("↓ Validating configuration... ")
	if err := kernel.ValidateConfig(clashResourcesDir, runtimePath, clashBinDir); err != nil {
		return fmt.Errorf("\nconfig validation failed: %v", err)
	}
	fmt.Println(green("✓ Done"))

	return nil
}

func verifyTUN() {
	out, err := runCmd("bash", "-c", "ip link show 2>/dev/null | grep -i 'tun\\|utun' | head -3")
	if err == nil && out != "" {
		lines := strings.Split(out, "\n")
		for _, line := range lines {
			fmt.Printf("  %s %s\n", green("✓"), strings.TrimSpace(line))
		}
	} else {
		fmt.Printf("  %s No TUN device detected (may need root/setcap)\n", yellow("⚠"))
		fmt.Println("  Run: sudo setcap cap_net_admin,cap_net_raw+ep ~/.clashctl/bin/mihomo")
	}
}

func runTUNOn(cmd *cobra.Command, args []string) {
	mixinPath := filepath.Join(clashResourcesDir, "mixin.yaml")

	if !fileExists(mixinPath) {
		fmt.Printf("%s mixin.yaml not found\n", red("✗"))
		return
	}

	// Save original TUN state for rollback
	wasEnabled, _ := config.GetTUNMode(mixinPath)
	if wasEnabled {
		fmt.Println(yellow("⚠ TUN mode is already enabled"))
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

	// Apply TUN config
	fmt.Print("↓ Enabling TUN mode... ")
	if err := config.SetTUNMode(mixinPath, true); err != nil {
		fmt.Printf("\n%s Failed to enable TUN: %v\n", red("✗"), err)
		return
	}
	fmt.Println(green("✓ Done"))

	if err := applyTUNMode(true); err != nil {
		// Rollback: restore original TUN state
		fmt.Printf("\n%s %v\n", red("✗"), err)
		fmt.Print("↓ Rolling back TUN config... ")
		if rbErr := config.SetTUNMode(mixinPath, wasEnabled); rbErr != nil {
			fmt.Printf("\n%s Rollback failed: %v\n", red("✗"), rbErr)
			fmt.Println(yellow("⚠ mixin.yaml may be in an inconsistent state. Check tun.enable manually."))
		} else {
			// Re-merge after rollback
			configPath := filepath.Join(clashResourcesDir, "config.yaml")
			if fileExists(configPath) {
				runtimePath := filepath.Join(clashResourcesDir, "runtime.yaml")
				config.MergeConfig(configPath, mixinPath, runtimePath)
			}
			fmt.Println(green("✓ Rolled back"))
		}
		return
	}

	// Restart kernel
	if isRunning() {
		fmt.Println("↓ Restarting kernel...")
		runStop(nil, nil)
		runStart(nil, nil)
	} else {
		fmt.Println("↓ Starting kernel...")
		runStart(nil, nil)
	}

	fmt.Printf("\n%s TUN mode enabled\n", green("✓"))
	verifyTUN()
	fmt.Println()
	fmt.Println("Tips: Ensure proxy mode is not DIRECT (use 'clashctl node switch GLOBAL <node>' or TUI).")
}

func runTUNOff(cmd *cobra.Command, args []string) {
	mixinPath := filepath.Join(clashResourcesDir, "mixin.yaml")

	if !fileExists(mixinPath) {
		fmt.Printf("%s mixin.yaml not found\n", red("✗"))
		return
	}

	// Save original TUN state for rollback
	wasEnabled, _ := config.GetTUNMode(mixinPath)
	if !wasEnabled {
		fmt.Println(yellow("⚠ TUN mode is already disabled"))
		return
	}

	fmt.Print("↓ Disabling TUN mode... ")
	if err := config.SetTUNMode(mixinPath, false); err != nil {
		fmt.Printf("\n%s Failed to disable TUN: %v\n", red("✗"), err)
		return
	}
	fmt.Println(green("✓ Done"))

	if err := applyTUNMode(false); err != nil {
		// Rollback: restore original TUN state
		fmt.Printf("\n%s %v\n", red("✗"), err)
		fmt.Print("↓ Rolling back TUN config... ")
		if rbErr := config.SetTUNMode(mixinPath, wasEnabled); rbErr != nil {
			fmt.Printf("\n%s Rollback failed: %v\n", red("✗"), rbErr)
			fmt.Println(yellow("⚠ mixin.yaml may be in an inconsistent state. Check tun.enable manually."))
		} else {
			// Re-merge after rollback
			configPath := filepath.Join(clashResourcesDir, "config.yaml")
			if fileExists(configPath) {
				runtimePath := filepath.Join(clashResourcesDir, "runtime.yaml")
				config.MergeConfig(configPath, mixinPath, runtimePath)
			}
			fmt.Println(green("✓ Rolled back"))
		}
		return
	}

	// Restart kernel
	if isRunning() {
		fmt.Println("↓ Restarting kernel...")
		runStop(nil, nil)
		runStart(nil, nil)
	} else {
		fmt.Println("↓ Starting kernel...")
		runStart(nil, nil)
	}

	fmt.Printf("\n%s TUN mode disabled\n", green("✓"))
}

func runTUNStatus(cmd *cobra.Command, args []string) {
	mixinPath := filepath.Join(clashResourcesDir, "mixin.yaml")

	enabled, err := config.GetTUNMode(mixinPath)
	if err != nil {
		fmt.Printf("%s Cannot read TUN status: %v\n", red("✗"), err)
		return
	}

	// Also check via API if kernel is running
	if isRunning() {
		envConfig, _ := config.LoadEnv(getEnvFilePath())
		port := envConfig.GetPort()
		apiClient := kernel.NewAPIClient(fmt.Sprintf("http://127.0.0.1:%d", port), readSecret())
		runtimeConfig, err := apiClient.GetConfig()
		if err == nil && runtimeConfig.TUN != nil {
			if runtimeConfig.TUN.Enable {
				fmt.Printf("%s TUN mode: %s (kernel)\n", green("●"), green("Enabled"))
			} else {
				fmt.Printf("%s TUN mode: %s (kernel)\n", gray("○"), "Disabled")
			}
			return
		}
	}

	if enabled {
		fmt.Printf("%s TUN mode: %s (config)\n", green("●"), green("Enabled"))
	} else {
		fmt.Printf("%s TUN mode: %s (config)\n", gray("○"), "Disabled")
	}
}
