package main

import (
	"encoding/json"
	"os"
	"strings"
	"testing"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
)

func TestIsSafeController(t *testing.T) {
	tests := []struct {
		controller string
		want       bool
	}{
		{controller: "127.0.0.1:9090", want: true},
		{controller: "localhost:9090", want: true},
		{controller: "[::1]:9090", want: true},
		{controller: "0.0.0.0:9090", want: false},
		{controller: "192.168.1.10:9090", want: false},
		{controller: ":9090", want: false},
	}

	for _, tt := range tests {
		t.Run(tt.controller, func(t *testing.T) {
			if got := isSafeController(tt.controller); got != tt.want {
				t.Fatalf("isSafeController(%q) = %v, want %v", tt.controller, got, tt.want)
			}
		})
	}
}

func TestValidateInstallState(t *testing.T) {
	baseDir := t.TempDir()
	cfg := &config.EnvConfig{
		ClashBaseDir: baseDir,
		ServiceName:  "clashctl",
		KernelName:   "mihomo",
	}
	state := installStateFile{
		SchemaVersion: 1,
		BaseDir:       baseDir,
		ServiceName:   "clashctl",
		KernelName:    "mihomo",
		Components: map[string]installComponent{
			"mihomo":   {Version: "v1", SourceURL: "https://example.test/mihomo", Path: "/tmp/mihomo"},
			"yq":       {Version: "v1", SourceURL: "https://example.test/yq", Path: "/tmp/yq"},
			"geodata":  {Version: "v1", SourceURL: "https://example.test/geodata", Path: "/tmp/resources"},
			"clashctl": {Path: "/tmp/clashctl"},
		},
	}
	data, err := json.Marshal(state)
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(cfg.InstallState(), data, 0o644); err != nil {
		t.Fatal(err)
	}

	message, err := validateInstallState(cfg.InstallState(), cfg)
	if err != nil {
		t.Fatalf("validateInstallState returned error: %v", err)
	}
	if !strings.Contains(message, "schema 1") {
		t.Fatalf("message = %q, want schema summary", message)
	}

	state.ServiceName = "other"
	data, err = json.Marshal(state)
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(cfg.InstallState(), data, 0o644); err != nil {
		t.Fatal(err)
	}
	if _, err := validateInstallState(cfg.InstallState(), cfg); err == nil || !strings.Contains(err.Error(), "service_name mismatch") {
		t.Fatalf("validateInstallState mismatch error = %v", err)
	}
}

func TestHasCronAutoUpdate(t *testing.T) {
	tests := []struct {
		name    string
		crontab string
		want    bool
	}{
		{name: "enabled", crontab: "*/10 * * * * /usr/local/bin/clashctl sub update --scheduled --cron\n", want: true},
		{name: "legacy cron only", crontab: "0 */12 * * * /usr/local/bin/clashctl sub update --cron\n", want: false},
		{name: "commented", crontab: "# */10 * * * * clashctl sub update --scheduled --cron\n", want: false},
		{name: "missing cron flag", crontab: "0 */12 * * * clashctl sub update\n", want: false},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if got := hasCronAutoUpdate(tt.crontab); got != tt.want {
				t.Fatalf("hasCronAutoUpdate = %v, want %v", got, tt.want)
			}
		})
	}
}
