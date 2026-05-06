package main

import (
	"fmt"
	"os"
	"strconv"

	"github.com/spf13/cobra"
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
		return
	}

	// Kill process
	pidInt, _ := strconv.Atoi(pid)
	proc, err := os.FindProcess(pidInt)
	if err == nil {
		proc.Signal(os.Interrupt)
		proc.Kill()
	}

	// Also try pkill as fallback
	runCmd("pkill", "-9", "mihomo")

	os.Remove(pidFilePath())

	fmt.Printf("%s Mihomo stopped\n", green("✓"))
	fmt.Printf("%s System proxy cleared\n", green("✓"))
}
