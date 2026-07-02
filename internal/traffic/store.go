package traffic

import (
	"bufio"
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"time"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
)

type Store struct {
	Dir string
}

type PruneResult struct {
	Raw       int `json:"raw"`
	Rollup10s int `json:"rollup_10s"`
	Rollup1m  int `json:"rollup_1m"`
}

const (
	DefaultRawRetention       = 24 * time.Hour
	DefaultRollup10sRetention = 7 * 24 * time.Hour
	DefaultRollup1mRetention  = 90 * 24 * time.Hour
)

func NewStore(dir string) Store {
	return Store{Dir: dir}
}

func NewConfigStore(cfg *config.EnvConfig) Store {
	return NewStore(cfg.TrafficDir())
}

func (s Store) SamplesPath() string { return filepath.Join(s.Dir, "samples.jsonl") }
func (s Store) StatePath() string   { return filepath.Join(s.Dir, "state.json") }
func (s Store) LockPath() string    { return filepath.Join(s.Dir, "lock") }
func (s Store) CollectorPath() string {
	return filepath.Join(s.Dir, "collector.json")
}
func (s Store) CollectorLogPath() string {
	return filepath.Join(s.Dir, "collector.log")
}
func (s Store) Rollup10sPath() string {
	return filepath.Join(s.Dir, "rollup-10s.jsonl")
}
func (s Store) Rollup1mPath() string {
	return filepath.Join(s.Dir, "rollup-1m.jsonl")
}

func (s Store) LoadState() (CollectorState, error) {
	data, err := os.ReadFile(s.StatePath())
	if err != nil {
		if errors.Is(err, os.ErrNotExist) {
			return NewState(), nil
		}
		return CollectorState{}, fmt.Errorf("read traffic state: %w", err)
	}
	var state CollectorState
	if err := json.Unmarshal(data, &state); err != nil {
		return CollectorState{}, fmt.Errorf("parse traffic state: %w", err)
	}
	if state.Connections == nil {
		state.Connections = map[string]ConnectionStat{}
	}
	return state, nil
}

func (s Store) SaveState(state CollectorState) error {
	if err := os.MkdirAll(s.Dir, 0o755); err != nil {
		return err
	}
	if state.SchemaVersion == 0 {
		state.SchemaVersion = SchemaVersion
	}
	data, err := json.MarshalIndent(state, "", "  ")
	if err != nil {
		return err
	}
	return config.AtomicWriteFile(s.StatePath(), append(data, '\n'), 0o644)
}

func (s Store) AppendSample(sample Sample) error {
	return appendSampleToPath(s.SamplesPath(), sample)
}

func (s Store) AppendSampleWithRollups(sample Sample) error {
	if err := s.AppendSample(sample); err != nil {
		return err
	}
	if err := s.UpsertRollup(10*time.Second, sample); err != nil {
		return err
	}
	return s.UpsertRollup(time.Minute, sample)
}

func appendSampleToPath(path string, sample Sample) error {
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		return err
	}
	if sample.SchemaVersion == 0 {
		sample.SchemaVersion = SchemaVersion
	}
	data, err := json.Marshal(sample)
	if err != nil {
		return err
	}
	f, err := os.OpenFile(path, os.O_CREATE|os.O_WRONLY|os.O_APPEND, 0o644)
	if err != nil {
		return fmt.Errorf("open samples: %w", err)
	}
	defer f.Close()
	if _, err := f.Write(append(data, '\n')); err != nil {
		return fmt.Errorf("write sample: %w", err)
	}
	return nil
}

func (s Store) UpsertRollup(step time.Duration, sample Sample) error {
	path, err := s.rollupPath(step)
	if err != nil {
		return err
	}
	if err := os.MkdirAll(s.Dir, 0o755); err != nil {
		return err
	}
	samples, err := loadSamplesFromPath(path, time.Time{}, time.Time{})
	if err != nil {
		return err
	}
	bucket := sample.TS.Truncate(step)
	found := false
	for i := range samples {
		if samples[i].TS.Equal(bucket) {
			samples[i] = mergeRollupSample(samples[i], sample, bucket)
			found = true
			break
		}
	}
	if !found {
		samples = append(samples, mergeRollupSample(Sample{SchemaVersion: SchemaVersion, TS: bucket}, sample, bucket))
	}
	sort.Slice(samples, func(i, j int) bool { return samples[i].TS.Before(samples[j].TS) })
	return writeSamplesToPath(path, samples)
}

func (s Store) LoadSamples(from, to time.Time) ([]Sample, error) {
	return loadSamplesFromPath(s.SamplesPath(), from, to)
}

func (s Store) LoadRollups(step time.Duration, from, to time.Time) ([]Sample, error) {
	path, err := s.rollupPath(step)
	if err != nil {
		return nil, err
	}
	return loadSamplesFromPath(path, from, to)
}

func (s Store) LoadBestSamples(from, to time.Time, step time.Duration) ([]Sample, string, error) {
	if step >= time.Minute {
		if samples, err := s.LoadRollups(time.Minute, from, to); err != nil {
			return nil, "", err
		} else if len(samples) > 0 {
			return samples, "rollup-1m", nil
		}
	}
	if step >= 10*time.Second {
		if samples, err := s.LoadRollups(10*time.Second, from, to); err != nil {
			return nil, "", err
		} else if len(samples) > 0 {
			return samples, "rollup-10s", nil
		}
	}
	samples, err := s.LoadSamples(from, to)
	return samples, "raw", err
}

func loadSamplesFromPath(path string, from, to time.Time) ([]Sample, error) {
	f, err := os.Open(path)
	if err != nil {
		if errors.Is(err, os.ErrNotExist) {
			return nil, nil
		}
		return nil, fmt.Errorf("open samples: %w", err)
	}
	defer f.Close()

	var samples []Sample
	scanner := bufio.NewScanner(f)
	scanner.Buffer(make([]byte, 64*1024), 4*1024*1024)
	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		if line == "" {
			continue
		}
		var sample Sample
		if err := json.Unmarshal([]byte(line), &sample); err != nil {
			return nil, fmt.Errorf("parse sample: %w", err)
		}
		if !from.IsZero() && sample.TS.Before(from) {
			continue
		}
		if !to.IsZero() && sample.TS.After(to) {
			continue
		}
		samples = append(samples, sample)
	}
	if err := scanner.Err(); err != nil {
		return nil, fmt.Errorf("read samples: %w", err)
	}
	return samples, nil
}

func (s Store) Prune(retention time.Duration, now time.Time) (int, error) {
	return pruneSamplesPath(s.SamplesPath(), retention, now)
}

func (s Store) PruneAll(now time.Time, rawRetention, rollup10sRetention, rollup1mRetention time.Duration) (PruneResult, error) {
	result := PruneResult{}
	var err error
	result.Raw, err = pruneSamplesPath(s.SamplesPath(), rawRetention, now)
	if err != nil {
		return result, err
	}
	result.Rollup10s, err = pruneSamplesPath(s.Rollup10sPath(), rollup10sRetention, now)
	if err != nil {
		return result, err
	}
	result.Rollup1m, err = pruneSamplesPath(s.Rollup1mPath(), rollup1mRetention, now)
	if err != nil {
		return result, err
	}
	return result, nil
}

func pruneSamplesPath(path string, retention time.Duration, now time.Time) (int, error) {
	if retention <= 0 {
		return 0, fmt.Errorf("retention must be positive")
	}
	samples, err := loadSamplesFromPath(path, now.Add(-retention), time.Time{})
	if err != nil {
		return 0, err
	}
	all, err := loadSamplesFromPath(path, time.Time{}, time.Time{})
	if err != nil {
		return 0, err
	}
	return len(all) - len(samples), writeSamplesToPath(path, samples)
}

func History(samples []Sample, step time.Duration, dimension, key string) []Point {
	if step <= 0 {
		step = time.Minute
	}
	if dimension == "" {
		dimension = "total"
	}
	if dimension == "total" && key == "" {
		key = "total"
	}
	buckets := map[int64]Point{}
	for _, sample := range samples {
		bucket := sample.TS.Truncate(step).Unix()
		point := buckets[bucket]
		point.TS = time.Unix(bucket, 0)
		point.UpBPS = maxUint64(point.UpBPS, sample.UpBPS)
		point.DownBPS = maxUint64(point.DownBPS, sample.DownBPS)
		point.Connections = maxInt(point.Connections, sample.ActiveConnections)
		point.Dimension = dimension
		point.Key = key
		for _, b := range sample.Breakdown {
			if dimension != "" && b.Dimension != dimension {
				continue
			}
			if key != "" && b.Key != key {
				continue
			}
			point.UploadDelta += b.UploadDelta
			point.DownloadDelta += b.DownloadDelta
		}
		buckets[bucket] = point
	}
	points := make([]Point, 0, len(buckets))
	for _, point := range buckets {
		points = append(points, point)
	}
	sort.Slice(points, func(i, j int) bool { return points[i].TS.Before(points[j].TS) })
	return points
}

func Top(samples []Sample, dimension string, limit int) []TopRow {
	if dimension == "" {
		dimension = "route"
	}
	if limit <= 0 {
		limit = 10
	}
	rows := map[string]TopRow{}
	for _, sample := range samples {
		for _, b := range sample.Breakdown {
			if b.Dimension != dimension {
				continue
			}
			row := rows[b.Key]
			row.Dimension = dimension
			row.Key = b.Key
			row.UploadDelta += b.UploadDelta
			row.DownloadDelta += b.DownloadDelta
			row.TotalDelta = row.UploadDelta + row.DownloadDelta
			rows[b.Key] = row
		}
	}
	out := make([]TopRow, 0, len(rows))
	for _, row := range rows {
		out = append(out, row)
	}
	sort.Slice(out, func(i, j int) bool {
		if out[i].TotalDelta == out[j].TotalDelta {
			return out[i].Key < out[j].Key
		}
		return out[i].TotalDelta > out[j].TotalDelta
	})
	if len(out) > limit {
		out = out[:limit]
	}
	return out
}

func (s Store) rollupPath(step time.Duration) (string, error) {
	switch step {
	case 10 * time.Second:
		return s.Rollup10sPath(), nil
	case time.Minute:
		return s.Rollup1mPath(), nil
	default:
		return "", fmt.Errorf("unsupported rollup step: %s", step)
	}
}

func writeSamplesToPath(path string, samples []Sample) error {
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		return err
	}
	var out bytes.Buffer
	for _, sample := range samples {
		if sample.SchemaVersion == 0 {
			sample.SchemaVersion = SchemaVersion
		}
		data, err := json.Marshal(sample)
		if err != nil {
			return err
		}
		if _, err := out.Write(append(data, '\n')); err != nil {
			return err
		}
	}
	return config.AtomicWriteFile(path, out.Bytes(), 0o644)
}

func mergeRollupSample(base, sample Sample, bucket time.Time) Sample {
	base.SchemaVersion = SchemaVersion
	base.TS = bucket
	base.UpBPS = maxUint64(base.UpBPS, sample.UpBPS)
	base.DownBPS = maxUint64(base.DownBPS, sample.DownBPS)
	base.UploadTotal = maxUint64(base.UploadTotal, sample.UploadTotal)
	base.DownloadTotal = maxUint64(base.DownloadTotal, sample.DownloadTotal)
	base.ActiveConnections = maxInt(base.ActiveConnections, sample.ActiveConnections)

	rows := map[string]Breakdown{}
	for _, row := range base.Breakdown {
		rows[row.Dimension+"\x00"+row.Key] = row
	}
	for _, row := range sample.Breakdown {
		key := row.Dimension + "\x00" + row.Key
		current := rows[key]
		if current.Dimension == "" {
			current = row
			current.UploadDelta = 0
			current.DownloadDelta = 0
		}
		current.UploadDelta += row.UploadDelta
		current.DownloadDelta += row.DownloadDelta
		rows[key] = current
	}

	keys := make([]string, 0, len(rows))
	for key := range rows {
		keys = append(keys, key)
	}
	sort.Strings(keys)
	base.Breakdown = base.Breakdown[:0]
	for _, key := range keys {
		base.Breakdown = append(base.Breakdown, rows[key])
	}
	return base
}

func maxUint64(a, b uint64) uint64 {
	if a > b {
		return a
	}
	return b
}

func maxInt(a, b int) int {
	if a > b {
		return a
	}
	return b
}
