package main

import (
	"fmt"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"time"

	"github.com/spf13/cobra"
	"github.com/yequdesu/clashctl/internal/config"
	"github.com/yequdesu/clashctl/internal/sub"
)

var subCmd = &cobra.Command{
	Use:   "sub",
	Short: "Subscription management",
	Long:  "Add, remove, list, switch, and update proxy subscriptions.",
}

var subAddCmd = &cobra.Command{
	Use:   "add <url>",
	Short: "Add a new subscription",
	Long:  "Download and add a new proxy subscription by URL.",
	Args:  cobra.ExactArgs(1),
	Run:   runSubAdd,
}

var subRemoveCmd = &cobra.Command{
	Use:   "remove <id>",
	Short: "Remove a subscription",
	Long:  "Remove a subscription by ID.",
	Args:  cobra.ExactArgs(1),
	Run:   runSubRemove,
}

var subListCmd = &cobra.Command{
	Use:   "list",
	Short: "List all subscriptions",
	Long:  "List all subscriptions with status information.",
	Run:   runSubList,
}

var subUseCmd = &cobra.Command{
	Use:   "use <id>",
	Short: "Switch to a subscription",
	Long:  "Switch the active subscription by ID.",
	Args:  cobra.ExactArgs(1),
	Run:   runSubUse,
}

var subUpdateCmd = &cobra.Command{
	Use:   "update [id]",
	Short: "Update subscriptions",
	Long:  "Update a specific subscription by ID, or all subscriptions.",
	Run:   runSubUpdate,
}

var subLogCmd = &cobra.Command{
	Use:   "log",
	Short: "View subscription operation log",
	Long:  "View the subscription operation log.",
	Run:   runSubLog,
}

var (
	subName   string
	subAuto   bool
	subCron   bool
)

func init() {
	subAddCmd.Flags().StringVar(&subName, "name", "", "Subscription name")
	subUpdateCmd.Flags().BoolVar(&subAuto, "auto", false, "Auto-update mode")
	subUpdateCmd.Flags().BoolVar(&subCron, "cron", false, "Cron mode (silent)")

	subCmd.AddCommand(subAddCmd)
	subCmd.AddCommand(subRemoveCmd)
	subCmd.AddCommand(subListCmd)
	subCmd.AddCommand(subUseCmd)
	subCmd.AddCommand(subUpdateCmd)
	subCmd.AddCommand(subLogCmd)
	rootCmd.AddCommand(subCmd)
}

func runSubAdd(cmd *cobra.Command, args []string) {
	url := args[0]
	name := subName

	profilesPath := filepath.Join(clashResourcesDir, "profiles.yaml")
	profilesDir := filepath.Join(clashResourcesDir, "profiles")

	ensureDir(profilesDir)

	profilesCfg, err := config.LoadProfiles(profilesPath)
	if err != nil {
		profilesCfg = &config.ProfilesConfig{Use: 0, Profiles: []config.Profile{}}
	}

	// Determine new ID
	newID := 1
	for _, p := range profilesCfg.Profiles {
		if p.ID >= newID {
			newID = p.ID + 1
		}
	}

	targetPath := filepath.Join(profilesDir, fmt.Sprintf("%d.yaml", newID))

	fmt.Print("↓ Downloading subscription... ")
	downloaded, err := sub.DownloadSubscription(url, targetPath)
	if err != nil {
		fmt.Printf("\n%s Download failed: %v\n", red("✗"), err)
		return
	}

	if name == "" {
		name = downloaded.Name
	}
	if name == "" {
		// Extract name from URL
		name = extractNameFromURL(url)
	}

	profile := config.Profile{
		ID:      newID,
		Path:    targetPath,
		URL:     url,
		Name:    name,
		Updated: config.Now(),
	}

	if downloaded.Interval > 0 {
		profile.Interval = downloaded.Interval
	}

	if downloaded.Info != nil {
		profile.Extra = config.ProfileExtra{
			Upload:   downloaded.Info.Upload,
			Download: downloaded.Info.Download,
			Total:    downloaded.Info.Total,
			Expire:   downloaded.Info.Expire,
		}
	}

	fmt.Println(green("✓ Download successful"))
	fmt.Printf("  %s\n", cyan(fmt.Sprintf("%d proxies", downloaded.ProxyCount)))

	profilesCfg.Profiles = append(profilesCfg.Profiles, profile)
	profilesCfg.Use = newID

	if err := config.SaveProfiles(profilesPath, profilesCfg); err != nil {
		fmt.Printf("%s Failed to save profiles: %v\n", red("✗"), err)
		return
	}

	fmt.Printf("  %s Added subscription: %s (ID: %d)\n", green("✓"), bold(name), newID)
	fmt.Printf("  %s Activated as current subscription\n", green("✓"))
	fmt.Printf("  %s Proxy reloaded\n", green("✓"))

	// Copy to config.yaml
	os.Remove(filepath.Join(clashResourcesDir, "config.yaml"))
	if data, err := os.ReadFile(targetPath); err == nil {
		os.WriteFile(filepath.Join(clashResourcesDir, "config.yaml"), data, 0644)
	}
}

func runSubRemove(cmd *cobra.Command, args []string) {
	id, err := strconv.Atoi(args[0])
	if err != nil {
		fmt.Printf("%s Invalid ID: %s\n", red("✗"), args[0])
		return
	}

	profilesPath := filepath.Join(clashResourcesDir, "profiles.yaml")
	profilesCfg, err := config.LoadProfiles(profilesPath)
	if err != nil {
		fmt.Printf("%s Failed to load profiles: %v\n", red("✗"), err)
		return
	}

	var newProfiles []config.Profile
	found := false
	for _, p := range profilesCfg.Profiles {
		if p.ID == id {
			found = true
			os.Remove(p.Path)
			continue
		}
		newProfiles = append(newProfiles, p)
	}

	if !found {
		fmt.Printf("%s Subscription ID %d not found\n", red("✗"), id)
		return
	}

	profilesCfg.Profiles = newProfiles
	if profilesCfg.Use == id {
		if len(newProfiles) > 0 {
			profilesCfg.Use = newProfiles[0].ID
		} else {
			profilesCfg.Use = 0
		}
	}

	if err := config.SaveProfiles(profilesPath, profilesCfg); err != nil {
		fmt.Printf("%s Failed to save profiles: %v\n", red("✗"), err)
		return
	}

	fmt.Printf("%s Subscription removed (ID: %d)\n", green("✓"), id)
}

func runSubList(cmd *cobra.Command, args []string) {
	profilesPath := filepath.Join(clashResourcesDir, "profiles.yaml")
	profilesCfg, err := config.LoadProfiles(profilesPath)
	if err != nil || len(profilesCfg.Profiles) == 0 {
		fmt.Println(gray("No subscriptions found."))
		return
	}

	// Header
	separator := strings.Repeat("─", 70)
	fmt.Printf("┌%s┐\n", separator)
	fmt.Printf("│ %-4s %-20s %-8s %-20s %-6s │\n", "ID", "Name", "Status", "Updated", "Proxies")
	fmt.Printf("├%s┤\n", separator)

	for _, p := range profilesCfg.Profiles {
		marker := " "
		if p.ID == profilesCfg.Use {
			marker = green("●")
		}

		status := "ready"
		if p.ID == profilesCfg.Use {
			status = green("active")
		}

		updated := "--"
		if p.Updated > 0 {
			updated = tsToDate(p.Updated)
		}

		proxyCount := "--"
		if fileExists(p.Path) {
			if count, err := countProxiesInFile(p.Path); err == nil {
				proxyCount = fmt.Sprintf("%d", count)
			}
		}

		fmt.Printf("│ %s %-2d %-19s %-8s %-20s %-6s │\n",
			marker, p.ID, truncateStr(p.Name, 18), status, updated, proxyCount)
	}
	fmt.Printf("└%s┘\n", separator)
	fmt.Printf("  %s = current active\n", green("●"))
}

func runSubUse(cmd *cobra.Command, args []string) {
	id, err := strconv.Atoi(args[0])
	if err != nil {
		fmt.Printf("%s Invalid ID: %s\n", red("✗"), args[0])
		return
	}

	profilesPath := filepath.Join(clashResourcesDir, "profiles.yaml")
	profilesCfg, err := config.LoadProfiles(profilesPath)
	if err != nil {
		fmt.Printf("%s Failed to load profiles: %v\n", red("✗"), err)
		return
	}

	found := false
	for _, p := range profilesCfg.Profiles {
		if p.ID == id {
			found = true
			break
		}
	}
	if !found {
		fmt.Printf("%s Subscription ID %d not found\n", red("✗"), id)
		return
	}

	profilesCfg.Use = id
	if err := config.SaveProfiles(profilesPath, profilesCfg); err != nil {
		fmt.Printf("%s Failed to save profiles: %v\n", red("✗"), err)
		return
	}

	// Copy to config.yaml
	for _, p := range profilesCfg.Profiles {
		if p.ID == id {
			data, err := os.ReadFile(p.Path)
			if err == nil {
				os.WriteFile(filepath.Join(clashResourcesDir, "config.yaml"), data, 0644)
			}
			fmt.Printf("%s Switched to: %s (ID: %d)\n", green("✓"), bold(p.Name), id)
			if isRunning() {
				fmt.Println(yellow("⚠ Run 'clashctl restart' to apply changes."))
			}
			return
		}
	}
}

func runSubUpdate(cmd *cobra.Command, args []string) {
	profilesPath := filepath.Join(clashResourcesDir, "profiles.yaml")

	profilesCfg, err := config.LoadProfiles(profilesPath)
	if err != nil || len(profilesCfg.Profiles) == 0 {
		if !subCron {
			fmt.Println(gray("No subscriptions to update."))
		}
		return
	}

	var toUpdate []config.Profile
	if len(args) > 0 {
		id, err := strconv.Atoi(args[0])
		if err != nil {
			fmt.Printf("%s Invalid ID: %s\n", red("✗"), args[0])
			return
		}
		for _, p := range profilesCfg.Profiles {
			if p.ID == id {
				toUpdate = append(toUpdate, p)
				break
			}
		}
	} else {
		toUpdate = profilesCfg.Profiles
	}

	if len(toUpdate) == 0 {
		fmt.Println(gray("No subscriptions to update."))
		return
	}

	for i, p := range toUpdate {
		if !subCron {
			fmt.Printf("↓ [%d/%d] Updating: %s... ", i+1, len(toUpdate), p.Name)
		}

		_, err := sub.DownloadSubscription(p.URL, p.Path)
		if err != nil {
			if !subCron {
				fmt.Printf("\n%s Update failed: %v\n", red("✗"), err)
			}
			continue
		}

		// Update timestamp in profilesCfg
		for j := range profilesCfg.Profiles {
			if profilesCfg.Profiles[j].ID == p.ID {
				profilesCfg.Profiles[j].Updated = config.Now()
				break
			}
		}

		if !subCron {
			fmt.Println(green("✓ Done"))
		}
	}

	config.SaveProfiles(profilesPath, profilesCfg)

	if !subCron {
		fmt.Printf("%s All subscriptions updated\n", green("✓"))
		if isRunning() {
			fmt.Println(yellow("⚠ Run 'clashctl restart' to apply changes."))
		}
	}
}

func runSubLog(cmd *cobra.Command, args []string) {
	logPath := filepath.Join(clashResourcesDir, "profiles.log")
	data, err := os.ReadFile(logPath)
	if err != nil {
		fmt.Println(gray("No operation log found."))
		return
	}
	fmt.Print(string(data))
}

func extractNameFromURL(url string) string {
	url = strings.TrimPrefix(url, "https://")
	url = strings.TrimPrefix(url, "http://")
	if idx := strings.Index(url, "?"); idx > 0 {
		url = url[:idx]
	}
	if idx := strings.Index(url, "/"); idx > 0 {
		url = url[:idx]
	}
	return url
}

func truncateStr(s string, max int) string {
	runes := []rune(s)
	if len(runes) <= max {
		return s
	}
	return string(runes[:max-1]) + "…"
}

func tsToDate(ts int64) string {
	if ts == 0 {
		return "--"
	}
	t := time.Unix(ts, 0)
	return t.Format("01-02 15:04")
}

func countProxiesInFile(path string) (int, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return 0, err
	}
	return strings.Count(string(data), "name:"), nil
}
