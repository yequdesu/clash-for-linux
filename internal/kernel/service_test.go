package kernel

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"testing"
	"time"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
)

func TestStopNohupDoesNotKillUnmatchedPidFile(t *testing.T) {
	if runtime.GOOS != "linux" {
		t.Skip("requires /proc")
	}
	cfg := testServiceConfig(t)
	if err := os.WriteFile(cfg.PidFile(), []byte(fmt.Sprintf("%d\n", os.Getpid())), 0o644); err != nil {
		t.Fatal(err)
	}

	svc := &ServiceManager{cfg: cfg, initType: "nohup"}
	if err := svc.stopNohup(); err != nil {
		t.Fatalf("stopNohup: %v", err)
	}
	if _, err := os.Stat(cfg.PidFile()); !os.IsNotExist(err) {
		t.Fatalf("pid file should be removed for unmatched pid, err=%v", err)
	}
	if !processAlive(os.Getpid()) {
		t.Fatal("current process was unexpectedly considered dead")
	}
}

func TestStopNohupStopsMatchedPidFile(t *testing.T) {
	if runtime.GOOS != "linux" {
		t.Skip("requires /proc")
	}
	sleepPath, err := exec.LookPath("sleep")
	if err != nil {
		t.Skip("sleep binary is required")
	}
	cfg := testServiceConfig(t)
	cfg.KernelName = "sleep"
	sleepBytes, err := os.ReadFile(sleepPath)
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(cfg.KernelBin(), sleepBytes, 0o755); err != nil {
		t.Fatal(err)
	}

	cmd := exec.Command(cfg.KernelBin(), "60")
	cmd.SysProcAttr = processAttrs()
	if err := cmd.Start(); err != nil {
		t.Fatal(err)
	}
	done := make(chan error, 1)
	go func() {
		done <- cmd.Wait()
	}()
	exited := false
	t.Cleanup(func() {
		if !exited && cmd.Process != nil {
			_ = cmd.Process.Kill()
			<-done
		}
	})

	if err := os.WriteFile(cfg.PidFile(), []byte(fmt.Sprintf("%d\n", cmd.Process.Pid)), 0o644); err != nil {
		t.Fatal(err)
	}
	svc := &ServiceManager{cfg: cfg, initType: "nohup"}
	if err := svc.stopNohup(); err != nil {
		t.Fatalf("stopNohup: %v", err)
	}
	select {
	case <-done:
		exited = true
	case <-time.After(3 * time.Second):
		t.Fatal("managed process did not exit")
	}
	if _, err := os.Stat(cfg.PidFile()); !os.IsNotExist(err) {
		t.Fatalf("pid file should be removed for stopped pid, err=%v", err)
	}
}

func TestIsRunningNohupRejectsUnmatchedPidFile(t *testing.T) {
	if runtime.GOOS != "linux" {
		t.Skip("requires /proc")
	}
	cfg := testServiceConfig(t)
	if err := os.WriteFile(cfg.PidFile(), []byte(fmt.Sprintf("%d\n", os.Getpid())), 0o644); err != nil {
		t.Fatal(err)
	}

	svc := &ServiceManager{cfg: cfg, initType: "nohup"}
	if svc.isRunningNohup() {
		t.Fatal("isRunningNohup accepted pid file for a non-kernel process")
	}
}

func TestProcessMatchesExecutableCurrentProcess(t *testing.T) {
	if runtime.GOOS != "linux" {
		t.Skip("requires /proc")
	}
	exe, err := os.Executable()
	if err != nil {
		t.Fatal(err)
	}
	if !processMatchesExecutable(os.Getpid(), exe) {
		t.Fatalf("processMatchesExecutable rejected current executable %s", exe)
	}
}

func TestProcessAliveCurrentProcess(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("process liveness helper is only used by Unix fallback")
	}
	if !processAlive(os.Getpid()) {
		t.Fatal("processAlive rejected current process")
	}
}

func TestDetectInitUsesExplicitConfig(t *testing.T) {
	t.Setenv("CLASH_INIT_TYPE", "")
	t.Setenv("INIT_TYPE", "")

	cfg := &config.EnvConfig{InitType: "systemd", ServiceName: "clashctl"}
	if got := detectInit(cfg); got != "systemd" {
		t.Fatalf("detectInit = %q, want systemd", got)
	}
}

func TestDetectInitEnvOverridesConfig(t *testing.T) {
	t.Setenv("CLASH_INIT_TYPE", "nohup")
	t.Setenv("INIT_TYPE", "")

	cfg := &config.EnvConfig{InitType: "systemd", ServiceName: "clashctl"}
	if got := detectInit(cfg); got != "nohup" {
		t.Fatalf("detectInit = %q, want nohup", got)
	}
}

func TestProcessAliveTreatsZombieAsExited(t *testing.T) {
	if runtime.GOOS != "linux" {
		t.Skip("requires /proc")
	}
	cmd := exec.Command("sh", "-c", "exit 0")
	if err := cmd.Start(); err != nil {
		t.Fatal(err)
	}
	defer func() {
		_ = cmd.Wait()
	}()

	deadline := time.Now().Add(3 * time.Second)
	for time.Now().Before(deadline) {
		if linuxProcessState(cmd.Process.Pid) == 'Z' {
			if processAlive(cmd.Process.Pid) {
				t.Fatal("processAlive reported a zombie process as alive")
			}
			return
		}
		time.Sleep(20 * time.Millisecond)
	}
	t.Fatal("test process did not become zombie before timeout")
}

func TestFindKernelPIDScansProcWithoutPgrep(t *testing.T) {
	if runtime.GOOS != "linux" {
		t.Skip("requires /proc")
	}
	exe, err := os.Executable()
	if err != nil {
		t.Fatal(err)
	}
	if got := findKernelPID(exe); got != os.Getpid() {
		t.Fatalf("findKernelPID(%s) = %d, want current pid %d", exe, got, os.Getpid())
	}
}

func testServiceConfig(t *testing.T) *config.EnvConfig {
	t.Helper()
	base := t.TempDir()
	cfg := &config.EnvConfig{ClashBaseDir: base, KernelName: "mihomo", ServiceName: "clashctl"}
	if err := os.MkdirAll(filepath.Dir(cfg.PidFile()), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.MkdirAll(cfg.BinDir(), 0o755); err != nil {
		t.Fatal(err)
	}
	return cfg
}
