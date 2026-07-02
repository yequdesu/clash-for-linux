package main

import (
	"encoding/csv"
	"encoding/json"
	"fmt"
	"os"
	"strconv"
	"time"

	"github.com/spf13/cobra"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
	"github.com/yequdesu/linux-cli-tui-clash/internal/kernel"
	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
	"github.com/yequdesu/linux-cli-tui-clash/internal/traffic"
)

var trafficCmd = &cobra.Command{
	Use:   "traffic",
	Short: "Collect and query persistent traffic statistics",
	Run: func(cmd *cobra.Command, args []string) {
		showHelpAndExit(cmd)
	},
}

var trafficStatusCmd = &cobra.Command{
	Use:   "status",
	Short: "Show traffic collector/store status",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		status, err := trafficStatusSnapshot(cfg)
		if err != nil {
			ilog.Fatal("%v", err)
		}
		if trafficStatusJSON {
			printJSON(status)
			return
		}
		ilog.Info("traffic store: %s", status.StoreDir)
		ilog.Info("samples: raw=%d rollup-10s=%d rollup-1m=%d", status.RawSamples, status.Rollup10s, status.Rollup1m)
		if status.LastSample != "" {
			ilog.Info("last sample: %s", status.LastSample)
			ilog.Info("tracked connections: %d", status.TrackedConnections)
		} else {
			ilog.Warn("last sample: never")
		}
		switch {
		case status.CollectorRunning:
			ilog.Ok("collector: running pid=%d interval=%s", status.CollectorPID, status.CollectorInterval)
		case status.CollectorStale:
			ilog.Warn("collector: stale pid=%d", status.CollectorPID)
		default:
			ilog.Warn("collector: stopped")
		}
		if status.CollectorLog != "" {
			ilog.Info("collector log: %s", status.CollectorLog)
		}
		if status.CollectorStatusReadError != "" {
			ilog.Warn("collector status: %s", status.CollectorStatusReadError)
		}
	},
}

type trafficStatusOutput struct {
	StoreDir                 string   `json:"store_dir"`
	RawSamples               int      `json:"raw_samples"`
	Rollup10s                int      `json:"rollup_10s"`
	Rollup1m                 int      `json:"rollup_1m"`
	LastSample               string   `json:"last_sample"`
	TrackedConnections       int      `json:"tracked_connections"`
	CollectorRunning         bool     `json:"collector_running"`
	CollectorStale           bool     `json:"collector_stale"`
	CollectorPID             int      `json:"collector_pid,omitempty"`
	CollectorStartedAt       string   `json:"collector_started_at,omitempty"`
	CollectorInterval        string   `json:"collector_interval,omitempty"`
	CollectorLog             string   `json:"collector_log,omitempty"`
	CollectorCommand         []string `json:"collector_command,omitempty"`
	CollectorStatusReadError string   `json:"collector_status_read_error,omitempty"`
}

func trafficStatusSnapshot(cfg *config.EnvConfig) (trafficStatusOutput, error) {
	store := traffic.NewConfigStore(cfg)
	state, err := store.LoadState()
	if err != nil {
		return trafficStatusOutput{}, err
	}
	samples, err := store.LoadSamples(time.Time{}, time.Time{})
	if err != nil {
		return trafficStatusOutput{}, err
	}
	rollup10s, err := store.LoadRollups(10*time.Second, time.Time{}, time.Time{})
	if err != nil {
		return trafficStatusOutput{}, err
	}
	rollup1m, err := store.LoadRollups(time.Minute, time.Time{}, time.Time{})
	if err != nil {
		return trafficStatusOutput{}, err
	}
	status := trafficStatusOutput{
		StoreDir:           store.Dir,
		RawSamples:         len(samples),
		Rollup10s:          len(rollup10s),
		Rollup1m:           len(rollup1m),
		TrackedConnections: len(state.Connections),
	}
	if !state.LastSample.IsZero() {
		status.LastSample = state.LastSample.Format(profileTimeFormat)
	}
	collector, err := trafficCollectorSnapshot(store)
	if err != nil {
		status.CollectorStatusReadError = err.Error()
		return status, nil
	}
	status.CollectorRunning = collector.Running
	status.CollectorStale = collector.Stale
	status.CollectorPID = collector.PID
	status.CollectorStartedAt = collector.StartedAt
	status.CollectorInterval = collector.Interval
	status.CollectorLog = collector.LogPath
	status.CollectorCommand = collector.Command
	return status, nil
}

var trafficSampleCmd = &cobra.Command{
	Use:   "sample",
	Short: "Collect one traffic sample",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		sample, err := collectTrafficSample(cfg)
		if err != nil {
			ilog.Fatal("%v", err)
		}
		ilog.Ok("traffic sample: down %s/s up %s/s conn %d", formatBytes(sample.DownBPS), formatBytes(sample.UpBPS), sample.ActiveConnections)
	},
}

var trafficCollectCmd = &cobra.Command{
	Use:   "collect",
	Short: "Collect traffic samples continuously",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		if !trafficCollectDaemon {
			ilog.Fatal("traffic collect currently requires --daemon")
		}
		if trafficCollectInterval <= 0 {
			ilog.Fatal("collector interval must be positive")
		}
		ticker := time.NewTicker(trafficCollectInterval)
		defer ticker.Stop()
		for {
			if _, err := collectTrafficSample(cfg); err != nil {
				ilog.Warn("traffic sample failed: %v", err)
			}
			<-ticker.C
		}
	},
}

var trafficHistoryCmd = &cobra.Command{
	Use:   "history",
	Short: "Query traffic history",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		samples, source, err := loadTrafficRange(trafficHistoryStep)
		if err != nil {
			ilog.Fatal("%v", err)
		}
		points := traffic.History(samples, trafficHistoryStep, trafficBy, trafficKey)
		if trafficJSON {
			printJSON(points)
			return
		}
		ilog.Info("source: %s", source)
		for _, point := range points {
			fmt.Printf("%s down=%s up=%s rate_down=%s/s rate_up=%s/s conn=%d\n",
				point.TS.Format(profileTimeFormat),
				formatBytes(point.DownloadDelta),
				formatBytes(point.UploadDelta),
				formatBytes(point.DownBPS),
				formatBytes(point.UpBPS),
				point.Connections,
			)
		}
	},
}

var trafficTopCmd = &cobra.Command{
	Use:   "top",
	Short: "Show top traffic dimensions",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		samples, source, err := loadTrafficRange(trafficTopStep())
		if err != nil {
			ilog.Fatal("%v", err)
		}
		rows := traffic.Top(samples, trafficBy, trafficTopLimit)
		if trafficJSON {
			printJSON(rows)
			return
		}
		ilog.Info("source: %s", source)
		for _, row := range rows {
			fmt.Printf("%-8s %-48s down=%s up=%s total=%s\n",
				row.Dimension,
				truncStr(row.Key, 48),
				formatBytes(row.DownloadDelta),
				formatBytes(row.UploadDelta),
				formatBytes(row.TotalDelta),
			)
		}
	},
}

var trafficExportCmd = &cobra.Command{
	Use:   "export",
	Short: "Export traffic samples",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		samples, _, err := loadTrafficRange(time.Second)
		if err != nil {
			ilog.Fatal("%v", err)
		}
		samples = filterTrafficSamplesForExport(samples, trafficBy)
		switch trafficExportFormat {
		case "json":
			printJSON(samples)
		case "csv":
			w := csv.NewWriter(os.Stdout)
			_ = w.Write([]string{"ts", "dimension", "key", "download_delta", "upload_delta"})
			for _, sample := range samples {
				for _, row := range sample.Breakdown {
					_ = w.Write([]string{
						sample.TS.Format(time.RFC3339),
						row.Dimension,
						row.Key,
						strconv.FormatUint(row.DownloadDelta, 10),
						strconv.FormatUint(row.UploadDelta, 10),
					})
				}
			}
			w.Flush()
			if err := w.Error(); err != nil {
				ilog.Fatal("write csv: %v", err)
			}
		default:
			ilog.Fatal("unsupported export format: %s", trafficExportFormat)
		}
	},
}

func filterTrafficSamplesForExport(samples []traffic.Sample, dimension string) []traffic.Sample {
	if dimension == "" || dimension == "all" {
		return samples
	}

	filtered := make([]traffic.Sample, 0, len(samples))
	for _, sample := range samples {
		next := sample
		next.Breakdown = filterTrafficBreakdownsForExport(sample.Breakdown, dimension)
		filtered = append(filtered, next)
	}
	return filtered
}

func filterTrafficBreakdownsForExport(rows []traffic.Breakdown, dimension string) []traffic.Breakdown {
	filtered := make([]traffic.Breakdown, 0, len(rows))
	for _, row := range rows {
		if row.Dimension == dimension {
			filtered = append(filtered, row)
		}
	}
	return filtered
}

var trafficPruneCmd = &cobra.Command{
	Use:   "prune",
	Short: "Prune old traffic samples",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		if trafficPruneRetention <= 0 {
			ilog.Fatal("retention must be positive")
		}
		store := traffic.NewConfigStore(cfg)
		var removed int
		err := config.WithFileLock(store.LockPath(), func() error {
			var err error
			result, err := store.PruneAll(timeNow(), trafficPruneRetention, trafficPruneRollup10sRetention, trafficPruneRollup1mRetention)
			removed = result.Raw + result.Rollup10s + result.Rollup1m
			return err
		})
		if err != nil {
			ilog.Fatal("%v", err)
		}
		ilog.Ok("traffic samples pruned: %d", removed)
	},
}

var trafficResetCmd = &cobra.Command{
	Use:   "reset",
	Short: "Delete all traffic statistics",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		if !trafficResetYes {
			ilog.Fatal("refusing to reset traffic data without --yes")
		}
		store := traffic.NewConfigStore(cfg)
		err := config.WithFileLock(store.LockPath(), func() error {
			if err := os.Remove(store.SamplesPath()); err != nil && !os.IsNotExist(err) {
				return err
			}
			if err := os.Remove(store.StatePath()); err != nil && !os.IsNotExist(err) {
				return err
			}
			if err := os.Remove(store.Rollup10sPath()); err != nil && !os.IsNotExist(err) {
				return err
			}
			if err := os.Remove(store.Rollup1mPath()); err != nil && !os.IsNotExist(err) {
				return err
			}
			return nil
		})
		if err != nil {
			ilog.Fatal("%v", err)
		}
		ilog.Ok("traffic data reset")
	},
}

var (
	trafficCollectDaemon           bool
	trafficCollectInterval         time.Duration
	trafficRange                   time.Duration
	trafficHistoryStep             time.Duration
	trafficBy                      string
	trafficKey                     string
	trafficTopLimit                int
	trafficJSON                    bool
	trafficExportFormat            string
	trafficPruneRetention          time.Duration
	trafficPruneRollup10sRetention time.Duration
	trafficPruneRollup1mRetention  time.Duration
	trafficResetYes                bool
	trafficStatusJSON              bool
)

func init() {
	trafficCmd.AddCommand(trafficStatusCmd)
	trafficCmd.AddCommand(trafficSampleCmd)
	trafficCmd.AddCommand(trafficCollectCmd)
	trafficCmd.AddCommand(trafficHistoryCmd)
	trafficCmd.AddCommand(trafficTopCmd)
	trafficCmd.AddCommand(trafficExportCmd)
	trafficCmd.AddCommand(trafficPruneCmd)
	trafficCmd.AddCommand(trafficResetCmd)

	trafficStatusCmd.Flags().BoolVar(&trafficStatusJSON, "json", false, "print JSON")
	trafficSampleCmd.Flags().Bool("once", true, "collect a single sample")
	trafficCollectCmd.Flags().BoolVar(&trafficCollectDaemon, "daemon", false, "run continuous collector loop")
	trafficCollectCmd.Flags().DurationVar(&trafficCollectInterval, "interval", time.Second, "collector sample interval")

	for _, cmd := range []*cobra.Command{trafficHistoryCmd, trafficTopCmd, trafficExportCmd} {
		cmd.Flags().DurationVar(&trafficRange, "range", time.Hour, "query time range")
		cmd.Flags().StringVar(&trafficBy, "by", "", "dimension: total|route|rule|group|node|host|process|network")
		cmd.Flags().BoolVar(&trafficJSON, "json", false, "print JSON")
	}
	trafficHistoryCmd.Flags().DurationVar(&trafficHistoryStep, "step", time.Minute, "history bucket step")
	trafficHistoryCmd.Flags().StringVar(&trafficKey, "key", "", "optional dimension key filter")
	trafficTopCmd.Flags().IntVar(&trafficTopLimit, "limit", 10, "maximum rows")
	trafficExportCmd.Flags().StringVar(&trafficExportFormat, "format", "json", "export format: json|csv")
	trafficPruneCmd.Flags().DurationVar(&trafficPruneRetention, "retention", traffic.DefaultRawRetention, "raw sample retention window")
	trafficPruneCmd.Flags().DurationVar(&trafficPruneRollup10sRetention, "rollup-10s-retention", traffic.DefaultRollup10sRetention, "10s rollup retention window")
	trafficPruneCmd.Flags().DurationVar(&trafficPruneRollup1mRetention, "rollup-1m-retention", traffic.DefaultRollup1mRetention, "1m rollup retention window")
	trafficResetCmd.Flags().BoolVar(&trafficResetYes, "yes", false, "confirm deleting all traffic data")
}

func collectTrafficSample(cfg *config.EnvConfig) (traffic.Sample, error) {
	info := readRuntimeInfo(cfg)
	if !apiOpen(info) {
		return traffic.Sample{}, fmt.Errorf("kernel API not available at %s", info.apiAddress())
	}
	api := kernel.NewClient(info.apiBaseURL(), info.secret)
	rate, rateErr := api.GetTraffic()
	connections, err := api.GetConnections()
	if err != nil {
		return traffic.Sample{}, fmt.Errorf("read connections: %w", err)
	}
	if rateErr != nil {
		ilog.Warn("traffic rate unavailable, falling back to connection totals: %v", rateErr)
	}
	store := traffic.NewConfigStore(cfg)
	var sample traffic.Sample
	err = config.WithFileLock(store.LockPath(), func() error {
		state, err := store.LoadState()
		if err != nil {
			return err
		}
		nextSample, nextState := traffic.BuildSample(timeNow(), connections, rate, state)
		if err := store.AppendSampleWithRollups(nextSample); err != nil {
			return err
		}
		if err := store.SaveState(nextState); err != nil {
			return err
		}
		sample = nextSample
		return nil
	})
	return sample, err
}

func loadTrafficRange(step time.Duration) ([]traffic.Sample, string, error) {
	if trafficRange <= 0 {
		return nil, "", fmt.Errorf("range must be positive")
	}
	store := traffic.NewConfigStore(cfg)
	return store.LoadBestSamples(timeNow().Add(-trafficRange), timeNow(), step)
}

func trafficTopStep() time.Duration {
	if trafficRange >= 24*time.Hour {
		return time.Minute
	}
	if trafficRange >= time.Hour {
		return 10 * time.Second
	}
	return time.Second
}

func printJSON(v any) {
	enc := json.NewEncoder(os.Stdout)
	enc.SetIndent("", "  ")
	if err := enc.Encode(v); err != nil {
		ilog.Fatal("encode json: %v", err)
	}
}

func formatBytes(v uint64) string {
	const unit = 1024
	if v < unit {
		return fmt.Sprintf("%d B", v)
	}
	value := float64(v)
	units := []string{"KB", "MB", "GB", "TB", "PB"}
	for _, suffix := range units {
		value /= unit
		if value < unit {
			return fmt.Sprintf("%.1f %s", value, suffix)
		}
	}
	return fmt.Sprintf("%.1f EB", value/unit)
}
