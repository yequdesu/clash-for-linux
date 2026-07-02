package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestWriteSubLogWritesTimestampedEntry(t *testing.T) {
	path := filepath.Join(t.TempDir(), "profiles.log")

	if err := writeSubLog(path, "updated: [1]"); err != nil {
		t.Fatalf("writeSubLog: %v", err)
	}
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	if got := string(data); !strings.Contains(got, "updated: [1]") {
		t.Fatalf("log = %q, want message", got)
	}
}

func TestWriteSubLogReturnsOpenError(t *testing.T) {
	path := filepath.Join(t.TempDir(), "missing", "profiles.log")

	if err := writeSubLog(path, "updated: [1]"); err == nil {
		t.Fatal("writeSubLog error = nil, want open error")
	}
}
