package main

import (
	"fmt"
	"os"
	"path/filepath"
	"strconv"

	"github.com/spf13/cobra"
	"github.com/yequdesu/clashctl/internal/config"
)

var (
	rescueEnv bool
)

var rescueCmd = &cobra.Command{
	Use:   "rescue",
	Short: "Emergency: restore direct network access",
	Long: `Emergency recovery command — restores direct network access when the proxy
is broken or cannot start.

This is a LAST-RESORT command that:
  1. Force-kills any running mihomo kernel
  2. Removes all proxy environment variables from shell RC files
  3. Removes stale PID and runtime files

After running 'clashctl rescue', your system will use the direct network
connection (no proxy). To clear proxy from the current shell, run:

  eval $(clashctl rescue --env)

Or simply open a new terminal.`,
	Run: runRescue,
}

func init() {
	rescueCmd.Flags().BoolVar(&rescueEnv, "env", false, "Print unset commands for current shell (use with eval)")
	rootCmd.AddCommand(rescueCmd)
}

func runRescue(cmd *cobra.Command, args []string) {
	// --env flag: output ONLY the raw unset command for eval (no ANSI, no banner)
	if rescueEnv {
		fmt.Println("unset http_proxy https_proxy HTTP_PROXY HTTPS_PROXY all_proxy ALL_PROXY no_proxy NO_PROXY")
		return
	}

	fmt.Println(bold("=== Emergency Network Recovery ==="))
	fmt.Println()

	// 1. Force-kill kernel
	pid := readPID()
	if pid != "" {
		pidInt, err := strconv.Atoi(pid)
		if err == nil {
			proc, _ := os.FindProcess(pidInt)
			if proc != nil {
				proc.Signal(os.Interrupt)
				proc.Kill()
			}
		}
		os.Remove(pidFilePath())
		fmt.Printf("%s Kernel stopped\n", green("✓"))
	} else {
		fmt.Printf("%s No running kernel\n", gray("○"))
	}

	// 2. Clear proxy from shell RC files
	home, err := os.UserHomeDir()
	cleared := 0
	if err == nil {
		for _, rc := range []string{".bashrc", ".zshrc"} {
			rcPath := filepath.Join(home, rc)
			if fileExists(rcPath) {
				if err := config.RemoveShellRC(rcPath); err == nil {
					cleared++
				}
			}
		}
	}
	if cleared > 0 {
		fmt.Printf("%s Shell RC files cleaned (%d file(s))\n", green("✓"), cleared)
	} else {
		fmt.Printf("%s No proxy entries in shell RC files\n", gray("○"))
	}

	// 3. Clean runtime files
	os.Remove(filepath.Join(clashRuntimeDir, "mihomo.pid"))
	fmt.Printf("%s Runtime files cleaned\n", green("✓"))

	// 4. Print next steps
	fmt.Println()
	if rescueEnv {
		// Print unset commands for eval in current shell
		fmt.Println("unset http_proxy https_proxy HTTP_PROXY HTTPS_PROXY all_proxy ALL_PROXY no_proxy NO_PROXY")
		return
	}

	fmt.Println(bold("Direct network connection restored."))
	fmt.Println()
	fmt.Println("  To clear proxy from CURRENT shell:")
	fmt.Println("    " + cyan("eval $(clashctl rescue --env)"))
	fmt.Println()
	fmt.Println("  To re-enable the proxy later:")
	fmt.Println("    " + cyan("clashctl sub use <valid-id>"))
	fmt.Println("    " + cyan("clashctl start"))
	fmt.Println("    " + cyan("clashctl proxy on"))
}
