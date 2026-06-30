package main

import (
	"fmt"
	"os"
	"path/filepath"
	"strconv"
	"time"

	"github.com/spf13/cobra"
	"github.com/yequdesu/clashctl/internal/config"
)

var stopCmd = &cobra.Command{
	Use:   "stop",
	Short: "Stop Mihomo proxy kernel and clear system proxy",
	Long:  "Stop Mihomo proxy kernel and clear system proxy environment variables.",
	Run:   runStop,
}

func init() {
	rootCmd.AddCommand(stopCmd)
}

func runStop(cmd *cobra.Command, args []string) {
	pid := readPID()

	if pid == "" {
		fmt.Println(yellow("⚠ Mihomo is not running"))
		// Still clear proxy from shell RC files if requested
		clearShellProxy()
		return
	}

	// Kill only the process we own, by PID
	pidInt, err := strconv.Atoi(pid)
	if err != nil {
		fmt.Printf("%s Invalid PID in pid file: %s\n", red("✗"), pid)
		os.Remove(pidFilePath())
		return
	}

	proc, err := os.FindProcess(pidInt)
	if err != nil {
		// PID file exists but process not found — clean up stale PID file
		fmt.Printf("%s Process not found (PID: %d), cleaning up...\n", yellow("⚠"), pidInt)
		os.Remove(pidFilePath())
		fmt.Printf("%s Mihomo stopped\n", green("✓"))
		return
	}

	// Graceful shutdown first (SIGTERM), then force kill if still running
	if err := proc.Signal(os.Interrupt); err != nil {
		// Process might already be dead or owned by another user
		fmt.Printf("%s Cannot signal process (PID: %d): %v\n", yellow("⚠"), pidInt, err)
		fmt.Println(yellow("  Try: sudo kill") + " " + pid)
		os.Remove(pidFilePath())
		return
	}

	// Wait up to 3 seconds for graceful shutdown
	for i := 0; i < 30; i++ {
		if !isRunningByPID(pidInt) {
			break
		}
		time.Sleep(100 * time.Millisecond)
	}

	// Force kill if still running
	if isRunningByPID(pidInt) {
		proc.Kill()
		time.Sleep(100 * time.Millisecond)
	}

	os.Remove(pidFilePath())

	fmt.Printf("%s Mihomo stopped (PID: %d)\n", green("✓"), pidInt)

	// Actually clear system proxy from shell RC files
	clearShellProxy()
}

// clearShellProxy removes proxy environment variables from shell RC files
func clearShellProxy() {
	home, err := os.UserHomeDir()
	if err != nil {
		return
	}
	cleared := 0
	for _, rc := range []string{".bashrc", ".zshrc"} {
		rcPath := filepath.Join(home, rc)
		if fileExists(rcPath) {
			if err := config.RemoveShellRC(rcPath); err == nil {
				cleared++
			}
		}
	}
	if cleared > 0 {
		fmt.Printf("%s Proxy removed from shell RC files (%d file(s))\n", green("✓"), cleared)
	}

	// Remind user that CURRENT shell still has proxy vars set
	printCurrentShellHint()
}

// printCurrentShellHint prints instructions to clear proxy from the current shell
func printCurrentShellHint() {
	fmt.Println()
	fmt.Println(yellow("⚠ Your CURRENT shell still has proxy environment variables set."))
	fmt.Println(yellow("  To restore internet access NOW, run:"))
	fmt.Println()
	fmt.Println(cyan("  unset http_proxy https_proxy HTTP_PROXY HTTPS_PROXY all_proxy ALL_PROXY no_proxy NO_PROXY"))
	fmt.Println()
	fmt.Println("  New terminals will use direct connection automatically.")
}
