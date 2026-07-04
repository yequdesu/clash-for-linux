package main

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/spf13/cobra"

	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

var completionSystem bool

var completionCmd = &cobra.Command{
	Use:   "completion",
	Short: "Generate or install shell completion",
	Long:  "Generate completion scripts for bash, zsh, fish, and powershell, or install/remove them for the current user.",
	Run: func(cmd *cobra.Command, args []string) {
		showHelpAndExit(cmd)
	},
}

var completionBashCmd = &cobra.Command{
	Use:   "bash",
	Short: "Generate bash completion script",
	Run: func(cmd *cobra.Command, args []string) {
		if err := rootCmd.GenBashCompletion(os.Stdout); err != nil {
			ilog.Fatal("generate bash completion failed: %v", err)
		}
	},
}

var completionZshCmd = &cobra.Command{
	Use:   "zsh",
	Short: "Generate zsh completion script",
	Run: func(cmd *cobra.Command, args []string) {
		if err := rootCmd.GenZshCompletion(os.Stdout); err != nil {
			ilog.Fatal("generate zsh completion failed: %v", err)
		}
	},
}

var completionFishCmd = &cobra.Command{
	Use:   "fish",
	Short: "Generate fish completion script",
	Run: func(cmd *cobra.Command, args []string) {
		if err := rootCmd.GenFishCompletion(os.Stdout, true); err != nil {
			ilog.Fatal("generate fish completion failed: %v", err)
		}
	},
}

var completionPowerShellCmd = &cobra.Command{
	Use:     "powershell",
	Aliases: []string{"pwsh"},
	Short:   "Generate powershell completion script",
	Run: func(cmd *cobra.Command, args []string) {
		if err := rootCmd.GenPowerShellCompletion(os.Stdout); err != nil {
			ilog.Fatal("generate powershell completion failed: %v", err)
		}
	},
}

var completionInstallCmd = &cobra.Command{
	Use:   "install [bash|zsh|fish|powershell]",
	Short: "Install completion for the current shell",
	Args:  cobra.MaximumNArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		shell := ""
		if len(args) == 1 {
			shell = args[0]
		}
		if err := installCompletion(shell, completionSystem); err != nil {
			ilog.Fatal("completion install failed: %v", err)
		}
	},
}

var completionUninstallCmd = &cobra.Command{
	Use:   "uninstall [bash|zsh|fish|powershell]",
	Short: "Remove installed completion",
	Args:  cobra.MaximumNArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		shell := ""
		if len(args) == 1 {
			shell = args[0]
		}
		if err := uninstallCompletion(shell, completionSystem); err != nil {
			ilog.Fatal("completion uninstall failed: %v", err)
		}
	},
}

func init() {
	completionInstallCmd.Flags().BoolVar(&completionSystem, "system", false, "install completion to system paths")
	completionUninstallCmd.Flags().BoolVar(&completionSystem, "system", false, "remove completion from system paths")
	completionCmd.AddCommand(
		completionBashCmd,
		completionZshCmd,
		completionFishCmd,
		completionPowerShellCmd,
		completionInstallCmd,
		completionUninstallCmd,
	)
}

func installCompletion(shell string, system bool) error {
	shell = normalizeShell(shell)
	if shell == "" {
		return fmt.Errorf("could not detect shell; pass bash, zsh, fish, or powershell")
	}
	path, err := completionPath(shell, system)
	if err != nil {
		return err
	}
	if system && runningAsRoot() == false {
		rerunWithSudoOrFatal("completion install")
	}
	script, err := generateCompletion(shell)
	if err != nil {
		return err
	}
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		return err
	}
	if err := os.WriteFile(path, []byte(script), 0o644); err != nil {
		return err
	}
	if shell == "zsh" && !system {
		if err := ensureZshCompletionBlock(filepath.Dir(path)); err != nil {
			return err
		}
	}
	ilog.Ok("completion installed: %s", path)
	return nil
}

func uninstallCompletion(shell string, system bool) error {
	shell = normalizeShell(shell)
	if shell == "" {
		return fmt.Errorf("could not detect shell; pass bash, zsh, fish, or powershell")
	}
	path, err := completionPath(shell, system)
	if err != nil {
		return err
	}
	if system && runningAsRoot() == false {
		rerunWithSudoOrFatal("completion uninstall")
	}
	if err := os.Remove(path); err != nil && !os.IsNotExist(err) {
		return err
	}
	if shell == "zsh" && !system {
		_ = removeManagedBlock(filepath.Join(userHomeDir(), ".zshrc"), "clashctl completion")
	}
	ilog.Ok("completion removed: %s", path)
	return nil
}

func normalizeShell(shell string) string {
	shell = strings.ToLower(strings.TrimSpace(shell))
	if shell == "" {
		shell = filepath.Base(os.Getenv("SHELL"))
	}
	switch shell {
	case "bash", "zsh", "fish", "powershell", "pwsh":
		if shell == "pwsh" {
			return "powershell"
		}
		return shell
	default:
		return ""
	}
}

func completionPath(shell string, system bool) (string, error) {
	home := userHomeDir()
	switch shell {
	case "bash":
		if system {
			return "/etc/bash_completion.d/clashctl", nil
		}
		return filepath.Join(home, ".local", "share", "bash-completion", "completions", "clashctl"), nil
	case "zsh":
		if system {
			return "/usr/local/share/zsh/site-functions/_clashctl", nil
		}
		return filepath.Join(home, ".zsh", "completions", "_clashctl"), nil
	case "fish":
		if system {
			return "/usr/share/fish/vendor_completions.d/clashctl.fish", nil
		}
		return filepath.Join(home, ".config", "fish", "completions", "clashctl.fish"), nil
	case "powershell":
		return filepath.Join(home, ".config", "powershell", "clashctl-completion.ps1"), nil
	default:
		return "", fmt.Errorf("unsupported shell: %s", shell)
	}
}

func generateCompletion(shell string) (string, error) {
	var b strings.Builder
	var err error
	switch shell {
	case "bash":
		err = rootCmd.GenBashCompletion(&b)
	case "zsh":
		err = rootCmd.GenZshCompletion(&b)
	case "fish":
		err = rootCmd.GenFishCompletion(&b, true)
	case "powershell":
		err = rootCmd.GenPowerShellCompletion(&b)
	default:
		return "", fmt.Errorf("unsupported shell: %s", shell)
	}
	return b.String(), err
}

func ensureZshCompletionBlock(dir string) error {
	rc := filepath.Join(userHomeDir(), ".zshrc")
	block := fmt.Sprintf("fpath=(%s $fpath)\nautoload -Uz compinit\ncompinit\n", shellQuote(dir))
	return upsertManagedBlock(rc, "clashctl completion", block)
}
