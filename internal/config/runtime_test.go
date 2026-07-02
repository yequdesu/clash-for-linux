package config

import "testing"

func TestParseRuntimeInfo(t *testing.T) {
	info, err := ParseRuntimeInfo([]byte(`
mixed-port: 7890
port: "7892"
external-controller: "127.0.0.1:9090"
secret: "abc"
dns:
  enable: true
  enhanced-mode: fake-ip
  fake-ip-filter:
    - localhost
    - "*.lan"
tun:
  enable: true
  stack: gvisor
  auto-route: true
  strict-route: true
  device: SakuraiTunnel
`))
	if err != nil {
		t.Fatalf("ParseRuntimeInfo: %v", err)
	}
	if info.ProxyPort != "7890" {
		t.Fatalf("ProxyPort = %q, want 7890", info.ProxyPort)
	}
	if info.APIPort != "9090" || info.APIHost != "127.0.0.1" {
		t.Fatalf("API = %q:%q, want 127.0.0.1:9090", info.APIHost, info.APIPort)
	}
	if info.Secret != "abc" {
		t.Fatalf("Secret = %q, want abc", info.Secret)
	}
	if !info.DNSEnabled || info.DNSEnhancedMode != "fake-ip" {
		t.Fatalf("DNS = %v/%q, want enabled/fake-ip", info.DNSEnabled, info.DNSEnhancedMode)
	}
	if len(info.FakeIPFilter) != 2 || info.FakeIPFilter[1] != "*.lan" {
		t.Fatalf("FakeIPFilter = %#v", info.FakeIPFilter)
	}
	if !info.TunEnabled {
		t.Fatal("TunEnabled = false, want true")
	}
	if info.TunStack != "gvisor" || !info.TunAutoRoute || !info.TunStrictRoute || info.TunDevice != "SakuraiTunnel" {
		t.Fatalf("Tun details = stack %q auto %v strict %v device %q", info.TunStack, info.TunAutoRoute, info.TunStrictRoute, info.TunDevice)
	}
}

func TestParseRuntimeInfoIPv6Controller(t *testing.T) {
	info, err := ParseRuntimeInfo([]byte(`external-controller: "[::1]:9090"`))
	if err != nil {
		t.Fatalf("ParseRuntimeInfo: %v", err)
	}
	if info.APIHost != "::1" || info.APIPort != "9090" {
		t.Fatalf("API = %q:%q, want ::1:9090", info.APIHost, info.APIPort)
	}
	if got := info.APIBaseURL("9090"); got != "http://[::1]:9090" {
		t.Fatalf("APIBaseURL = %q, want http://[::1]:9090", got)
	}
}

func TestParseRuntimeInfoFallsBackToPort(t *testing.T) {
	info, err := ParseRuntimeInfo([]byte(`port: 7892`))
	if err != nil {
		t.Fatalf("ParseRuntimeInfo: %v", err)
	}
	if info.ProxyPort != "7892" {
		t.Fatalf("ProxyPort = %q, want 7892", info.ProxyPort)
	}
}

func TestRuntimeInfoAPIAddress(t *testing.T) {
	tests := []struct {
		name string
		info RuntimeInfo
		want string
	}{
		{
			name: "explicit loopback",
			info: RuntimeInfo{APIHost: "127.0.0.1", APIPort: "9090"},
			want: "127.0.0.1:9090",
		},
		{
			name: "default port",
			info: RuntimeInfo{APIHost: "127.0.0.1"},
			want: "127.0.0.1:9090",
		},
		{
			name: "localhost preserved",
			info: RuntimeInfo{APIHost: "localhost", APIPort: "9091"},
			want: "localhost:9091",
		},
		{
			name: "ipv4 unspecified dials loopback",
			info: RuntimeInfo{APIHost: "0.0.0.0", APIPort: "9090"},
			want: "127.0.0.1:9090",
		},
		{
			name: "ipv6 unspecified dials loopback",
			info: RuntimeInfo{APIHost: "::", APIPort: "9090"},
			want: "[::1]:9090",
		},
		{
			name: "ipv6 loopback",
			info: RuntimeInfo{APIHost: "::1", APIPort: "9090"},
			want: "[::1]:9090",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if got := tt.info.APIAddress("9090"); got != tt.want {
				t.Fatalf("APIAddress = %q, want %q", got, tt.want)
			}
		})
	}
}
