package main

import (
	"errors"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
)

func TestTunStartCanCaptureRoutes(t *testing.T) {
	if !tunStartCanCaptureRoutes(config.RuntimeInfo{TunEnabled: true, TunAutoRoute: true}, false) {
		t.Fatal("auto-route TUN should be treated as route-capturing")
	}
	if !tunStartCanCaptureRoutes(config.RuntimeInfo{TunEnabled: true, TunStrictRoute: true}, false) {
		t.Fatal("strict-route TUN should be treated as route-capturing")
	}
	if tunStartCanCaptureRoutes(config.RuntimeInfo{TunEnabled: true}, false) {
		t.Fatal("TUN without auto-route/strict-route should not be treated as route-capturing")
	}
	if !tunStartCanCaptureRoutes(config.RuntimeInfo{}, true) {
		t.Fatal("forced TUN enable should be treated as route-capturing")
	}
}

func TestEnsureSafeKernelStartFromSSHAllowsDirectMainRouteWithoutRoot(t *testing.T) {
	cfg := testSSHTunGuardConfig(t, `
mixed-port: 7897
tun:
  enable: true
  auto-route: true
`)
	t.Setenv("SSH_CONNECTION", "192.168.1.20 50000 192.168.1.23 22")
	restore := stubSSHGuard(t, 1000, func(args ...string) ([]byte, error) {
		if strings.Join(args, " ") == "-4 route get 192.168.1.20" {
			return []byte("192.168.1.20 dev wlp5s0 src 192.168.1.23 uid 1000\n"), nil
		}
		t.Fatalf("unexpected ip command: %v", args)
		return nil, nil
	})
	defer restore()

	if err := ensureSafeKernelStartFromSSH(cfg, false, false); err != nil {
		t.Fatalf("direct SSH route should be allowed without root: %v", err)
	}
}

func TestEnsureSafeKernelStartFromSSHRequiresRootForGatewayRoute(t *testing.T) {
	cfg := testSSHTunGuardConfig(t, `
tun:
  enable: true
  strict-route: true
`)
	t.Setenv("SSH_CONNECTION", "203.0.113.7 50000 192.168.1.23 22")
	restore := stubSSHGuard(t, 1000, func(args ...string) ([]byte, error) {
		if strings.Join(args, " ") == "-4 route get 203.0.113.7" {
			return []byte("203.0.113.7 via 192.168.1.1 dev wlp5s0 src 192.168.1.23 uid 1000\n"), nil
		}
		t.Fatalf("unexpected ip command: %v", args)
		return nil, nil
	})
	defer restore()

	err := ensureSafeKernelStartFromSSH(cfg, false, false)
	if err == nil {
		t.Fatal("expected gateway SSH route to require root")
	}
	if !strings.Contains(err.Error(), "run this command with sudo") {
		t.Fatalf("unexpected error: %v", err)
	}
}

func TestEnsureSafeKernelStartFromSSHInstallsBypassRouteAsRoot(t *testing.T) {
	cfg := testSSHTunGuardConfig(t, `
tun:
  enable: true
  auto-route: true
`)
	t.Setenv("SSH_CONNECTION", "203.0.113.7 50000 192.168.1.23 22")
	var commands []string
	restore := stubSSHGuard(t, 0, func(args ...string) ([]byte, error) {
		joined := strings.Join(args, " ")
		commands = append(commands, joined)
		switch joined {
		case "-4 route get 203.0.113.7":
			return []byte("203.0.113.7 via 192.168.1.1 dev wlp5s0 src 192.168.1.23 uid 0\n"), nil
		case "-4 rule del priority 8000 to 203.0.113.7/32 lookup 2021":
			return []byte("not found\n"), errors.New("not found")
		case "-4 route del 203.0.113.7/32 table 2021":
			return []byte("not found\n"), errors.New("not found")
		case "-4 route replace 203.0.113.7/32 table 2021 via 192.168.1.1 dev wlp5s0 src 192.168.1.23":
			return nil, nil
		case "-4 rule add priority 8000 to 203.0.113.7/32 lookup 2021":
			return nil, nil
		default:
			t.Fatalf("unexpected ip command: %s", joined)
			return nil, nil
		}
	})
	defer restore()

	if err := ensureSafeKernelStartFromSSH(cfg, false, false); err != nil {
		t.Fatalf("root should install bypass route: %v", err)
	}
	statePath := sshTunBypassStatePath(cfg)
	if _, err := os.Stat(statePath); err != nil {
		t.Fatalf("state file not written: %v", err)
	}
	wantRoute := "-4 route replace 203.0.113.7/32 table 2021 via 192.168.1.1 dev wlp5s0 src 192.168.1.23"
	if !containsString(commands, wantRoute) {
		t.Fatalf("missing route install command, got %#v", commands)
	}
}

func TestInstallSSHTunBypassCleansRouteWhenStateWriteFails(t *testing.T) {
	cfg := testSSHTunGuardConfig(t, "mixed-port: 7897\n")
	entry := sshTunBypassEntry{
		Family:   "ipv4",
		Prefix:   "203.0.113.7/32",
		Table:    sshTunBypassTable,
		Priority: sshTunBypassPriority,
		Route: routeInfo{
			Raw: "203.0.113.7 via 192.168.1.1 dev wlp5s0 src 192.168.1.23",
			Via: "192.168.1.1",
			Dev: "wlp5s0",
			Src: "192.168.1.23",
		},
	}

	var commands []string
	restore := stubSSHGuard(t, 0, func(args ...string) ([]byte, error) {
		joined := strings.Join(args, " ")
		commands = append(commands, joined)
		switch joined {
		case "-4 rule del priority 8000 to 203.0.113.7/32 lookup 2021":
			return nil, nil
		case "-4 route del 203.0.113.7/32 table 2021":
			return nil, nil
		case "-4 route replace 203.0.113.7/32 table 2021 via 192.168.1.1 dev wlp5s0 src 192.168.1.23":
			return nil, nil
		case "-4 rule add priority 8000 to 203.0.113.7/32 lookup 2021":
			return nil, nil
		default:
			t.Fatalf("unexpected ip command: %s", joined)
			return nil, nil
		}
	})
	defer restore()
	writeSSHTunBypassStateFile = func(string, []byte, os.FileMode) error {
		return errors.New("forced state write failure")
	}

	err := installSSHTunBypass(cfg, entry)
	if err == nil || !strings.Contains(err.Error(), "write SSH bypass state") {
		t.Fatalf("installSSHTunBypass error = %v, want state write failure", err)
	}
	wantCleanupRule := "-4 rule del priority 8000 to 203.0.113.7/32 lookup 2021"
	if countString(commands, wantCleanupRule) != 2 {
		t.Fatalf("cleanup rule command count = %d, want 2; commands=%#v", countString(commands, wantCleanupRule), commands)
	}
}

func TestBuildSSHTunBypassEntryFallsBackToMainTableWhenPolicyRouteUsesTunnel(t *testing.T) {
	restore := stubSSHGuard(t, 0, func(args ...string) ([]byte, error) {
		switch strings.Join(args, " ") {
		case "-4 route get 203.0.113.7":
			return []byte("203.0.113.7 via 198.18.0.2 dev SakuraiTunnel table 2022 src 198.18.0.1 uid 0\n"), nil
		case "-4 route show table main match 203.0.113.7":
			return []byte("default via 192.168.1.1 dev wlp5s0 proto dhcp src 192.168.1.23 metric 600\n"), nil
		default:
			t.Fatalf("unexpected ip command: %v", args)
			return nil, nil
		}
	})
	defer restore()

	entry, err := buildSSHTunBypassEntry("203.0.113.7")
	if err != nil {
		t.Fatalf("buildSSHTunBypassEntry: %v", err)
	}
	if entry.Route.Dev != "wlp5s0" || entry.Route.Via != "192.168.1.1" {
		t.Fatalf("route = %#v, want physical gateway route", entry.Route)
	}
}

func TestCurrentSSHSessionFallsBackToProcessTree(t *testing.T) {
	t.Setenv("SSH_CONNECTION", "")
	t.Setenv("SSH_TTY", "")

	files := map[string][]byte{
		"/proc/10/environ": []byte("SUDO_USER=yequdesu\x00"),
		"/proc/10/status":  []byte("Name:\tsudo\nPPid:\t20\n"),
		"/proc/20/environ": []byte("SSH_CONNECTION=192.168.1.20 50000 192.168.1.23 22\x00SSH_TTY=/dev/pts/3\x00"),
		"/proc/20/status":  []byte("Name:\tzsh\nPPid:\t1\n"),
	}

	restore := stubSSHGuard(t, 0, func(args ...string) ([]byte, error) {
		t.Fatalf("route lookup should not run in this test: %v", args)
		return nil, nil
	})
	defer restore()
	sshGuardGetppid = func() int { return 10 }
	sshGuardReadFile = func(path string) ([]byte, error) {
		data, ok := files[path]
		if !ok {
			return nil, os.ErrNotExist
		}
		return data, nil
	}

	session := currentSSHSession()
	if !session.Active || session.Client != "192.168.1.20" || session.Server != "192.168.1.23" {
		t.Fatalf("session = %#v", session)
	}
}

func TestEnsureSafeKernelStartFromSSHAllowsExplicitOverride(t *testing.T) {
	cfg := testSSHTunGuardConfig(t, `
tun:
  enable: true
  strict-route: true
`)
	t.Setenv("SSH_TTY", "/dev/pts/1")
	restore := stubSSHGuard(t, 1000, func(args ...string) ([]byte, error) {
		t.Fatalf("override should not inspect routes: %v", args)
		return nil, nil
	})
	defer restore()

	if err := ensureSafeKernelStartFromSSH(cfg, true, false); err != nil {
		t.Fatalf("override should allow risky start: %v", err)
	}
}

func testSSHTunGuardConfig(t *testing.T, runtimeYAML string) *config.EnvConfig {
	t.Helper()
	base := t.TempDir()
	resources := filepath.Join(base, "resources")
	if err := os.MkdirAll(resources, 0755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(resources, "runtime.yaml"), []byte(runtimeYAML), 0644); err != nil {
		t.Fatal(err)
	}
	return &config.EnvConfig{ClashBaseDir: base, KernelName: "mihomo"}
}

func stubSSHGuard(t *testing.T, uid int, run func(args ...string) ([]byte, error)) func() {
	t.Helper()
	oldRun := runSSHGuardIPCommand
	oldUID := sshGuardGetuid
	oldPPID := sshGuardGetppid
	oldReadFile := sshGuardReadFile
	oldWriteState := writeSSHTunBypassStateFile
	runSSHGuardIPCommand = run
	sshGuardGetuid = func() int { return uid }
	return func() {
		runSSHGuardIPCommand = oldRun
		sshGuardGetuid = oldUID
		sshGuardGetppid = oldPPID
		sshGuardReadFile = oldReadFile
		writeSSHTunBypassStateFile = oldWriteState
	}
}

func containsString(values []string, want string) bool {
	for _, value := range values {
		if value == want {
			return true
		}
	}
	return false
}

func countString(values []string, want string) int {
	count := 0
	for _, value := range values {
		if value == want {
			count++
		}
	}
	return count
}
