package traffic

import (
	"fmt"
	"sort"
	"strings"
	"time"

	"github.com/yequdesu/linux-cli-tui-clash/internal/kernel"
)

const SchemaVersion = 1

type Sample struct {
	SchemaVersion     int         `json:"schema_version"`
	TS                time.Time   `json:"ts"`
	UpBPS             uint64      `json:"up_bps"`
	DownBPS           uint64      `json:"down_bps"`
	UploadTotal       uint64      `json:"upload_total"`
	DownloadTotal     uint64      `json:"download_total"`
	ActiveConnections int         `json:"active_connections"`
	Breakdown         []Breakdown `json:"breakdown,omitempty"`
}

type Breakdown struct {
	Dimension     string `json:"dimension"`
	Key           string `json:"key"`
	Rule          string `json:"rule,omitempty"`
	RulePayload   string `json:"rule_payload,omitempty"`
	Chain         string `json:"chain,omitempty"`
	Group         string `json:"group,omitempty"`
	Node          string `json:"node,omitempty"`
	Host          string `json:"host,omitempty"`
	Process       string `json:"process,omitempty"`
	Network       string `json:"network,omitempty"`
	UploadDelta   uint64 `json:"upload_delta"`
	DownloadDelta uint64 `json:"download_delta"`
}

type CollectorState struct {
	SchemaVersion int                       `json:"schema_version"`
	LastSample    time.Time                 `json:"last_sample"`
	Connections   map[string]ConnectionStat `json:"connections"`
}

type ConnectionStat struct {
	Upload   uint64 `json:"upload"`
	Download uint64 `json:"download"`
}

type Point struct {
	TS            time.Time `json:"ts"`
	UploadDelta   uint64    `json:"upload_delta"`
	DownloadDelta uint64    `json:"download_delta"`
	UpBPS         uint64    `json:"up_bps"`
	DownBPS       uint64    `json:"down_bps"`
	Connections   int       `json:"connections"`
	Dimension     string    `json:"dimension,omitempty"`
	Key           string    `json:"key,omitempty"`
}

type TopRow struct {
	Dimension     string `json:"dimension"`
	Key           string `json:"key"`
	UploadDelta   uint64 `json:"upload_delta"`
	DownloadDelta uint64 `json:"download_delta"`
	TotalDelta    uint64 `json:"total_delta"`
}

func NewState() CollectorState {
	return CollectorState{
		SchemaVersion: SchemaVersion,
		Connections:   map[string]ConnectionStat{},
	}
}

func BuildSample(now time.Time, conn kernel.ConnectionsResponse, rate kernel.TrafficResponse, state CollectorState) (Sample, CollectorState) {
	if state.Connections == nil {
		state.Connections = map[string]ConnectionStat{}
	}
	next := NewState()
	next.LastSample = now

	sample := Sample{
		SchemaVersion:     SchemaVersion,
		TS:                now,
		UpBPS:             rate.Up,
		DownBPS:           rate.Down,
		UploadTotal:       conn.UploadTotal,
		DownloadTotal:     conn.DownloadTotal,
		ActiveConnections: len(conn.Connections),
	}

	aggregates := map[string]Breakdown{}
	for _, c := range conn.Connections {
		next.Connections[c.ID] = ConnectionStat{Upload: c.Upload, Download: c.Download}
		previous := state.Connections[c.ID]
		upDelta := saturatingDelta(c.Upload, previous.Upload)
		downDelta := saturatingDelta(c.Download, previous.Download)
		if upDelta == 0 && downDelta == 0 {
			continue
		}
		addBreakdowns(aggregates, c, upDelta, downDelta)
	}

	keys := make([]string, 0, len(aggregates))
	for key := range aggregates {
		keys = append(keys, key)
	}
	sort.Strings(keys)
	for _, key := range keys {
		b := aggregates[key]
		sample.Breakdown = append(sample.Breakdown, b)
	}
	return sample, next
}

func addBreakdowns(aggregates map[string]Breakdown, c kernel.Connection, upDelta, downDelta uint64) {
	chain, group, node := chainParts(c.Chains)
	routeKey := fmt.Sprintf("%s/%s -> %s", emptyAs(c.Rule, "UNKNOWN"), emptyAs(c.RulePayload, "-"), emptyAs(chain, "UNKNOWN"))
	host := firstNonEmpty(c.Metadata.Host, c.Metadata.DestinationIP, "unknown")
	process := firstNonEmpty(c.Metadata.Process, c.Metadata.ProcessPath, "unknown")
	network := emptyAs(c.Metadata.Network, "unknown")

	rows := []Breakdown{
		{Dimension: "total", Key: "total"},
		{Dimension: "rule", Key: emptyAs(c.Rule, "UNKNOWN"), Rule: c.Rule, RulePayload: c.RulePayload},
		{Dimension: "route", Key: routeKey, Rule: c.Rule, RulePayload: c.RulePayload, Chain: chain, Group: group, Node: node},
		{Dimension: "group", Key: emptyAs(group, "UNKNOWN"), Chain: chain, Group: group, Node: node},
		{Dimension: "node", Key: emptyAs(node, "UNKNOWN"), Chain: chain, Group: group, Node: node},
		{Dimension: "host", Key: host, Host: host},
		{Dimension: "process", Key: process, Process: process},
		{Dimension: "network", Key: network, Network: network},
	}
	for _, row := range rows {
		key := row.Dimension + "\x00" + row.Key
		current := aggregates[key]
		if current.Dimension == "" {
			current = row
		}
		current.UploadDelta += upDelta
		current.DownloadDelta += downDelta
		aggregates[key] = current
	}
}

func chainParts(chains []string) (chain, group, node string) {
	if len(chains) == 0 {
		return "", "", ""
	}
	chain = strings.Join(chains, " -> ")
	group = chains[0]
	node = chains[len(chains)-1]
	return chain, group, node
}

func saturatingDelta(current, previous uint64) uint64 {
	if current < previous {
		return 0
	}
	return current - previous
}

func firstNonEmpty(values ...string) string {
	for _, value := range values {
		value = strings.TrimSpace(value)
		if value != "" {
			return value
		}
	}
	return ""
}

func emptyAs(value, fallback string) string {
	value = strings.TrimSpace(value)
	if value == "" {
		return fallback
	}
	return value
}
