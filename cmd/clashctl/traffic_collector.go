package main

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"os/exec"
	"time"

	"github.com/spf13/cobra"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
	"github.com/yequdesu/linux-cli-tui-clash/internal/traffic"
)

type trafficCollectorState struct {
	PID       int       `json:"pid"`
	StartedAt time.Time `json:"started_at"`
	Interval  string    `json:"interval"`
	LogPath   string    `json:"log_path"`
	Command   []string  `json:"command,omitempty"`
}

type trafficCollectorStatusOutput struct {
	Running   bool     `json:"running"`
	Stale     bool     `json:"stale"`
	PID       int      `json:"pid,omitempty"`
	StartedAt string   `json:"started_at,omitempty"`
	Interval  string   `json:"interval,omitempty"`
	LogPath   string   `json:"log_path,omitempty"`
	Command   []string `json:"command,omitempty"`
}

var trafficCollectorCmd = &cobra.Command{
	Use:   "collector",
	Short: "Manage the persistent traffic collector",
	Run: func(cmd *cobra.Command, args []string) {
		showHelpAndExit(cmd)
	},
}

var trafficCollectorStatusCmd = &cobra.Command{
	Use:   "status",
	Short: "Show persistent traffic collector status",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		store := traffic.NewConfigStore(cfg)
		status, err := trafficCollectorSnapshot(store)
		if err != nil {
			ilog.Fatal("%v", err)
		}
		if trafficCollectorJSON {
			printJSON(status)
			return
		}
		printTrafficCollectorStatus(status)
	},
}

var trafficCollectorStartCmd = &cobra.Command{
	Use:   "start",
	Short: "Start the persistent traffic collector",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		status, err := startTrafficCollector(cfg, trafficCollectorInterval)
		if err != nil {
			ilog.Fatal("%v", err)
		}
		if status.Running {
			ilog.Ok("traffic collector started: pid=%d interval=%s log=%s", status.PID, status.Interval, status.LogPath)
		}
	},
}

var trafficCollectorStopCmd = &cobra.Command{
	Use:   "stop",
	Short: "Stop the persistent traffic collector",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		status, err := stopTrafficCollector(cfg)
		if err != nil {
			ilog.Fatal("%v", err)
		}
		if status.Stale {
			ilog.Warn("traffic collector stale pid cleared: %d", status.PID)
			return
		}
		if status.PID == 0 {
			ilog.Info("traffic collector already stopped")
			return
		}
		ilog.Ok("traffic collector stopped: pid=%d", status.PID)
	},
}

var trafficCollectorRestartCmd = &cobra.Command{
	Use:   "restart",
	Short: "Restart the persistent traffic collector",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		if _, err := stopTrafficCollector(cfg); err != nil {
			ilog.Fatal("%v", err)
		}
		status, err := startTrafficCollector(cfg, trafficCollectorInterval)
		if err != nil {
			ilog.Fatal("%v", err)
		}
		ilog.Ok("traffic collector restarted: pid=%d interval=%s log=%s", status.PID, status.Interval, status.LogPath)
	},
}

var (
	trafficCollectorInterval time.Duration
	trafficCollectorJSON     bool
)

func init() {
	trafficCmd.AddCommand(trafficCollectorCmd)
	trafficCollectorCmd.AddCommand(trafficCollectorStatusCmd)
	trafficCollectorCmd.AddCommand(trafficCollectorStartCmd)
	trafficCollectorCmd.AddCommand(trafficCollectorStopCmd)
	trafficCollectorCmd.AddCommand(trafficCollectorRestartCmd)
	trafficCollectorStatusCmd.Flags().BoolVar(&trafficCollectorJSON, "json", false, "print JSON")
	for _, cmd := range []*cobra.Command{trafficCollectorStartCmd, trafficCollectorRestartCmd} {
		cmd.Flags().DurationVar(&trafficCollectorInterval, "interval", time.Second, "collector sample interval")
	}
}

func startTrafficCollector(cfg *config.EnvConfig, interval time.Duration) (trafficCollectorStatusOutput, error) {
	if interval <= 0 {
		return trafficCollectorStatusOutput{}, fmt.Errorf("collector interval must be positive")
	}
	store := traffic.NewConfigStore(cfg)
	status, err := trafficCollectorSnapshot(store)
	if err != nil {
		return trafficCollectorStatusOutput{}, err
	}
	if status.Running {
		ilog.Info("traffic collector already running: pid=%d", status.PID)
		return status, nil
	}
	if err := os.MkdirAll(store.Dir, 0o755); err != nil {
		return trafficCollectorStatusOutput{}, err
	}
	logFile, err := os.OpenFile(store.CollectorLogPath(), os.O_CREATE|os.O_WRONLY|os.O_APPEND, 0o644)
	if err != nil {
		return trafficCollectorStatusOutput{}, fmt.Errorf("open collector log: %w", err)
	}
	defer logFile.Close()

	exe, err := os.Executable()
	if err != nil {
		return trafficCollectorStatusOutput{}, fmt.Errorf("resolve executable: %w", err)
	}
	args := trafficCollectorArgs(interval)
	cmd := exec.Command(exe, args...)
	cmd.Env = trafficCollectorEnv(cfg)
	cmd.Stdout = logFile
	cmd.Stderr = logFile
	configureDetachedCommand(cmd)
	if err := cmd.Start(); err != nil {
		return trafficCollectorStatusOutput{}, fmt.Errorf("start collector: %w", err)
	}
	state := trafficCollectorState{
		PID:       cmd.Process.Pid,
		StartedAt: timeNow(),
		Interval:  interval.String(),
		LogPath:   store.CollectorLogPath(),
		Command:   append([]string{exe}, args...),
	}
	if err := saveTrafficCollectorState(store, state); err != nil {
		_ = stopProcess(cmd.Process.Pid)
		return trafficCollectorStatusOutput{}, err
	}
	if err := cmd.Process.Release(); err != nil {
		ilog.Warn("collector process release failed: %v", err)
	}
	return collectorStateToStatus(state), nil
}

func stopTrafficCollector(cfg *config.EnvConfig) (trafficCollectorStatusOutput, error) {
	store := traffic.NewConfigStore(cfg)
	status, err := trafficCollectorSnapshot(store)
	if err != nil {
		return trafficCollectorStatusOutput{}, err
	}
	if status.PID == 0 {
		return status, nil
	}
	if !status.Running {
		if err := os.Remove(store.CollectorPath()); err != nil && !errors.Is(err, os.ErrNotExist) {
			return trafficCollectorStatusOutput{}, err
		}
		return status, nil
	}
	if err := stopProcess(status.PID); err != nil {
		return trafficCollectorStatusOutput{}, fmt.Errorf("stop collector pid %d: %w", status.PID, err)
	}
	deadline := time.Now().Add(3 * time.Second)
	for processRunning(status.PID) && time.Now().Before(deadline) {
		time.Sleep(100 * time.Millisecond)
	}
	if processRunning(status.PID) {
		return trafficCollectorStatusOutput{}, fmt.Errorf("collector pid %d did not stop", status.PID)
	}
	if err := os.Remove(store.CollectorPath()); err != nil && !errors.Is(err, os.ErrNotExist) {
		return trafficCollectorStatusOutput{}, err
	}
	return status, nil
}

func trafficCollectorSnapshot(store traffic.Store) (trafficCollectorStatusOutput, error) {
	state, ok, err := loadTrafficCollectorState(store)
	if err != nil || !ok {
		return trafficCollectorStatusOutput{LogPath: store.CollectorLogPath()}, err
	}
	status := collectorStateToStatus(state)
	status.Running = processRunning(state.PID)
	status.Stale = state.PID > 0 && !status.Running
	return status, nil
}

func collectorStateToStatus(state trafficCollectorState) trafficCollectorStatusOutput {
	status := trafficCollectorStatusOutput{
		Running:  true,
		PID:      state.PID,
		Interval: state.Interval,
		LogPath:  state.LogPath,
		Command:  append([]string(nil), state.Command...),
	}
	if !state.StartedAt.IsZero() {
		status.StartedAt = state.StartedAt.Format(profileTimeFormat)
	}
	return status
}

func loadTrafficCollectorState(store traffic.Store) (trafficCollectorState, bool, error) {
	data, err := os.ReadFile(store.CollectorPath())
	if err != nil {
		if errors.Is(err, os.ErrNotExist) {
			return trafficCollectorState{}, false, nil
		}
		return trafficCollectorState{}, false, fmt.Errorf("read traffic collector state: %w", err)
	}
	var state trafficCollectorState
	if err := json.Unmarshal(data, &state); err != nil {
		return trafficCollectorState{}, false, fmt.Errorf("parse traffic collector state: %w", err)
	}
	if state.LogPath == "" {
		state.LogPath = store.CollectorLogPath()
	}
	return state, true, nil
}

func saveTrafficCollectorState(store traffic.Store, state trafficCollectorState) error {
	data, err := json.MarshalIndent(state, "", "  ")
	if err != nil {
		return err
	}
	return config.AtomicWriteFile(store.CollectorPath(), append(data, '\n'), 0o644)
}

func trafficCollectorArgs(interval time.Duration) []string {
	return []string{"traffic", "collect", "--daemon", "--interval", interval.String()}
}

func trafficCollectorEnv(cfg *config.EnvConfig) []string {
	env := append([]string(nil), os.Environ()...)
	env = append(env,
		"CLASH_BASE_DIR="+cfg.ClashBaseDir,
		"KERNEL_NAME="+cfg.KernelName,
		"SERVICE_NAME="+cfg.ServiceName,
		"CLASH_SERVICE_NAME="+cfg.ServiceName,
		"INIT_TYPE="+cfg.InitType,
	)
	return env
}

func printTrafficCollectorStatus(status trafficCollectorStatusOutput) {
	switch {
	case status.Running:
		ilog.Ok("traffic collector: running")
		ilog.Info("pid: %d", status.PID)
	case status.Stale:
		ilog.Warn("traffic collector: stale pid %d", status.PID)
	default:
		ilog.Warn("traffic collector: stopped")
	}
	if status.Interval != "" {
		ilog.Info("interval: %s", status.Interval)
	}
	if status.StartedAt != "" {
		ilog.Info("started: %s", status.StartedAt)
	}
	if status.LogPath != "" {
		ilog.Info("log: %s", status.LogPath)
	}
}
