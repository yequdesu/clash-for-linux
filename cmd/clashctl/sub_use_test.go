package main

import (
	"errors"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
)

func TestSwitchSubscriptionRestoresConfigWhenMergeFails(t *testing.T) {
	base := t.TempDir()
	cfg := &config.EnvConfig{
		ClashBaseDir: base,
		KernelName:   "missing-kernel",
		ClashSubUA:   "test",
	}
	if err := os.MkdirAll(cfg.ResourcesDir(), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.MkdirAll(cfg.ProfilesDir(), 0o755); err != nil {
		t.Fatal(err)
	}

	oldConfig := []byte("proxies:\n  - name: old\n")
	oldRuntime := []byte("mixed-port: 7890\n")
	if err := os.WriteFile(cfg.ConfigPath(), oldConfig, 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(cfg.RuntimePath(), oldRuntime, 0o644); err != nil {
		t.Fatal(err)
	}

	nextProfile := filepath.Join(cfg.ProfilesDir(), "2.yaml")
	if err := os.WriteFile(nextProfile, []byte("proxies:\n  - name: next\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	meta := &config.ProfilesMeta{
		Use: 1,
		Profiles: []config.Profile{
			{ID: 1, Path: filepath.Join(cfg.ProfilesDir(), "1.yaml"), URL: "file://old.yaml"},
			{ID: 2, Path: nextProfile, URL: "file://next.yaml"},
		},
	}
	if err := config.SaveProfiles(cfg.ProfilesMeta(), meta); err != nil {
		t.Fatal(err)
	}

	err := switchSubscription(cfg, 2)
	if err == nil || !strings.Contains(err.Error(), "restored previous config") {
		t.Fatalf("switchSubscription error = %v, want restored previous config", err)
	}

	configData, err := os.ReadFile(cfg.ConfigPath())
	if err != nil {
		t.Fatal(err)
	}
	if string(configData) != string(oldConfig) {
		t.Fatalf("config.yaml = %q, want %q", configData, oldConfig)
	}
	runtimeData, err := os.ReadFile(cfg.RuntimePath())
	if err != nil {
		t.Fatal(err)
	}
	if string(runtimeData) != string(oldRuntime) {
		t.Fatalf("runtime.yaml = %q, want %q", runtimeData, oldRuntime)
	}
	after, err := config.LoadProfiles(cfg.ProfilesMeta())
	if err != nil {
		t.Fatal(err)
	}
	if after.Use != 1 {
		t.Fatalf("active profile = %d, want 1", after.Use)
	}
}

func TestSwitchSubscriptionRestoresRunningServiceWhenStartFails(t *testing.T) {
	base := t.TempDir()
	cfg := &config.EnvConfig{
		ClashBaseDir: base,
		KernelName:   "missing-kernel",
		ClashSubUA:   "test",
	}
	if err := os.MkdirAll(cfg.ResourcesDir(), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.MkdirAll(cfg.ProfilesDir(), 0o755); err != nil {
		t.Fatal(err)
	}

	oldConfig := []byte("proxies:\n  - name: old\n")
	oldRuntime := []byte("mixed-port: 7890\n")
	if err := os.WriteFile(cfg.ConfigPath(), oldConfig, 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(cfg.RuntimePath(), oldRuntime, 0o644); err != nil {
		t.Fatal(err)
	}
	nextProfile := filepath.Join(cfg.ProfilesDir(), "2.yaml")
	if err := os.WriteFile(nextProfile, []byte("proxies:\n  - name: next\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	meta := &config.ProfilesMeta{
		Use: 1,
		Profiles: []config.Profile{
			{ID: 1, Path: filepath.Join(cfg.ProfilesDir(), "1.yaml"), URL: "file://old.yaml"},
			{ID: 2, Path: nextProfile, URL: "file://next.yaml"},
		},
	}
	if err := config.SaveProfiles(cfg.ProfilesMeta(), meta); err != nil {
		t.Fatal(err)
	}

	originalMerge := mergeRuntimeConfig
	mergeRuntimeConfig = func(cfg *config.EnvConfig, _ bool) error {
		return os.WriteFile(cfg.RuntimePath(), []byte("mixed-port: 7999\n"), 0o644)
	}
	defer func() { mergeRuntimeConfig = originalMerge }()

	fakeSvc := &fakeSubscriptionService{running: true, failFirstStart: true}
	originalServiceFactory := newSubscriptionService
	newSubscriptionService = func(*config.EnvConfig) subscriptionService {
		return fakeSvc
	}
	defer func() { newSubscriptionService = originalServiceFactory }()

	err := switchSubscription(cfg, 2)
	if err == nil || !strings.Contains(err.Error(), "start failed, restored previous config") {
		t.Fatalf("switchSubscription error = %v, want restored start failure", err)
	}

	configData, err := os.ReadFile(cfg.ConfigPath())
	if err != nil {
		t.Fatal(err)
	}
	if string(configData) != string(oldConfig) {
		t.Fatalf("config.yaml = %q, want %q", configData, oldConfig)
	}
	runtimeData, err := os.ReadFile(cfg.RuntimePath())
	if err != nil {
		t.Fatal(err)
	}
	if string(runtimeData) != string(oldRuntime) {
		t.Fatalf("runtime.yaml = %q, want %q", runtimeData, oldRuntime)
	}
	after, err := config.LoadProfiles(cfg.ProfilesMeta())
	if err != nil {
		t.Fatal(err)
	}
	if after.Use != 1 {
		t.Fatalf("active profile = %d, want 1", after.Use)
	}
	if fakeSvc.stopCount != 1 {
		t.Fatalf("stopCount = %d, want 1", fakeSvc.stopCount)
	}
	if fakeSvc.startCount != 2 {
		t.Fatalf("startCount = %d, want 2", fakeSvc.startCount)
	}
	if !fakeSvc.running {
		t.Fatal("old service was not restarted")
	}
}

func TestSwitchSubscriptionDoesNotStartStoppedService(t *testing.T) {
	base := t.TempDir()
	testCfg := &config.EnvConfig{
		ClashBaseDir: base,
		KernelName:   "missing-kernel",
		ClashSubUA:   "test",
	}
	oldGlobalCfg := cfg
	cfg = testCfg
	defer func() { cfg = oldGlobalCfg }()

	if err := os.MkdirAll(testCfg.ResourcesDir(), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.MkdirAll(testCfg.ProfilesDir(), 0o755); err != nil {
		t.Fatal(err)
	}

	oldConfig := []byte("proxies:\n  - name: old\n")
	oldRuntime := []byte("mixed-port: 7890\n")
	if err := os.WriteFile(testCfg.ConfigPath(), oldConfig, 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(testCfg.RuntimePath(), oldRuntime, 0o644); err != nil {
		t.Fatal(err)
	}
	nextProfile := filepath.Join(testCfg.ProfilesDir(), "2.yaml")
	if err := os.WriteFile(nextProfile, []byte("proxies:\n  - name: next\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	meta := &config.ProfilesMeta{
		Use: 1,
		Profiles: []config.Profile{
			{ID: 1, Path: filepath.Join(testCfg.ProfilesDir(), "1.yaml"), URL: "file://old.yaml"},
			{ID: 2, Path: nextProfile, URL: "file://next.yaml"},
		},
	}
	if err := config.SaveProfiles(testCfg.ProfilesMeta(), meta); err != nil {
		t.Fatal(err)
	}

	originalMerge := mergeRuntimeConfig
	mergeRuntimeConfig = func(cfg *config.EnvConfig, _ bool) error {
		return os.WriteFile(cfg.RuntimePath(), []byte("mixed-port: 7999\n"), 0o644)
	}
	defer func() { mergeRuntimeConfig = originalMerge }()

	fakeSvc := &fakeSubscriptionService{running: false, failFirstStart: true}
	originalServiceFactory := newSubscriptionService
	newSubscriptionService = func(*config.EnvConfig) subscriptionService {
		return fakeSvc
	}
	defer func() { newSubscriptionService = originalServiceFactory }()

	if err := switchSubscription(testCfg, 2); err != nil {
		t.Fatalf("switchSubscription: %v", err)
	}

	configData, err := os.ReadFile(testCfg.ConfigPath())
	if err != nil {
		t.Fatal(err)
	}
	nextConfig := []byte("proxies:\n  - name: next\n")
	if string(configData) != string(nextConfig) {
		t.Fatalf("config.yaml = %q, want %q", configData, nextConfig)
	}
	runtimeData, err := os.ReadFile(testCfg.RuntimePath())
	if err != nil {
		t.Fatal(err)
	}
	if string(runtimeData) != "mixed-port: 7999\n" {
		t.Fatalf("runtime.yaml = %q, want merged runtime", runtimeData)
	}
	after, err := config.LoadProfiles(testCfg.ProfilesMeta())
	if err != nil {
		t.Fatal(err)
	}
	if after.Use != 2 {
		t.Fatalf("active profile = %d, want 2", after.Use)
	}
	if fakeSvc.startCount != 0 {
		t.Fatalf("startCount = %d, want 0", fakeSvc.startCount)
	}
}

type fakeSubscriptionService struct {
	running        bool
	failFirstStart bool
	startCount     int
	stopCount      int
}

func (s *fakeSubscriptionService) IsRunning() bool { return s.running }

func (s *fakeSubscriptionService) Stop() error {
	s.stopCount++
	s.running = false
	return nil
}

func (s *fakeSubscriptionService) Start() error {
	s.startCount++
	if s.failFirstStart && s.startCount == 1 {
		return errors.New("forced start failure")
	}
	s.running = true
	return nil
}

func (s *fakeSubscriptionService) ProxyPort() string { return "7890" }
