package main

import (
	"fmt"
	"os"
	"path/filepath"

	"github.com/spf13/cobra"
	"github.com/yequdesu/clashctl/internal/config"
	"github.com/yequdesu/clashctl/internal/kernel"
)

var startCmd = &cobra.Command{
	Use:   "start",
	Short: "Start Mihomo proxy kernel and set system proxy",
	Long: `Start Mihomo proxy kernel and optionally set system proxy environment variables.

Examples:
  clashctl start                # Start kernel + set system proxy
  clashctl start --no-proxy     # Start kernel only
  clashctl start --tun          # Start kernel + TUN mode`,
	Run: runStart,
}

var (
	startNoProxy bool
	startTUN     bool
)

func init() {
	startCmd.Flags().BoolVar(&startNoProxy, "no-proxy", false, "Start kernel without setting system proxy")
	startCmd.Flags().BoolVar(&startTUN, "tun", false, "Also enable TUN mode")
	rootCmd.AddCommand(startCmd)
}

func runStart(cmd *cobra.Command, args []string) {
	if isRunning() {
		fmt.Println(yellow("⚠ Mihomo is already running. Use 'clashctl restart' to restart."))
		return
	}

	ensureDir(clashRuntimeDir)
	ensureDir(clashLogsDir)

	envConfig, err := config.LoadEnv(getEnvFilePath())
	if err != nil {
		fmt.Printf("%s Failed to load config: %v\n", red("✗"), err)
		os.Exit(1)
	}

	profilesCfg, err := config.LoadProfiles(filepath.Join(clashResourcesDir, "profiles.yaml"))
	if err != nil {
		fmt.Printf("%s Failed to load profiles: %v\n", red("✗"), err)
		os.Exit(1)
	}

	if profilesCfg.Use == 0 || len(profilesCfg.Profiles) == 0 {
		fmt.Println(yellow("⚠ No active subscription. Please add a subscription first:"))
		fmt.Println("  clashctl sub add <url>")
		return
	}

	// Merge configuration
	configPath := filepath.Join(clashResourcesDir, "config.yaml")
	mixinPath := filepath.Join(clashResourcesDir, "mixin.yaml")
	runtimePath := filepath.Join(clashResourcesDir, "runtime.yaml")

	if !fileExists(configPath) {
		// Try ssrdog.yaml from current directory as a one-time import
		if fileExists("ssrdog.yaml") {
			fmt.Printf("%s Config file not found. Importing ssrdog.yaml as default.\n", yellow("⚠"))
			if data, err := os.ReadFile("ssrdog.yaml"); err == nil {
				os.WriteFile(configPath, data, 0644)
				fmt.Printf("%s ssrdog.yaml → %s\n", green("✓"), configPath)
			}
		}
		if !fileExists(configPath) {
			fmt.Printf("%s No config file found.\n", red("✗"))
			fmt.Println("  Add a subscription: clashctl sub add <url>")
			fmt.Println("  Or place a config at:", configPath)
			return
		}
	}

	fmt.Print("↓ Merging configuration... ")
	if err := config.MergeConfig(configPath, mixinPath, runtimePath); err != nil {
		fmt.Printf("\n%s Config merge failed: %v\n", red("✗"), err)
		os.Exit(1)
	}
	fmt.Println(green("✓ Done"))

	if startTUN {
		fmt.Print("↓ Enabling TUN mode... ")
		if err := config.SetTUNMode(mixinPath, true); err != nil {
			fmt.Printf("\n%s TUN setup failed: %v\n", red("✗"), err)
			os.Exit(1)
		}
		if err := config.MergeConfig(configPath, mixinPath, runtimePath); err != nil {
			fmt.Printf("%s Re-merge after TUN failed: %v\n", red("✗"), err)
			os.Exit(1)
		}
		fmt.Println(green("✓ Done"))
	}

	// Validate config
	fmt.Print("↓ Validating configuration... ")
	if err := kernel.ValidateConfig(clashResourcesDir, runtimePath, clashBinDir); err != nil {
		fmt.Printf("\n%s Config validation failed: %v\n", red("✗"), err)
		os.Exit(1)
	}
	fmt.Println(green("✓ Done"))

	// Start kernel
	fmt.Print("↓ Starting Mihomo kernel... ")
	pid, err := kernel.StartKernel(clashBinDir, clashResourcesDir, runtimePath, clashLogsDir, secretFilePath())
	if err != nil {
		fmt.Printf("\n%s Failed to start kernel: %v\n", red("✗"), err)
		os.Exit(1)
	}
	writeFile(pidFilePath(), fmt.Sprintf("%d", pid))
	fmt.Println(green("✓ Done"))

	// Wait for ready
	fmt.Print("↓ Waiting for kernel... ")
	port := envConfig.GetPort()
	if err := kernel.WaitForReady(port, 15*1000); err != nil {
		fmt.Printf("\n%s Kernel health check failed: %v\n", yellow("⚠"), err)
	} else {
		fmt.Println(green("✓ Ready"))
	}

	// Get version
	apiClient := kernel.NewAPIClient(fmt.Sprintf("http://127.0.0.1:%d", port), readSecret())
	version, _ := apiClient.GetVersion()

	fmt.Printf("\n%s Mihomo %s started\n", green("✓"), cyan(version))
	fmt.Printf("  %s HTTP: %s | SOCKS5: %s\n", green("✓"), cyan(fmt.Sprintf("127.0.0.1:%d", envConfig.MixedPort)), cyan(fmt.Sprintf("127.0.0.1:%d", envConfig.SocksPort)))

	if !startNoProxy {
		fmt.Printf("  %s System proxy set\n", green("✓"))
	}

	// Show active subscription
	if cfg := config.GetActiveProfile(profilesCfg); cfg != nil {
		fmt.Printf("  %s Current subscription: %s\n", cyan("→"), bold(cfg.Name))
	}
}

func readSecret() string {
	s, _ := readFile(secretFilePath())
	return s
}
