package config

import (
	"os"
	"path/filepath"
	"testing"
)

func TestWithFileLockRunsCallbackAndCreatesLock(t *testing.T) {
	lockPath := filepath.Join(t.TempDir(), "locks", "profiles.lock")
	called := false

	if err := WithFileLock(lockPath, func() error {
		called = true
		return nil
	}); err != nil {
		t.Fatalf("WithFileLock: %v", err)
	}
	if !called {
		t.Fatal("callback was not called")
	}
	if _, err := os.Stat(lockPath); err != nil {
		t.Fatalf("lock file was not created: %v", err)
	}
}
