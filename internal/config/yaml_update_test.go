package config

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestUpdateYAMLPreservesUnknownKeysAndSetsNestedValues(t *testing.T) {
	path := filepath.Join(t.TempDir(), "mixin.yaml")
	input := []byte("mode: rule\nunknown:\n  keep: yes\n")
	if err := os.WriteFile(path, input, 0o644); err != nil {
		t.Fatal(err)
	}

	err := UpdateYAML(path, map[string]any{
		"mixed-port":        7890,
		"dns.enable":        true,
		"dns.enhanced-mode": "fake-ip",
		"dns.fake-ip-filter": []string{
			"localhost",
			"*.local",
		},
	})
	if err != nil {
		t.Fatal(err)
	}

	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	out := string(data)
	for _, want := range []string{
		"mode: rule",
		"keep: yes",
		"mixed-port: 7890",
		"enhanced-mode: fake-ip",
		"*.local",
	} {
		if !strings.Contains(out, want) {
			t.Fatalf("updated yaml missing %q:\n%s", want, out)
		}
	}
}
