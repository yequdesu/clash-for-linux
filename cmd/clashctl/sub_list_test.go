package main

import (
	"os"
	"path/filepath"
	"testing"
)

func TestCountProxiesParsesStandardClashYAML(t *testing.T) {
	path := filepath.Join(t.TempDir(), "profile.yaml")
	data := []byte(`proxies:
  - name: A
    type: ss
    server: 127.0.0.1
  - name: B
    type: vmess
    server: 127.0.0.2
proxy-groups:
  - name: Auto
    type: select
    proxies:
      - A
      - B
`)
	if err := os.WriteFile(path, data, 0o644); err != nil {
		t.Fatal(err)
	}

	if got := countProxies(path); got != "2" {
		t.Fatalf("countProxies = %q, want 2", got)
	}
}

func TestCountProxiesReportsUnknownOnParseError(t *testing.T) {
	path := filepath.Join(t.TempDir(), "profile.yaml")
	if err := os.WriteFile(path, []byte("proxies: ["), 0o644); err != nil {
		t.Fatal(err)
	}

	if got := countProxies(path); got != "unknown" {
		t.Fatalf("countProxies = %q, want unknown", got)
	}
}
