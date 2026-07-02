package traffic

import (
	"os"
	"testing"
	"time"
)

func TestStoreAppendLoadHistoryTopAndPrune(t *testing.T) {
	store := NewStore(t.TempDir())
	base := time.Date(2026, 7, 2, 12, 0, 0, 0, time.UTC)
	samples := []Sample{
		{
			SchemaVersion: SchemaVersion,
			TS:            base.Add(-2 * time.Hour),
			Breakdown: []Breakdown{{
				Dimension:     "route",
				Key:           "old",
				DownloadDelta: 10,
			}, {
				Dimension:     "total",
				Key:           "total",
				DownloadDelta: 10,
			}},
		},
		{
			SchemaVersion: SchemaVersion,
			TS:            base.Add(-30 * time.Minute),
			DownBPS:       1024,
			Breakdown: []Breakdown{{
				Dimension:     "route",
				Key:           "new",
				DownloadDelta: 50,
				UploadDelta:   5,
			}, {
				Dimension:     "total",
				Key:           "total",
				DownloadDelta: 50,
				UploadDelta:   5,
			}},
		},
	}
	for _, sample := range samples {
		if err := store.AppendSample(sample); err != nil {
			t.Fatal(err)
		}
	}

	loaded, err := store.LoadSamples(base.Add(-time.Hour), base)
	if err != nil {
		t.Fatal(err)
	}
	if len(loaded) != 1 || loaded[0].Breakdown[0].Key != "new" {
		t.Fatalf("loaded samples = %+v", loaded)
	}
	points := History(loaded, time.Minute, "total", "")
	if len(points) != 1 || points[0].DownloadDelta != 50 || points[0].UploadDelta != 5 {
		t.Fatalf("history points = %+v", points)
	}
	rows := Top(loaded, "route", 1)
	if len(rows) != 1 || rows[0].Key != "new" || rows[0].TotalDelta != 55 {
		t.Fatalf("top rows = %+v", rows)
	}
	removed, err := store.Prune(time.Hour, base)
	if err != nil {
		t.Fatal(err)
	}
	if removed != 1 {
		t.Fatalf("removed = %d, want 1", removed)
	}
	remaining, err := store.LoadSamples(time.Time{}, time.Time{})
	if err != nil {
		t.Fatal(err)
	}
	if len(remaining) != 1 || remaining[0].Breakdown[0].Key != "new" {
		t.Fatalf("remaining samples = %+v", remaining)
	}
}

func TestStoreStateRoundTrip(t *testing.T) {
	store := NewStore(t.TempDir())
	state := NewState()
	state.LastSample = time.Date(2026, 7, 2, 12, 0, 0, 0, time.UTC)
	state.Connections["conn"] = ConnectionStat{Upload: 1, Download: 2}
	if err := store.SaveState(state); err != nil {
		t.Fatal(err)
	}
	if _, err := os.Stat(store.StatePath()); err != nil {
		t.Fatal(err)
	}
	loaded, err := store.LoadState()
	if err != nil {
		t.Fatal(err)
	}
	if loaded.Connections["conn"].Download != 2 {
		t.Fatalf("loaded state = %+v", loaded)
	}
}

func TestStoreRollupsMergeBucketsAndLoadBestSamples(t *testing.T) {
	store := NewStore(t.TempDir())
	base := time.Date(2026, 7, 2, 12, 0, 1, 0, time.UTC)
	for _, sample := range []Sample{
		testSample(base, "route-a", 10, 1),
		testSample(base.Add(2*time.Second), "route-a", 20, 2),
		testSample(base.Add(15*time.Second), "route-b", 30, 3),
	} {
		if err := store.AppendSampleWithRollups(sample); err != nil {
			t.Fatal(err)
		}
	}

	rollup10s, err := store.LoadRollups(10*time.Second, time.Time{}, time.Time{})
	if err != nil {
		t.Fatal(err)
	}
	if len(rollup10s) != 2 {
		t.Fatalf("10s rollup count = %d, want 2", len(rollup10s))
	}
	points := History(rollup10s[:1], 10*time.Second, "total", "")
	if len(points) != 1 || points[0].DownloadDelta != 30 || points[0].UploadDelta != 3 {
		t.Fatalf("first 10s rollup history = %+v", points)
	}

	rollup1m, err := store.LoadRollups(time.Minute, time.Time{}, time.Time{})
	if err != nil {
		t.Fatal(err)
	}
	if len(rollup1m) != 1 {
		t.Fatalf("1m rollup count = %d, want 1", len(rollup1m))
	}
	rows := Top(rollup1m, "route", 10)
	if len(rows) != 2 || rows[0].Key != "route-a" || rows[0].TotalDelta != 33 || rows[1].Key != "route-b" || rows[1].TotalDelta != 33 {
		t.Fatalf("1m rollup top = %+v", rows)
	}

	best, source, err := store.LoadBestSamples(base.Add(-time.Minute), base.Add(time.Minute), time.Minute)
	if err != nil {
		t.Fatal(err)
	}
	if source != "rollup-1m" || len(best) != 1 {
		t.Fatalf("best source=%s samples=%d, want rollup-1m/1", source, len(best))
	}
	best, source, err = store.LoadBestSamples(base.Add(-time.Minute), base.Add(time.Minute), 10*time.Second)
	if err != nil {
		t.Fatal(err)
	}
	if source != "rollup-10s" || len(best) != 2 {
		t.Fatalf("best source=%s samples=%d, want rollup-10s/2", source, len(best))
	}
}

func TestStorePruneAllUsesSeparateRetentions(t *testing.T) {
	store := NewStore(t.TempDir())
	now := time.Date(2026, 7, 2, 12, 0, 0, 0, time.UTC)
	for _, sample := range []Sample{
		testSample(now.Add(-2*time.Hour), "old", 10, 1),
		testSample(now.Add(-30*time.Minute), "new", 20, 2),
	} {
		if err := store.AppendSampleWithRollups(sample); err != nil {
			t.Fatal(err)
		}
	}
	result, err := store.PruneAll(now, time.Hour, 24*time.Hour, 24*time.Hour)
	if err != nil {
		t.Fatal(err)
	}
	if result.Raw != 1 || result.Rollup10s != 0 || result.Rollup1m != 0 {
		t.Fatalf("prune result = %+v", result)
	}
	raw, err := store.LoadSamples(time.Time{}, time.Time{})
	if err != nil {
		t.Fatal(err)
	}
	rollup10s, err := store.LoadRollups(10*time.Second, time.Time{}, time.Time{})
	if err != nil {
		t.Fatal(err)
	}
	if len(raw) != 1 || len(rollup10s) != 2 {
		t.Fatalf("raw=%d rollup10s=%d, want raw pruned only", len(raw), len(rollup10s))
	}
}

func testSample(ts time.Time, route string, down, up uint64) Sample {
	return Sample{
		SchemaVersion:     SchemaVersion,
		TS:                ts,
		DownBPS:           down,
		UpBPS:             up,
		ActiveConnections: 1,
		Breakdown: []Breakdown{{
			Dimension:     "route",
			Key:           route,
			DownloadDelta: down,
			UploadDelta:   up,
		}, {
			Dimension:     "total",
			Key:           "total",
			DownloadDelta: down,
			UploadDelta:   up,
		}},
	}
}
