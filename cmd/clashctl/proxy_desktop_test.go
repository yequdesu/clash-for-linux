package main

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
)

func TestDesktopProxySettingsFromConfig(t *testing.T) {
	baseDir := t.TempDir()
	resources := filepath.Join(baseDir, "resources")
	if err := os.MkdirAll(resources, 0o755); err != nil {
		t.Fatal(err)
	}
	runtime := []byte(`
mixed-port: 7897
socks-port: 7898
`)
	if err := os.WriteFile(filepath.Join(resources, "runtime.yaml"), runtime, 0o644); err != nil {
		t.Fatal(err)
	}

	settings := desktopProxySettingsFromConfig(&config.EnvConfig{ClashBaseDir: baseDir})
	if settings.httpPort != "7897" {
		t.Fatalf("httpPort = %q, want 7897", settings.httpPort)
	}
	if settings.socksPort != "7898" {
		t.Fatalf("socksPort = %q, want 7898", settings.socksPort)
	}
	if settings.host != "127.0.0.1" {
		t.Fatalf("host = %q, want 127.0.0.1", settings.host)
	}
}

func TestDesktopProxySettingsFallbacks(t *testing.T) {
	settings := desktopProxySettingsFromConfig(&config.EnvConfig{ClashBaseDir: t.TempDir()})
	if settings.httpPort != "7890" {
		t.Fatalf("httpPort = %q, want 7890", settings.httpPort)
	}
	if settings.socksPort != "7890" {
		t.Fatalf("socksPort = %q, want 7890", settings.socksPort)
	}
}

func TestDesktopProxyCommandBuilders(t *testing.T) {
	settings := desktopProxySettings{
		host:      "127.0.0.1",
		httpPort:  "7897",
		socksPort: "7898",
		noProxy:   "localhost,127.0.0.1,::1",
	}

	gnome := gnomeProxyOnCommands(settings)
	if len(gnome) < 10 {
		t.Fatalf("gnome commands len = %d", len(gnome))
	}
	assertCommandArg(t, gnome, "org.gnome.system.proxy.http", "7897")
	assertCommandArg(t, gnome, "org.gnome.system.proxy.socks", "7898")

	kde := kdeProxyOnCommands("kwriteconfig6", settings)
	if len(kde) < 6 {
		t.Fatalf("kde commands len = %d", len(kde))
	}
	assertCommandArg(t, kde, "httpProxy", "http://127.0.0.1 7897")
	assertCommandArg(t, kde, "socksProxy", "socks://127.0.0.1 7898")
	assertCommandArg(t, kde, "NoProxyFor", settings.noProxy)
}

func assertCommandArg(t *testing.T, commands []desktopProxyCommand, key, value string) {
	t.Helper()
	for _, cmd := range commands {
		hasKey := false
		hasValue := false
		for _, arg := range cmd.args {
			if arg == key {
				hasKey = true
			}
			if arg == value {
				hasValue = true
			}
		}
		if hasKey && hasValue {
			return
		}
	}
	t.Fatalf("command args missing key=%q value=%q: %#v", key, value, commands)
}
