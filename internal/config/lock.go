package config

import (
	"os"
	"path/filepath"
)

func WithFileLock(path string, fn func() error) error {
	if err := os.MkdirAll(filepath.Dir(path), 0755); err != nil {
		return err
	}

	f, err := os.OpenFile(path, os.O_CREATE|os.O_RDWR, 0644)
	if err != nil {
		return err
	}
	defer f.Close()

	if err := lockFile(f); err != nil {
		return err
	}
	defer unlockFile(f)

	return fn()
}

func WithProfilesLock(cfg *EnvConfig, fn func() error) error {
	return WithFileLock(filepath.Join(cfg.ResourcesDir(), ".profiles.lock"), fn)
}
