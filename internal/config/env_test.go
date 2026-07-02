package config

import (
	"path/filepath"
	"testing"
)

func TestLoadEnvDefaultsServiceName(t *testing.T) {
	baseDir := t.TempDir()
	t.Setenv("CLASH_BASE_DIR", baseDir)

	cfg := LoadEnv()
	if cfg.ServiceName != "clashctl" {
		t.Fatalf("ServiceName = %q, want clashctl", cfg.ServiceName)
	}
	if cfg.KernelName != "mihomo" {
		t.Fatalf("KernelName = %q, want mihomo", cfg.KernelName)
	}
	if cfg.VersionGeodata != "latest" {
		t.Fatalf("VersionGeodata = %q, want latest", cfg.VersionGeodata)
	}
	wantState := filepath.Join(baseDir, "install-state.json")
	if cfg.InstallState() != wantState {
		t.Fatalf("InstallState = %q, want %q", cfg.InstallState(), wantState)
	}
}

func TestLoadEnvProcessOverrides(t *testing.T) {
	baseDir := t.TempDir()
	t.Setenv("CLASH_BASE_DIR", baseDir)
	t.Setenv("SERVICE_NAME", "custom-clashctl")
	t.Setenv("KERNEL_NAME", "custom-mihomo")
	t.Setenv("VERSION_GEODATA", "20250101")

	cfg := LoadEnv()
	if cfg.ServiceName != "custom-clashctl" {
		t.Fatalf("ServiceName = %q, want custom-clashctl", cfg.ServiceName)
	}
	if cfg.KernelName != "custom-mihomo" {
		t.Fatalf("KernelName = %q, want custom-mihomo", cfg.KernelName)
	}
	if cfg.VersionGeodata != "20250101" {
		t.Fatalf("VersionGeodata = %q, want 20250101", cfg.VersionGeodata)
	}
	if cfg.KernelBin() != filepath.Join(baseDir, "bin", "custom-mihomo") {
		t.Fatalf("KernelBin = %q", cfg.KernelBin())
	}
	if cfg.PidFile() != filepath.Join(baseDir, "runtime", "custom-mihomo.pid") {
		t.Fatalf("PidFile = %q", cfg.PidFile())
	}
}
