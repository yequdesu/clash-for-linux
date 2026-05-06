package main

import (
	"fmt"
	"math/rand"
	"time"

	"github.com/spf13/cobra"
)

var secretCmd = &cobra.Command{
	Use:   "secret",
	Short: "API secret key management",
	Long:  "Show or generate API secret key for Mihomo controller.",
}

var secretShowCmd = &cobra.Command{
	Use:   "show",
	Short: "Show current API secret",
	Run:   runSecretShow,
}

var secretNewCmd = &cobra.Command{
	Use:   "new",
	Short: "Generate new API secret",
	Run:   runSecretNew,
}

func init() {
	secretCmd.AddCommand(secretShowCmd)
	secretCmd.AddCommand(secretNewCmd)
	rootCmd.AddCommand(secretCmd)
}

func runSecretShow(cmd *cobra.Command, args []string) {
	secret, err := readFile(secretFilePath())
	if err != nil || secret == "" {
		fmt.Println(gray("No API secret set."))
		return
	}
	fmt.Printf("API Secret: %s\n", cyan(secret))
}

func runSecretNew(cmd *cobra.Command, args []string) {
	seed := time.Now().UnixNano()
	rng := rand.New(rand.NewSource(seed))

	const charset = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
	secret := make([]byte, 32)
	for i := range secret {
		secret[i] = charset[rng.Intn(len(charset))]
	}
	secretStr := string(secret)

	writeFile(secretFilePath(), secretStr)
	fmt.Printf("%s New API secret generated\n", green("✓"))
	fmt.Printf("  %s\n", cyan(secretStr))
	fmt.Println()
	fmt.Println(yellow("⚠ Remember to run 'clashctl restart' for the new secret to take effect."))
}
