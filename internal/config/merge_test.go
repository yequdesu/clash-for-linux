package config

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestApplyProxyGroupFixOnlyChangesMatchRules(t *testing.T) {
	path := filepath.Join(t.TempDir(), "runtime.yaml")
	input := []byte(`
rules:
  - DOMAIN-SUFFIX,example.com,DIRECT
  - MATCH,MissingGroup
  - DOMAIN-KEYWORD,MATCH,MissingGroup,DIRECT
`)
	if err := os.WriteFile(path, input, 0644); err != nil {
		t.Fatalf("write input: %v", err)
	}

	if err := applyProxyGroupFix(path, "MissingGroup", "Proxy"); err != nil {
		t.Fatalf("applyProxyGroupFix: %v", err)
	}

	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read output: %v", err)
	}
	out := string(data)
	if !strings.Contains(out, "MATCH,Proxy") {
		t.Fatalf("fixed MATCH rule not found in:\n%s", out)
	}
	if !strings.Contains(out, "DOMAIN-KEYWORD,MATCH,MissingGroup,DIRECT") {
		t.Fatalf("non-MATCH rule was unexpectedly changed:\n%s", out)
	}
}

func TestWriteRuntimeCandidateUsesUniqueTempFiles(t *testing.T) {
	base := t.TempDir()
	cfg := &EnvConfig{ClashBaseDir: base}

	first, err := writeRuntimeCandidate(cfg, []byte("mixed-port: 7890\n"))
	if err != nil {
		t.Fatalf("writeRuntimeCandidate first: %v", err)
	}
	defer os.Remove(first)
	second, err := writeRuntimeCandidate(cfg, []byte("mixed-port: 7891\n"))
	if err != nil {
		t.Fatalf("writeRuntimeCandidate second: %v", err)
	}
	defer os.Remove(second)

	if first == second {
		t.Fatalf("temp paths are equal: %s", first)
	}
	if filepath.Dir(first) != cfg.ResourcesDir() || filepath.Dir(second) != cfg.ResourcesDir() {
		t.Fatalf("temp files must be in resources dir: %s %s", first, second)
	}
	if filepath.Base(first) == "runtime.yaml.tmp" || filepath.Base(second) == "runtime.yaml.tmp" {
		t.Fatalf("fixed runtime temp name was used: %s %s", first, second)
	}
	data, err := os.ReadFile(second)
	if err != nil {
		t.Fatalf("read second temp: %v", err)
	}
	if string(data) != "mixed-port: 7891\n" {
		t.Fatalf("second temp content = %q", data)
	}
}
