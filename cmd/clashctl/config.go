package main

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"

	"github.com/spf13/cobra"
	"github.com/yequdesu/clashctl/internal/config"
)

var configCmd = &cobra.Command{
	Use:   "config",
	Short: "Configuration management",
	Long:  "View, edit, merge, and validate configuration files.",
}

var configViewCmd = &cobra.Command{
	Use:   "view",
	Short: "View current runtime configuration",
	Long:  "Display the merged runtime configuration.",
	Run:   runConfigView,
}

var configEditCmd = &cobra.Command{
	Use:   "edit",
	Short: "Edit mixin.yaml",
	Long:  "Open mixin.yaml in $EDITOR for editing.",
	Run:   runConfigEdit,
}

var configMergeCmd = &cobra.Command{
	Use:   "merge",
	Short: "Manually trigger configuration merge",
	Long:  "Merge config.yaml and mixin.yaml into runtime.yaml.",
	Run:   runConfigMerge,
}

var configValidateCmd = &cobra.Command{
	Use:   "validate",
	Short: "Validate configuration",
	Long:  "Validate runtime.yaml using mihomo -t.",
	Run:   runConfigValidate,
}

func init() {
	configCmd.AddCommand(configViewCmd)
	configCmd.AddCommand(configEditCmd)
	configCmd.AddCommand(configMergeCmd)
	configCmd.AddCommand(configValidateCmd)
	rootCmd.AddCommand(configCmd)
}

func runConfigView(cmd *cobra.Command, args []string) {
	runtimePath := filepath.Join(clashResourcesDir, "runtime.yaml")
	if !fileExists(runtimePath) {
		fmt.Printf("%s runtime.yaml not found. Run 'clashctl config merge' first.\n", yellow("⚠"))
		return
	}
	data, _ := os.ReadFile(runtimePath)
	fmt.Print(string(data))
}

func runConfigEdit(cmd *cobra.Command, args []string) {
	mixinPath := filepath.Join(clashResourcesDir, "mixin.yaml")

	// Copy mixin from resources if not found
	if !fileExists(mixinPath) {
		fmt.Printf("%s mixin.yaml not found at %s\n", yellow("⚠"), mixinPath)
		if fileExists("resources/mixin.yaml") {
			os.MkdirAll(filepath.Dir(mixinPath), 0755)
			data, _ := os.ReadFile("resources/mixin.yaml")
			os.WriteFile(mixinPath, data, 0644)
			fmt.Printf("%s Copied default mixin.yaml\n", green("✓"))
		} else {
			return
		}
	}

	editor := os.Getenv("EDITOR")
	if editor == "" {
		editor = "nano"
	}

	runCmd(editor, mixinPath)
}

func runConfigMerge(cmd *cobra.Command, args []string) {
	configPath := filepath.Join(clashResourcesDir, "config.yaml")
	mixinPath := filepath.Join(clashResourcesDir, "mixin.yaml")
	runtimePath := filepath.Join(clashResourcesDir, "runtime.yaml")

	if !fileExists(configPath) {
		fmt.Printf("%s config.yaml not found. Use 'clashctl sub use <id>' to select a subscription.\n", yellow("⚠"))
		return
	}

	fmt.Print("↓ Merging configuration... ")
	if err := config.MergeConfig(configPath, mixinPath, runtimePath); err != nil {
		fmt.Printf("\n%s Merge failed: %v\n", red("✗"), err)
		return
	}
	fmt.Println(green("✓ Done"))
	fmt.Printf("  %s → runtime.yaml\n", cyan("Merged"))
}

func runConfigValidate(cmd *cobra.Command, args []string) {
	runtimePath := filepath.Join(clashResourcesDir, "runtime.yaml")

	if !fileExists(runtimePath) {
		fmt.Printf("%s runtime.yaml not found. Run 'clashctl config merge' first.\n", yellow("⚠"))
		return
	}

	mihomoPath := filepath.Join(clashBinDir, "mihomo")
	if !fileExists(mihomoPath) {
		fmt.Printf("%s Mihomo kernel not found at %s\n", red("✗"), mihomoPath)
		return
	}

	fmt.Print("↓ Validating configuration... ")

	cmd := exec.Command(mihomoPath, "-d", clashResourcesDir, "-f", runtimePath, "-t")
	out, err := cmd.CombinedOutput()
	output := string(out)

	if err != nil {
		fmt.Printf("\n%s Validation failed:\n%s\n", red("✗"), output)
		return
	}

	fmt.Println(green("✓ Configuration is valid"))
}
