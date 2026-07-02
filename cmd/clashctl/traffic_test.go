package main

import (
	"fmt"
	"net/http"
	"net/http/httptest"
	"os"
	"strings"
	"testing"
	"time"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
	"github.com/yequdesu/linux-cli-tui-clash/internal/traffic"
)

func TestCollectTrafficSampleWritesStore(t *testing.T) {
	mux := http.NewServeMux()
	mux.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/" {
			http.NotFound(w, r)
			return
		}
		_, _ = w.Write([]byte(`{"hello":"mihomo"}`))
	})
	mux.HandleFunc("/traffic", func(w http.ResponseWriter, r *http.Request) {
		_, _ = w.Write([]byte(`{"up":1024,"down":2048}`))
	})
	mux.HandleFunc("/connections", func(w http.ResponseWriter, r *http.Request) {
		_, _ = w.Write([]byte(`{
			"uploadTotal": 1000,
			"downloadTotal": 2000,
			"connections": [{
				"id": "c1",
				"upload": 10,
				"download": 20,
				"rule": "Proxy",
				"rulePayload": "geosite:google",
				"chains": ["Proxy", "HK-01"],
				"metadata": {"host": "google.com", "network": "tcp", "process": "curl"}
			}]
		}`))
	})
	server := httptest.NewServer(mux)
	defer server.Close()

	addr := strings.TrimPrefix(server.URL, "http://")
	base := t.TempDir()
	testCfg := &config.EnvConfig{ClashBaseDir: base, KernelName: "missing-kernel"}
	if err := os.MkdirAll(testCfg.ResourcesDir(), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(testCfg.RuntimePath(), []byte(fmt.Sprintf("external-controller: %q\n", addr)), 0o644); err != nil {
		t.Fatal(err)
	}
	oldNow := timeNow
	timeNow = func() time.Time { return time.Date(2026, 7, 2, 12, 0, 0, 0, time.UTC) }
	defer func() { timeNow = oldNow }()

	sample, err := collectTrafficSample(testCfg)
	if err != nil {
		t.Fatalf("collectTrafficSample: %v", err)
	}
	if sample.DownBPS != 2048 || sample.UpBPS != 1024 || sample.ActiveConnections != 1 {
		t.Fatalf("sample = %+v", sample)
	}
	store := traffic.NewConfigStore(testCfg)
	loaded, err := store.LoadSamples(time.Time{}, time.Time{})
	if err != nil {
		t.Fatal(err)
	}
	if len(loaded) != 1 {
		t.Fatalf("loaded sample count = %d, want 1", len(loaded))
	}
	rollup10s, err := store.LoadRollups(10*time.Second, time.Time{}, time.Time{})
	if err != nil {
		t.Fatal(err)
	}
	rollup1m, err := store.LoadRollups(time.Minute, time.Time{}, time.Time{})
	if err != nil {
		t.Fatal(err)
	}
	if len(rollup10s) != 1 || len(rollup1m) != 1 {
		t.Fatalf("rollup counts 10s=%d 1m=%d, want 1/1", len(rollup10s), len(rollup1m))
	}
	rows := traffic.Top(loaded, "route", 1)
	if len(rows) != 1 || !strings.Contains(rows[0].Key, "Proxy/geosite:google") {
		t.Fatalf("route rows = %+v", rows)
	}

	status, err := trafficStatusSnapshot(testCfg)
	if err != nil {
		t.Fatalf("trafficStatusSnapshot: %v", err)
	}
	if status.RawSamples != 1 || status.Rollup10s != 1 || status.Rollup1m != 1 {
		t.Fatalf("status counts = %+v, want raw/10s/1m = 1", status)
	}
	if status.LastSample != "2026-07-02 12:00:00" || status.TrackedConnections != 1 {
		t.Fatalf("status last/tracked = %+v", status)
	}
}

func TestFilterTrafficSamplesForExportKeepsRequestedDimension(t *testing.T) {
	samples := []traffic.Sample{{
		TS: time.Date(2026, 7, 2, 12, 0, 0, 0, time.UTC),
		Breakdown: []traffic.Breakdown{
			{Dimension: "route", Key: "rule -> proxy", DownloadDelta: 10},
			{Dimension: "node", Key: "hk-1", DownloadDelta: 10},
			{Dimension: "total", Key: "total", DownloadDelta: 10},
		},
	}}

	filtered := filterTrafficSamplesForExport(samples, "node")
	if len(filtered) != 1 || len(filtered[0].Breakdown) != 1 {
		t.Fatalf("filtered samples = %+v", filtered)
	}
	if filtered[0].Breakdown[0].Dimension != "node" || filtered[0].Breakdown[0].Key != "hk-1" {
		t.Fatalf("filtered breakdown = %+v", filtered[0].Breakdown[0])
	}

	unfiltered := filterTrafficSamplesForExport(samples, "")
	if len(unfiltered) != 1 || len(unfiltered[0].Breakdown) != 3 {
		t.Fatalf("unfiltered samples = %+v", unfiltered)
	}
}

func TestTrafficCollectorArgsAndEnvAreDeterministic(t *testing.T) {
	args := trafficCollectorArgs(2 * time.Second)
	want := []string{"traffic", "collect", "--daemon", "--interval", "2s"}
	if strings.Join(args, " ") != strings.Join(want, " ") {
		t.Fatalf("collector args = %v, want %v", args, want)
	}

	env := strings.Join(trafficCollectorEnv(&config.EnvConfig{
		ClashBaseDir: "/opt/clashctl",
		KernelName:   "mihomo",
		ServiceName:  "clashctl",
		InitType:     "systemd",
	}), "\n")
	for _, want := range []string{
		"CLASH_BASE_DIR=/opt/clashctl",
		"KERNEL_NAME=mihomo",
		"SERVICE_NAME=clashctl",
		"CLASH_SERVICE_NAME=clashctl",
		"INIT_TYPE=systemd",
	} {
		if !strings.Contains(env, want) {
			t.Fatalf("collector env missing %q in:\n%s", want, env)
		}
	}
}

func TestTrafficCollectorSnapshotDetectsStoppedAndStaleState(t *testing.T) {
	store := traffic.NewStore(t.TempDir())
	status, err := trafficCollectorSnapshot(store)
	if err != nil {
		t.Fatalf("trafficCollectorSnapshot stopped: %v", err)
	}
	if status.Running || status.Stale || status.PID != 0 {
		t.Fatalf("empty collector status = %+v, want stopped", status)
	}
	if status.LogPath != store.CollectorLogPath() {
		t.Fatalf("log path = %q, want %q", status.LogPath, store.CollectorLogPath())
	}

	started := time.Date(2026, 7, 2, 12, 0, 0, 0, time.UTC)
	if err := saveTrafficCollectorState(store, trafficCollectorState{
		PID:       2147483647,
		StartedAt: started,
		Interval:  "1s",
		LogPath:   store.CollectorLogPath(),
		Command:   []string{"clashctl", "traffic", "collect", "--daemon"},
	}); err != nil {
		t.Fatalf("saveTrafficCollectorState: %v", err)
	}
	status, err = trafficCollectorSnapshot(store)
	if err != nil {
		t.Fatalf("trafficCollectorSnapshot stale: %v", err)
	}
	if status.Running || !status.Stale || status.PID != 2147483647 {
		t.Fatalf("stale collector status = %+v", status)
	}
	if status.StartedAt != "2026-07-02 12:00:00" || status.Interval != "1s" {
		t.Fatalf("stale collector metadata = %+v", status)
	}
}

func TestTrafficStatusIncludesCollectorMetadata(t *testing.T) {
	base := t.TempDir()
	testCfg := &config.EnvConfig{ClashBaseDir: base}
	store := traffic.NewConfigStore(testCfg)
	if err := saveTrafficCollectorState(store, trafficCollectorState{
		PID:       2147483647,
		StartedAt: time.Date(2026, 7, 2, 12, 0, 0, 0, time.UTC),
		Interval:  "5s",
		LogPath:   store.CollectorLogPath(),
		Command:   []string{"clashctl", "traffic", "collect", "--daemon"},
	}); err != nil {
		t.Fatalf("saveTrafficCollectorState: %v", err)
	}

	status, err := trafficStatusSnapshot(testCfg)
	if err != nil {
		t.Fatalf("trafficStatusSnapshot: %v", err)
	}
	if status.CollectorRunning || !status.CollectorStale || status.CollectorPID != 2147483647 {
		t.Fatalf("collector fields = %+v", status)
	}
	if status.CollectorInterval != "5s" || status.CollectorLog != store.CollectorLogPath() {
		t.Fatalf("collector metadata = %+v", status)
	}
}
