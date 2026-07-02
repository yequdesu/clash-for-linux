package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
)

func TestChecksumForFileGNUFormat(t *testing.T) {
	sum := "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
	content := sum + "  mihomo-linux-amd64-v1.gz\n"

	got, err := checksumForFile(content, "https://example.com/download/mihomo-linux-amd64-v1.gz")
	if err != nil {
		t.Fatalf("checksumForFile: %v", err)
	}
	if got != sum {
		t.Fatalf("checksum = %q, want %q", got, sum)
	}
}

func TestChecksumForFileBSDFormat(t *testing.T) {
	sum := "abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd"
	content := "SHA256 (mihomo-linux-arm64-v1.gz) = " + sum + "\n"

	got, err := checksumForFile(content, "mihomo-linux-arm64-v1.gz")
	if err != nil {
		t.Fatalf("checksumForFile: %v", err)
	}
	if got != sum {
		t.Fatalf("checksum = %q, want %q", got, sum)
	}
}

func TestChecksumForFileMissing(t *testing.T) {
	_, err := checksumForFile("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef  other.gz\n", "mihomo.gz")
	if err == nil {
		t.Fatal("expected missing checksum error")
	}
}

func TestUpdateInstallStateKernelRecordsSourceAndChecksum(t *testing.T) {
	baseDir := t.TempDir()
	cfg := &config.EnvConfig{ClashBaseDir: baseDir, KernelName: "mihomo"}
	if err := os.MkdirAll(cfg.BinDir(), 0o755); err != nil {
		t.Fatal(err)
	}
	state := map[string]any{
		"schema_version": 1,
		"components": map[string]any{
			"yq": map[string]any{"version": "v4", "source_url": "https://example.test/yq", "path": "/tmp/yq"},
		},
	}
	data, err := json.Marshal(state)
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(cfg.InstallState(), data, 0o644); err != nil {
		t.Fatal(err)
	}

	if err := updateInstallStateKernel(
		cfg,
		"v1.2.3",
		"https://example.test/mihomo.gz",
		"https://example.test/SHA256SUMS",
		true,
		false,
	); err != nil {
		t.Fatal(err)
	}

	out, err := os.ReadFile(cfg.InstallState())
	if err != nil {
		t.Fatal(err)
	}
	var parsed map[string]any
	if err := json.Unmarshal(out, &parsed); err != nil {
		t.Fatal(err)
	}
	components := parsed["components"].(map[string]any)
	if _, ok := components["yq"]; !ok {
		t.Fatal("existing yq component was removed")
	}
	mihomo := components["mihomo"].(map[string]any)
	if mihomo["version"] != "v1.2.3" {
		t.Fatalf("version = %v", mihomo["version"])
	}
	if mihomo["source_url"] != "https://example.test/mihomo.gz" {
		t.Fatalf("source_url = %v", mihomo["source_url"])
	}
	if mihomo["checksum_url"] != "https://example.test/SHA256SUMS" {
		t.Fatalf("checksum_url = %v", mihomo["checksum_url"])
	}
	if mihomo["checksum_verified"] != true {
		t.Fatalf("checksum_verified = %v", mihomo["checksum_verified"])
	}
	if mihomo["allow_unsigned"] != false {
		t.Fatalf("allow_unsigned = %v", mihomo["allow_unsigned"])
	}
	if mihomo["path"] != filepath.Join(baseDir, "bin", "mihomo") {
		t.Fatalf("path = %v", mihomo["path"])
	}
	if mihomo["updated_at"] == "" {
		t.Fatal("updated_at was empty")
	}
}

func TestUpdateInstallStateKernelFailsWhenStateMissing(t *testing.T) {
	cfg := &config.EnvConfig{ClashBaseDir: t.TempDir(), KernelName: "mihomo"}
	if err := updateInstallStateKernel(cfg, "v1", "https://example.test/mihomo.gz", "", false, true); err == nil {
		t.Fatal("expected missing install-state error")
	}
}
