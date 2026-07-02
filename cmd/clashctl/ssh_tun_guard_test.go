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

func TestEnsureSafeKernelStartProtectsDetectedRoutesAsRoot(t *testing.T) {
	cfg := testSSHTunGuardConfig(t, `
tun:
  enable: true
  auto-route: true
`)
	var commands []string
	restore := stubSSHGuard(t, 0, func(args ...string) ([]byte, error) {
		joined := strings.Join(args, " ")
		commands = append(commands, joined)
		switch joined {
		case "-4 rule del priority 8000 to 10.0.0.0/24 lookup 2021",
			"-4 route del 10.0.0.0/24 table 2021",
			"-4 rule del priority 8000 to 118.178.171.166/32 lookup 2021",
			"-4 route del 118.178.171.166/32 table 2021":
			return []byte("not found\n"), errors.New("not found")
		case "-4 route replace 10.0.0.0/24 table 2021 dev wg0",
			"-4 rule add priority 8000 to 10.0.0.0/24 lookup 2021",
			"-4 route replace 118.178.171.166/32 table 2021 via 192.168.1.1 dev wlp5s0 src 192.168.1.23",
			"-4 rule add priority 8000 to 118.178.171.166/32 lookup 2021":
			return nil, nil
		default:
			t.Fatalf("unexpected ip command: %s", joined)
			return nil, nil
		}
	})
	defer restore()
	detectConnectedRouteEntries = func() ([]sshTunBypassEntry, []string) {
		return []sshTunBypassEntry{{
			Family:   "ipv4",
			Prefix:   "10.0.0.0/24",
			Table:    sshTunBypassTable,
			Priority: sshTunBypassPriority,
			Route:    routeInfo{Raw: "10.0.0.0/24 dev wg0", Dev: "wg0"},
			Source:   "connected-route",
			Reason:   "protect connected tunnel route",
		}}, nil
	}
	detectWireGuardRouteEntries = func() ([]sshTunBypassEntry, []string) {
		return []sshTunBypassEntry{{
			Family:   "ipv4",
			Prefix:   "118.178.171.166/32",
			Table:    sshTunBypassTable,
			Priority: sshTunBypassPriority,
			Route: routeInfo{
				Raw: "118.178.171.166 via 192.168.1.1 dev wlp5s0 src 192.168.1.23",
				Via: "192.168.1.1",
				Dev: "wlp5s0",
				Src: "192.168.1.23",
			},
			Source: "tunnel-endpoint",
			Reason: "protect detected tunnel endpoint",
		}}, nil
	}

	if err := ensureSafeKernelStartFromSSH(cfg, false, false); err != nil {
		t.Fatalf("root should install detected route protections: %v", err)
	}

	data, err := os.ReadFile(sshTunBypassStatePath(cfg))
	if err != nil {
		t.Fatalf("state file not written: %v", err)
	}
	if !strings.Contains(string(data), `"source": "connected-route"`) || !strings.Contains(string(data), `"source": "tunnel-endpoint"`) {
		t.Fatalf("state file missing route sources:\n%s", string(data))
	}
	if !containsString(commands, "-4 route replace 10.0.0.0/24 table 2021 dev wg0") {
		t.Fatalf("missing connected route protection install, commands=%#v", commands)
	}
	if !containsString(commands, "-4 route replace 118.178.171.166/32 table 2021 via 192.168.1.1 dev wlp5s0 src 192.168.1.23") {
		t.Fatalf("missing endpoint route protection install, commands=%#v", commands)
	}
}

func TestEnsureSafeKernelStartRequiresRootForDetectedTunnelRoute(t *testing.T) {
	cfg := testSSHTunGuardConfig(t, `
tun:
  enable: true
  auto-route: true
`)
	restore := stubSSHGuard(t, 1000, func(args ...string) ([]byte, error) {
		t.Fatalf("detector-provided entries should not need ip commands in this test: %v", args)
		return nil, nil
	})
	defer restore()
	detectConnectedRouteEntries = func() ([]sshTunBypassEntry, []string) {
		return []sshTunBypassEntry{{
			Family:   "ipv4",
			Prefix:   "10.0.0.0/24",
			Table:    sshTunBypassTable,
			Priority: sshTunBypassPriority,
			Route:    routeInfo{Raw: "10.0.0.0/24 dev wg0", Dev: "wg0"},
			Source:   "connected-route",
			Reason:   "protect connected tunnel route",
		}}, nil
	}

	err := ensureSafeKernelStartFromSSH(cfg, false, false)
	if err == nil {
		t.Fatal("expected detected tunnel route to require root")
	}
	if !strings.Contains(err.Error(), "--allow-route-risk") || !strings.Contains(err.Error(), "CLASH_TUN_PROTECT_ROUTES") {
		t.Fatalf("unexpected guidance: %v", err)
	}
}

func TestManualRouteProtectionEntriesParsesCIDRs(t *testing.T) {
	t.Setenv("CLASH_TUN_PROTECT_ROUTES", "10.0.0.0/24,192.168.50.7;bad")
	restore := stubSSHGuard(t, 0, func(args ...string) ([]byte, error) {
		switch strings.Join(args, " ") {
		case "-4 route get 10.0.0.0":
			return []byte("10.0.0.0 dev wg0 src 10.0.0.2\n"), nil
		case "-4 route get 192.168.50.7":
			return []byte("192.168.50.7 via 192.168.1.1 dev wlp5s0 src 192.168.1.23\n"), nil
		default:
			t.Fatalf("unexpected ip command: %v", args)
			return nil, nil
		}
	})
	defer restore()

	entries, risks := manualRouteProtectionEntries()
	if len(entries) != 2 {
		t.Fatalf("entries = %#v, want 2", entries)
	}
	if entries[0].Prefix != "10.0.0.0/24" || entries[1].Prefix != "192.168.50.7/32" {
		t.Fatalf("prefixes = %s, %s", entries[0].Prefix, entries[1].Prefix)
	}
	if len(risks) != 1 || !strings.Contains(risks[0], `invalid manual protected route "bad"`) {
		t.Fatalf("risks = %#v, want invalid token risk", risks)
	}
}

func TestManualRouteProtectionRejectsDefaultRoute(t *testing.T) {
	t.Setenv("CLASH_TUN_PROTECT_ROUTES", "0.0.0.0/0")
	restore := stubSSHGuard(t, 0, func(args ...string) ([]byte, error) {
		t.Fatalf("default protected route should not inspect ip route: %v", args)
		return nil, nil
	})
	defer restore()

	entries, risks := manualRouteProtectionEntries()
	if len(entries) != 0 {
		t.Fatalf("entries = %#v, want none", entries)
	}
	if len(risks) != 1 || !strings.Contains(risks[0], "default routes cannot be protected") {
		t.Fatalf("risks = %#v, want default route rejection", risks)
	}
}

func TestDetectConnectedRouteProtectionEntriesSelectsTunnelInterfaces(t *testing.T) {
	restore := stubSSHGuard(t, 0, func(args ...string) ([]byte, error) {
		switch strings.Join(args, " ") {
		case "-4 route show table main scope link":
			return []byte(strings.Join([]string{
				"10.0.0.0/24 dev wg0 proto kernel scope link src 10.0.0.2",
				"172.17.0.0/16 dev docker0 proto kernel scope link src 172.17.0.1",
				"192.168.1.0/24 dev wlp5s0 proto kernel scope link src 192.168.1.23",
				"198.18.0.0/15 dev SakuraiTunnel proto kernel scope link src 198.18.0.1",
			}, "\n")), nil
		case "-6 route show table main scope link":
			return []byte("fd00::/64 dev tailscale0 proto kernel metric 256 pref medium\n"), nil
		default:
			t.Fatalf("unexpected ip command: %v", args)
			return nil, nil
		}
	})
	defer restore()

	entries, risks := detectConnectedRouteProtectionEntries()
	if len(risks) != 0 {
		t.Fatalf("risks = %#v, want none", risks)
	}
	if len(entries) != 2 {
		t.Fatalf("entries = %#v, want wg0 and tailscale0 routes", entries)
	}
	if entries[0].Prefix != "10.0.0.0/24" || entries[0].Route.Dev != "wg0" {
		t.Fatalf("first entry = %#v, want wg0 route", entries[0])
	}
	if entries[1].Prefix != "fd00::/64" || entries[1].Route.Dev != "tailscale0" {
		t.Fatalf("second entry = %#v, want tailscale0 route", entries[1])
	}
}

func TestEndpointHost(t *testing.T) {
	tests := []struct {
		endpoint string
		want     string
	}{
		{endpoint: "118.178.171.166:51820", want: "118.178.171.166"},
		{endpoint: "[2001:db8::1]:51820", want: "2001:db8::1"},
		{endpoint: "vpn.example.test:51820", want: "vpn.example.test"},
		{endpoint: "(none)", want: ""},
	}
	for _, tt := range tests {
		t.Run(tt.endpoint, func(t *testing.T) {
			if got := endpointHost(tt.endpoint); got != tt.want {
				t.Fatalf("endpointHost(%q) = %q, want %q", tt.endpoint, got, tt.want)
			}
		})
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
	if err == nil || !strings.Contains(err.Error(), "write route guard state") {
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
	oldConnectedRoutes := detectConnectedRouteEntries
	oldWireGuardRoutes := detectWireGuardRouteEntries
	runSSHGuardIPCommand = run
	sshGuardGetuid = func() int { return uid }
	detectConnectedRouteEntries = func() ([]sshTunBypassEntry, []string) { return nil, nil }
	detectWireGuardRouteEntries = func() ([]sshTunBypassEntry, []string) { return nil, nil }
	return func() {
		runSSHGuardIPCommand = oldRun
		sshGuardGetuid = oldUID
		sshGuardGetppid = oldPPID
		sshGuardReadFile = oldReadFile
		writeSSHTunBypassStateFile = oldWriteState
		detectConnectedRouteEntries = oldConnectedRoutes
		detectWireGuardRouteEntries = oldWireGuardRoutes
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
