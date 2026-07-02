package sub

import (
	"net/http"
	"net/http/httptest"
	"net/url"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
	"gopkg.in/yaml.v3"
)

func TestSelectSubconverterPortAllowsRunningSubconverter(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path == "/version" {
			w.WriteHeader(http.StatusOK)
			return
		}
		w.WriteHeader(http.StatusNotFound)
	}))
	defer server.Close()

	port := serverPort(t, server.URL)
	got, running, err := selectSubconverterPort(server.Client(), port)
	if err != nil {
		t.Fatalf("selectSubconverterPort returned error: %v", err)
	}
	if !running {
		t.Fatal("running = false, want true")
	}
	if got != port {
		t.Fatalf("port = %q, want %q", got, port)
	}
}

func TestSelectSubconverterPortFallsBackWhenOccupiedByNonSubconverter(t *testing.T) {
	server := httptest.NewServer(http.NotFoundHandler())
	defer server.Close()

	port := serverPort(t, server.URL)
	got, running, err := selectSubconverterPort(server.Client(), port)
	if err != nil {
		t.Fatalf("selectSubconverterPort returned error: %v", err)
	}
	if running {
		t.Fatal("running = true, want false")
	}
	if got == "" || got == port {
		t.Fatalf("fallback port = %q, want a different free port", got)
	}
	if tcpPortOpen(got) {
		t.Fatalf("fallback port %s is already open", got)
	}
}

func TestTailFileLimit(t *testing.T) {
	path := filepath.Join(t.TempDir(), "latest.log")
	if err := os.WriteFile(path, []byte("0123456789"), 0o644); err != nil {
		t.Fatal(err)
	}
	if got := tailFile(path, 4); got != "6789" {
		t.Fatalf("tailFile = %q, want 6789", got)
	}
}

func TestUpdateSubconverterPrefSetsLoopbackPortAndKeepsOtherKeys(t *testing.T) {
	content := []byte(`
server:
  listen: 0.0.0.0
  port: 25500
advanced:
  log_level: debug
managed_config:
  managed_config_prefix: http://127.0.0.1:25500
`)
	got, err := updateSubconverterPref(content, "32123")
	if err != nil {
		t.Fatalf("updateSubconverterPref: %v", err)
	}

	var pref map[string]any
	if err := yaml.Unmarshal(got, &pref); err != nil {
		t.Fatalf("updated YAML parse failed: %v", err)
	}
	server := pref["server"].(map[string]any)
	if server["listen"] != "127.0.0.1" {
		t.Fatalf("listen = %v, want 127.0.0.1", server["listen"])
	}
	if server["port"] != 32123 {
		t.Fatalf("port = %v, want 32123", server["port"])
	}
	advanced := pref["advanced"].(map[string]any)
	if advanced["log_level"] != "debug" {
		t.Fatalf("advanced.log_level = %v, want debug", advanced["log_level"])
	}
	managedConfig := pref["managed_config"].(map[string]any)
	if managedConfig["managed_config_prefix"] != "http://127.0.0.1:32123" {
		t.Fatalf("managed_config_prefix = %v, want http://127.0.0.1:32123", managedConfig["managed_config_prefix"])
	}
}

func TestWriteManagedSubconverterPrefRestoresExistingConfig(t *testing.T) {
	base := t.TempDir()
	cfg := &config.EnvConfig{ClashBaseDir: base}
	if err := os.MkdirAll(cfg.SubconverterDir(), 0o755); err != nil {
		t.Fatal(err)
	}
	original := []byte("server:\n  listen: 0.0.0.0\n  port: 25500\n")
	if err := os.WriteFile(cfg.SubconverterConfig(), original, 0o644); err != nil {
		t.Fatal(err)
	}

	restore, err := writeManagedSubconverterPref(cfg, "32124")
	if err != nil {
		t.Fatalf("writeManagedSubconverterPref: %v", err)
	}
	updated, err := os.ReadFile(cfg.SubconverterConfig())
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(string(updated), "32124") || !strings.Contains(string(updated), "127.0.0.1") {
		t.Fatalf("managed pref was not updated: %s", updated)
	}
	if _, err := os.Stat(cfg.SubconverterConfig() + ".clashctl.bak"); err != nil {
		t.Fatalf("backup missing: %v", err)
	}

	restore()
	restored, err := os.ReadFile(cfg.SubconverterConfig())
	if err != nil {
		t.Fatal(err)
	}
	if string(restored) != string(original) {
		t.Fatalf("restored pref = %q, want %q", restored, original)
	}
	if _, err := os.Stat(cfg.SubconverterConfig() + ".clashctl.bak"); !os.IsNotExist(err) {
		t.Fatalf("backup still exists or stat failed unexpectedly: %v", err)
	}
}

func TestWriteManagedSubconverterPrefRemovesGeneratedConfig(t *testing.T) {
	base := t.TempDir()
	cfg := &config.EnvConfig{ClashBaseDir: base}
	if err := os.MkdirAll(cfg.SubconverterDir(), 0o755); err != nil {
		t.Fatal(err)
	}

	restore, err := writeManagedSubconverterPref(cfg, "32125")
	if err != nil {
		t.Fatalf("writeManagedSubconverterPref: %v", err)
	}
	if _, err := os.Stat(cfg.SubconverterConfig()); err != nil {
		t.Fatalf("generated pref missing: %v", err)
	}

	restore()
	if _, err := os.Stat(cfg.SubconverterConfig()); !os.IsNotExist(err) {
		t.Fatalf("generated pref still exists or stat failed unexpectedly: %v", err)
	}
}

func serverPort(t *testing.T, rawURL string) string {
	t.Helper()
	u, err := url.Parse(rawURL)
	if err != nil {
		t.Fatal(err)
	}
	return u.Port()
}
