package config

import (
	"os"
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

func TestDefaultBaseDirPrefersExistingInstallDirs(t *testing.T) {
	home := t.TempDir()
	hidden := filepath.Join(home, ".clashctl")
	if err := os.Mkdir(hidden, 0o755); err != nil {
		t.Fatal(err)
	}
	if got := defaultBaseDir(home); got != hidden {
		t.Fatalf("defaultBaseDir = %q, want existing hidden dir %q", got, hidden)
	}

	legacy := filepath.Join(home, "clashctl")
	if err := os.Mkdir(legacy, 0o755); err != nil {
		t.Fatal(err)
	}
	if got := defaultBaseDir(home); got != legacy {
		t.Fatalf("defaultBaseDir = %q, want existing legacy dir %q", got, legacy)
	}
}

func TestReadBaseDirFromEnvFileExpandsHomeOverride(t *testing.T) {
	home := t.TempDir()
	marker := filepath.Join(t.TempDir(), "install.env")
	if err := os.WriteFile(marker, []byte("CLASH_BASE_DIR=~/.clashctl\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	got, ok := readBaseDirFromEnvFile(marker, home)
	if !ok {
		t.Fatal("readBaseDirFromEnvFile did not find CLASH_BASE_DIR")
	}
	if want := filepath.Join(home, ".clashctl"); got != want {
		t.Fatalf("base dir = %q, want %q", got, want)
	}
}
