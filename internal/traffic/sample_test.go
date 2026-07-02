package traffic

import (
	"strings"
	"testing"
	"time"

	"github.com/yequdesu/linux-cli-tui-clash/internal/kernel"
)

func TestBuildSampleUsesConnectionDeltasWithoutDoubleCounting(t *testing.T) {
	now := time.Date(2026, 7, 2, 12, 0, 0, 0, time.UTC)
	state := NewState()
	first, next := BuildSample(now, kernel.ConnectionsResponse{
		UploadTotal:   1000,
		DownloadTotal: 2000,
		Connections: []kernel.Connection{{
			ID:       "conn-1",
			Upload:   100,
			Download: 200,
			Rule:     "Proxy",
			Chains:   []string{"Proxy", "HK-01"},
			Metadata: kernel.ConnectionMetadata{Host: "example.test", Network: "tcp"},
		}},
	}, kernel.TrafficResponse{Up: 10, Down: 20}, state)
	if got := totalDelta(first); got != 300 {
		t.Fatalf("first total delta = %d, want 300", got)
	}

	second, _ := BuildSample(now.Add(time.Second), kernel.ConnectionsResponse{
		UploadTotal:   1000,
		DownloadTotal: 2000,
		Connections: []kernel.Connection{{
			ID:       "conn-1",
			Upload:   100,
			Download: 200,
			Rule:     "Proxy",
			Chains:   []string{"Proxy", "HK-01"},
			Metadata: kernel.ConnectionMetadata{Host: "example.test", Network: "tcp"},
		}},
	}, kernel.TrafficResponse{Up: 10, Down: 20}, next)
	if got := totalDelta(second); got != 0 {
		t.Fatalf("second total delta = %d, want 0", got)
	}
}

func TestBuildSampleCreatesRouteAggregationKey(t *testing.T) {
	sample, _ := BuildSample(time.Now(), kernel.ConnectionsResponse{
		Connections: []kernel.Connection{{
			ID:          "conn-1",
			Upload:      1,
			Download:    2,
			Rule:        "RuleSet",
			RulePayload: "geosite:google",
			Chains:      []string{"Proxy", "HK-01"},
		}},
	}, kernel.TrafficResponse{}, NewState())
	for _, row := range sample.Breakdown {
		if row.Dimension == "route" {
			if !strings.Contains(row.Key, "RuleSet/geosite:google -> Proxy -> HK-01") {
				t.Fatalf("route key = %q", row.Key)
			}
			if row.Group != "Proxy" || row.Node != "HK-01" {
				t.Fatalf("route group/node = %q/%q", row.Group, row.Node)
			}
			return
		}
	}
	t.Fatal("route breakdown missing")
}

func totalDelta(sample Sample) uint64 {
	var total uint64
	for _, row := range sample.Breakdown {
		if row.Dimension == "total" {
			total += row.UploadDelta + row.DownloadDelta
		}
	}
	return total
}
