package main

import (
	"fmt"
	"strconv"
	"strings"
	"time"

	"github.com/spf13/cobra"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

const profileTimeFormat = "2006-01-02 15:04:05"

var timeNow = time.Now

var subRenameCmd = &cobra.Command{
	Use:   "rename <id> <name>",
	Short: "Rename a subscription profile",
	Args:  cobra.MinimumNArgs(2),
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		id, err := parseSubscriptionIDArg(args[0])
		if err != nil {
			ilog.Fatal("%v", err)
		}
		name := strings.TrimSpace(strings.Join(args[1:], " "))
		if name == "" {
			ilog.Fatal("subscription name cannot be empty")
		}
		if err := updateProfileMetadata(cfg, id, func(p *config.Profile) error {
			p.Name = name
			return nil
		}); err != nil {
			ilog.Fatal("%v", err)
		}
		logSub(fmt.Sprintf("renamed: [%d] %s", id, name))
		ilog.Ok("subscription renamed: [%d]", id)
	},
}

var subSetURLCmd = &cobra.Command{
	Use:   "set-url <id> <url|path>",
	Short: "Set subscription source URL or file path",
	Args:  cobra.ExactArgs(2),
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		id, err := parseSubscriptionIDArg(args[0])
		if err != nil {
			ilog.Fatal("%v", err)
		}
		source := normalizeSubscriptionSource(args[1])
		if source == "" {
			ilog.Fatal("subscription URL cannot be empty")
		}
		if err := updateProfileMetadata(cfg, id, func(p *config.Profile) error {
			p.URL = source
			if p.Name == "" {
				p.Name = profileNameFromSource(source)
			}
			return nil
		}); err != nil {
			ilog.Fatal("%v", err)
		}
		logSub(fmt.Sprintf("source changed: [%d] %s", id, source))
		ilog.Ok("subscription source updated: [%d]", id)
	},
}

var subSetIntervalCmd = &cobra.Command{
	Use:   "set-interval <id> <duration|off>",
	Short: "Set per-profile subscription update interval",
	Args:  cobra.ExactArgs(2),
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		id, err := parseSubscriptionIDArg(args[0])
		if err != nil {
			ilog.Fatal("%v", err)
		}
		interval, enabled, err := normalizeUpdateInterval(args[1])
		if err != nil {
			ilog.Fatal("%v", err)
		}
		if err := updateProfileMetadata(cfg, id, func(p *config.Profile) error {
			p.UpdateEnabled = boolPtr(enabled)
			p.UpdateInterval = interval
			p.Interval = interval
			p.NextUpdate = nextProfileUpdateString(*p, timeNow())
			return nil
		}); err != nil {
			ilog.Fatal("%v", err)
		}
		logSub(fmt.Sprintf("update interval changed: [%d] %s", id, interval))
		ilog.Ok("subscription update interval saved: [%d]", id)
	},
}

var subSetUpdateProxyCmd = &cobra.Command{
	Use:   "set-update-proxy <id> <direct|system|core|auto>",
	Short: "Set network path used for subscription updates",
	Args:  cobra.ExactArgs(2),
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		id, value, err := parseProfileEnum(args, "update proxy", []string{"direct", "system", "core", "auto"})
		if err != nil {
			ilog.Fatal("%v", err)
		}
		if err := updateProfileMetadata(cfg, id, func(p *config.Profile) error {
			p.UpdateProxy = value
			return nil
		}); err != nil {
			ilog.Fatal("%v", err)
		}
		logSub(fmt.Sprintf("update proxy changed: [%d] %s", id, value))
		ilog.Ok("subscription update proxy saved: [%d]", id)
	},
}

var subSetUserAgentCmd = &cobra.Command{
	Use:   "set-user-agent <id> <user-agent>",
	Short: "Set per-profile subscription User-Agent",
	Args:  cobra.MinimumNArgs(2),
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		id, err := parseSubscriptionIDArg(args[0])
		if err != nil {
			ilog.Fatal("%v", err)
		}
		userAgent := strings.TrimSpace(strings.Join(args[1:], " "))
		if err := updateProfileMetadata(cfg, id, func(p *config.Profile) error {
			p.UserAgent = userAgent
			return nil
		}); err != nil {
			ilog.Fatal("%v", err)
		}
		logSub(fmt.Sprintf("user-agent changed: [%d]", id))
		ilog.Ok("subscription user-agent saved: [%d]", id)
	},
}

var subSetConvertCmd = &cobra.Command{
	Use:   "set-convert <id> <auto|off|force>",
	Short: "Set per-profile subscription conversion mode",
	Args:  cobra.ExactArgs(2),
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		id, value, err := parseProfileEnum(args, "convert mode", []string{"auto", "off", "force"})
		if err != nil {
			ilog.Fatal("%v", err)
		}
		if err := updateProfileMetadata(cfg, id, func(p *config.Profile) error {
			p.ConvertMode = value
			return nil
		}); err != nil {
			ilog.Fatal("%v", err)
		}
		logSub(fmt.Sprintf("convert mode changed: [%d] %s", id, value))
		ilog.Ok("subscription convert mode saved: [%d]", id)
	},
}

var subTagCmd = &cobra.Command{
	Use:   "tag <add|remove> <id> <tag>",
	Short: "Add or remove a subscription tag",
	Args:  cobra.ExactArgs(3),
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		op := strings.ToLower(strings.TrimSpace(args[0]))
		if op != "add" && op != "remove" {
			ilog.Fatal("tag operation must be add or remove")
		}
		id, err := parseSubscriptionIDArg(args[1])
		if err != nil {
			ilog.Fatal("%v", err)
		}
		tag := strings.TrimSpace(args[2])
		if tag == "" {
			ilog.Fatal("tag cannot be empty")
		}
		if err := updateProfileMetadata(cfg, id, func(p *config.Profile) error {
			switch op {
			case "add":
				p.Tags = addUniqueTag(p.Tags, tag)
			case "remove":
				p.Tags = removeTag(p.Tags, tag)
			}
			return nil
		}); err != nil {
			ilog.Fatal("%v", err)
		}
		logSub(fmt.Sprintf("tag %s: [%d] %s", op, id, tag))
		ilog.Ok("subscription tag %s: [%d]", op, id)
	},
}

func parseSubscriptionIDArg(raw string) (int, error) {
	id, err := strconv.Atoi(strings.TrimSpace(raw))
	if err != nil || id <= 0 {
		return 0, fmt.Errorf("invalid subscription id")
	}
	return id, nil
}

func parseProfileEnum(args []string, label string, allowed []string) (int, string, error) {
	id, err := parseSubscriptionIDArg(args[0])
	if err != nil {
		return 0, "", err
	}
	value, err := normalizeProfileEnum(args[1], label, allowed)
	if err != nil {
		return 0, "", err
	}
	return id, value, nil
}

func normalizeProfileEnum(raw string, label string, allowed []string) (string, error) {
	value := strings.ToLower(strings.TrimSpace(raw))
	for _, candidate := range allowed {
		if value == candidate {
			return value, nil
		}
	}
	return "", fmt.Errorf("invalid %s %q, allowed: %s", label, value, strings.Join(allowed, "|"))
}

func updateProfileMetadata(cfg *config.EnvConfig, id int, mutate func(*config.Profile) error) error {
	return config.WithProfilesLock(cfg, func() error {
		meta, err := config.LoadProfiles(cfg.ProfilesMeta())
		if err != nil {
			return err
		}
		p := meta.FindByID(id)
		if p == nil {
			return fmt.Errorf("subscription id %d not found", id)
		}
		beforeURL := p.URL
		if err := mutate(p); err != nil {
			return err
		}
		if p.URL != beforeURL && meta.FindByURL(p.URL) != nil {
			for _, candidate := range meta.Profiles {
				if candidate.ID != id && candidate.URL == p.URL {
					return fmt.Errorf("subscription already exists")
				}
			}
		}
		return saveProfiles(cfg.ProfilesMeta(), meta)
	})
}

func normalizeUpdateInterval(raw string) (string, bool, error) {
	value := strings.ToLower(strings.TrimSpace(raw))
	if value == "" {
		return "", false, fmt.Errorf("update interval cannot be empty")
	}
	if value == "off" || value == "disable" || value == "disabled" || value == "0" {
		return "off", false, nil
	}
	d, err := time.ParseDuration(value)
	if err != nil || d <= 0 {
		return "", false, fmt.Errorf("invalid update interval %q, use Go duration like 12h or off", raw)
	}
	return value, true, nil
}

func profileUpdateInterval(p config.Profile) (time.Duration, bool) {
	raw := strings.TrimSpace(p.UpdateInterval)
	if raw == "" {
		raw = strings.TrimSpace(p.Interval)
	}
	if raw == "" {
		raw = "12h"
	}
	raw = strings.ToLower(raw)
	if raw == "off" || raw == "disable" || raw == "disabled" || raw == "0" {
		return 0, false
	}
	d, err := time.ParseDuration(raw)
	if err != nil || d <= 0 {
		return 0, false
	}
	return d, true
}

func profileIntervalLabel(p config.Profile) string {
	if !profileUpdateEnabled(p) {
		return "off"
	}
	raw := strings.TrimSpace(p.UpdateInterval)
	if raw == "" {
		raw = strings.TrimSpace(p.Interval)
	}
	if raw == "" {
		return "12h"
	}
	return raw
}

func profileUpdateEnabled(p config.Profile) bool {
	if p.UpdateEnabled != nil {
		return *p.UpdateEnabled
	}
	if strings.HasPrefix(strings.ToLower(strings.TrimSpace(p.URL)), "file://") {
		return false
	}
	_, ok := profileUpdateInterval(p)
	return ok
}

func profileLastUpdatedTime(p config.Profile) time.Time {
	for _, raw := range []string{p.LastUpdated, p.Updated} {
		raw = strings.TrimSpace(raw)
		if raw == "" {
			continue
		}
		if ts, err := time.ParseInLocation(profileTimeFormat, raw, time.Local); err == nil {
			return ts
		}
	}
	return time.Time{}
}

func profileDueForUpdate(p config.Profile, now time.Time) bool {
	if !profileUpdateEnabled(p) {
		return false
	}
	interval, ok := profileUpdateInterval(p)
	if !ok {
		return false
	}
	if next := strings.TrimSpace(p.NextUpdate); next != "" {
		if ts, err := time.ParseInLocation(profileTimeFormat, next, time.Local); err == nil {
			return !ts.After(now)
		}
	}
	last := profileLastUpdatedTime(p)
	return last.IsZero() || !last.Add(interval).After(now)
}

func nextProfileUpdateString(p config.Profile, now time.Time) string {
	if !profileUpdateEnabled(p) {
		return ""
	}
	interval, ok := profileUpdateInterval(p)
	if !ok {
		return ""
	}
	last := profileLastUpdatedTime(p)
	if last.IsZero() {
		last = now
	}
	return last.Add(interval).Format(profileTimeFormat)
}

func nextProfileRetryString(p config.Profile, now time.Time) string {
	if !profileUpdateEnabled(p) {
		return ""
	}
	interval, ok := profileUpdateInterval(p)
	if !ok {
		return ""
	}
	return now.Add(interval).Format(profileTimeFormat)
}

func boolPtr(v bool) *bool {
	return &v
}

func addUniqueTag(tags []string, tag string) []string {
	for _, existing := range tags {
		if strings.EqualFold(existing, tag) {
			return tags
		}
	}
	return append(tags, tag)
}

func removeTag(tags []string, tag string) []string {
	filtered := tags[:0]
	for _, existing := range tags {
		if !strings.EqualFold(existing, tag) {
			filtered = append(filtered, existing)
		}
	}
	if len(filtered) == 0 {
		return nil
	}
	return filtered
}
