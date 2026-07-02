package main

import (
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"testing"
)

func TestCLIProcessExitCodes(t *testing.T) {
	bin := buildClashctlForTest(t)

	run := func(t *testing.T, base string, want int, args ...string) {
		t.Helper()
		cmd := exec.Command(bin, args...)
		cmd.Env = append(os.Environ(),
			"CLASH_BASE_DIR="+base,
			"KERNEL_NAME=mihomo",
			"SERVICE_NAME=clashctl",
			"INIT_TYPE=nohup",
		)
		out, err := cmd.CombinedOutput()
		got := 0
		if err != nil {
			exitErr, ok := err.(*exec.ExitError)
			if !ok {
				t.Fatalf("run %v failed without exit code: %v\n%s", args, err, out)
			}
			got = exitErr.ExitCode()
		}
		if got != want {
			t.Fatalf("run %v exit code = %d, want %d\n%s", args, got, want, out)
		}
	}

	t.Run("parent help exits nonzero", func(t *testing.T) {
		base := testInstalledBase(t)
		for _, args := range [][]string{
			nil,
			{"config"},
			{"sub"},
			{"node"},
			{"geodata"},
		} {
			run(t, base, 1, args...)
		}
	})

	t.Run("status commands still allow empty state", func(t *testing.T) {
		base := testInstalledBase(t)
		run(t, base, 0, "sub", "list")
		run(t, base, 0, "sub", "log")
		run(t, base, 0, "proxy", "on")
		run(t, base, 0, "proxy", "off")
	})

	t.Run("invalid or failed commands exit nonzero", func(t *testing.T) {
		base := testInstalledBase(t)
		run(t, base, 1, "proxy", "invalid")
		run(t, base, 1, "secret", "show")
		run(t, base, 1, "sub", "update")
		run(t, base, 1, "sub", "update", "not-a-number")
	})

	t.Run("profile read errors exit nonzero", func(t *testing.T) {
		base := testInstalledBase(t)
		profilesMeta := filepath.Join(base, "resources", "profiles.yaml")
		if err := os.WriteFile(profilesMeta, []byte("profiles: [\n"), 0o644); err != nil {
			t.Fatal(err)
		}
		run(t, base, 1, "sub", "list")

		base = testInstalledBase(t)
		profilesMeta = filepath.Join(base, "resources", "profiles.yaml")
		if err := os.Mkdir(profilesMeta, 0o755); err != nil {
			t.Fatal(err)
		}
		run(t, base, 1, "sub", "list")
	})
}

func buildClashctlForTest(t *testing.T) string {
	t.Helper()
	name := "clashctl"
	if runtime.GOOS == "windows" {
		name += ".exe"
	}
	bin := filepath.Join(t.TempDir(), name)
	cmd := exec.Command("go", "build", "-trimpath", "-o", bin, ".")
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("build clashctl: %v\n%s", err, out)
	}
	return bin
}

func testInstalledBase(t *testing.T) string {
	t.Helper()
	base := t.TempDir()
	for _, dir := range []string{
		filepath.Join(base, "resources"),
		filepath.Join(base, "logs"),
	} {
		if err := os.MkdirAll(dir, 0o755); err != nil {
			t.Fatal(err)
		}
	}
	if err := os.WriteFile(filepath.Join(base, "resources", "runtime.yaml"), []byte("mixed-port: 7890\nexternal-controller: 127.0.0.1:9090\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	return base
}
