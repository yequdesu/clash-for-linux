package main

import (
	"compress/gzip"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/spf13/cobra"
)

var upgradeKernelCmd = &cobra.Command{
	Use:   "upgrade-kernel",
	Short: "Upgrade Mihomo kernel",
	Long:  "Check and upgrade Mihomo kernel to the latest version from GitHub.",
	Run:   runUpgradeKernel,
}

func init() {
	rootCmd.AddCommand(upgradeKernelCmd)
}

func runUpgradeKernel(cmd *cobra.Command, args []string) {
	mihomoPath := filepath.Join(clashBinDir, "mihomo")

	if !fileExists(mihomoPath) {
		fmt.Printf("%s Mihomo kernel not found at %s\n", red("✗"), mihomoPath)
		return
	}

	fmt.Println("↓ Checking for Mihomo kernel updates...")

	latestURL := "https://github.com/MetaCubeX/mihomo/releases/latest"
	ghProxy := os.Getenv("GH_PROXY")
	if ghProxy != "" {
		latestURL = ghProxy + "/" + latestURL
	}

	// Get latest release info
	client := &http.Client{
		Timeout: 30 * time.Second,
		CheckRedirect: func(req *http.Request, via []*http.Request) error {
			return http.ErrUseLastResponse
		},
	}

	resp, err := client.Get(latestURL)
	if err != nil {
		fmt.Printf("%s Failed to check updates: %v\n", red("✗"), err)
		fmt.Println(yellow("⚠ GitHub may be inaccessible from your location."))
		fmt.Println(yellow("  Try setting GH_PROXY environment variable."))
		return
	}
	defer resp.Body.Close()

	loc := resp.Header.Get("Location")
	if loc == "" {
		fmt.Printf("%s Could not determine latest version\n", red("✗"))
		return
	}

	// Extract version from URL
	parts := strings.Split(strings.TrimRight(loc, "/"), "/")
	latestVer := parts[len(parts)-1]

	fmt.Printf("  Current: checking...\n")
	fmt.Printf("  Latest:  %s\n", cyan(latestVer))

	fmt.Print("Download and install? [y/N] ")
	var answer string
	fmt.Scanln(&answer)
	if answer != "y" && answer != "Y" && answer != "yes" {
		fmt.Println("Cancelled.")
		return
	}

	// Determine architecture
	arch := detectArch()
	downloadURL := fmt.Sprintf("https://github.com/MetaCubeX/mihomo/releases/download/%s/mihomo-linux-%s-%s.gz",
		latestVer, arch, latestVer)
	if ghProxy != "" {
		downloadURL = ghProxy + "/" + downloadURL
	}

	fmt.Printf("↓ Downloading mihomo-linux-%s...\n", arch)

	resp, err = http.Get(downloadURL)
	if err != nil {
		fmt.Printf("%s Download failed: %v\n", red("✗"), err)
		return
	}
	defer resp.Body.Close()

	tmpFile := mihomoPath + ".new"
	out, err := os.Create(tmpFile)
	if err != nil {
		fmt.Printf("%s Cannot create temp file: %v\n", red("✗"), err)
		return
	}
	defer out.Close()

	gzReader, err := gzip.NewReader(resp.Body)
	if err != nil {
		fmt.Printf("%s Failed to decompress: %v\n", red("✗"), err)
		return
	}
	defer gzReader.Close()

	_, err = io.Copy(out, gzReader)
	if err != nil {
		fmt.Printf("%s Download failed: %v\n", red("✗"), err)
		return
	}

	// Stop kernel if running
	if isRunning() {
		fmt.Print("↓ Stopping kernel... ")
		runStop(nil, nil)
		fmt.Println(green("✓ Done"))
	}

	// Replace binary
	os.Remove(mihomoPath)
	os.Rename(tmpFile, mihomoPath)
	os.Chmod(mihomoPath, 0755)

	fmt.Printf("%s Mihomo kernel upgraded to %s\n", green("✓"), cyan(latestVer))
	fmt.Println(yellow("  Run 'clashctl start' to restart the proxy."))
}

func detectArch() string {
	out, _ := runCmd("uname", "-m")
	arch := strings.TrimSpace(out)

	// Check CPU flags for v1/v2/v3/v4
	switch arch {
	case "x86_64":
		return detectX86Variant()
	case "aarch64":
		return "arm64"
	case "armv7l":
		return "armv7"
	default:
		return "amd64"
	}
}

func detectX86Variant() string {
	data, err := os.ReadFile("/proc/cpuinfo")
	if err != nil {
		return "amd64"
	}
	info := string(data)

	if strings.Contains(info, "avx512") {
		return "amd64-v4"
	}
	if strings.Contains(info, "avx2") {
		return "amd64-v3"
	}
	if strings.Contains(info, "sse4_2") {
		return "amd64-v2"
	}
	return "amd64"
}
