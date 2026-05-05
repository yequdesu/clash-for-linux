package kernel

import (
	"fmt"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"syscall"
	"time"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
)

type ServiceManager struct {
	cfg      *config.EnvConfig
	initType string
}

func NewServiceManager(cfg *config.EnvConfig) *ServiceManager {
	return &ServiceManager{
		cfg:      cfg,
		initType: detectInit(),
	}
}

func detectInit() string {
	if initType := os.Getenv("CLASH_INIT_TYPE"); initType != "" {
		return initType
	}
	if !hasSystemd() {
		return "nohup"
	}
	if isContainer() {
		return "nohup"
	}
	return "systemd"
}

func hasSystemd() bool {
	exe, err := os.Readlink("/proc/1/exe")
	if err != nil {
		return false
	}
	return strings.Contains(filepath.Base(exe), "systemd")
}

func isContainer() bool {
	data, err := os.ReadFile("/proc/1/cgroup")
	if err != nil {
		return false
	}
	markers := []string{"docker", "kubepods", "containerd", "podman", "lxc"}
	for _, m := range markers {
		if strings.Contains(string(data), m) {
			return true
		}
	}
	return false
}

func (s *ServiceManager) InitType() string { return s.initType }

func (s *ServiceManager) Start() error {
	if s.initType == "systemd" && unitExists(s.cfg.KernelName+".service") {
		if err := runCmd("systemctl", "start", s.cfg.KernelName); err != nil {
			return err
		}
		return s.waitReady(15 * time.Second)
	}
	if err := s.startRaw(); err != nil {
		return err
	}
	return s.waitReady(15 * time.Second)
}

func (s *ServiceManager) Stop() error {
	if s.initType == "systemd" && unitExists(s.cfg.KernelName+".service") {
		return runCmd("systemctl", "stop", s.cfg.KernelName)
	}
	return s.stopNohup()
}

func (s *ServiceManager) IsRunning() bool {
	if s.initType == "systemd" && unitExists(s.cfg.KernelName+".service") {
		return runCmd("systemctl", "is-active", "--quiet", s.cfg.KernelName) == nil
	}
	return s.isRunningNohup()
}

func unitExists(name string) bool {
	_, err := os.Stat("/etc/systemd/system/" + name)
	if err == nil {
		return true
	}
	_, err = os.Stat("/lib/systemd/system/" + name)
	return err == nil
}

func (s *ServiceManager) DetectAndEnable() error {
	switch s.initType {
	case "systemd":
		runCmd("systemctl", "enable", s.cfg.KernelName)
	case "sysvinit":
		if _, err := exec.LookPath("chkconfig"); err == nil {
			runCmd("chkconfig", s.cfg.KernelName, "on")
		} else if _, err := exec.LookPath("update-rc.d"); err == nil {
			runCmd("update-rc.d", s.cfg.KernelName, "enable")
		}
	case "openrc":
		runCmd("rc-update", "add", s.cfg.KernelName, "default")
	case "runit":
		src := filepath.Join("/etc/sv", s.cfg.KernelName)
		dst := filepath.Join("/etc/runit/runsvdir/default", s.cfg.KernelName)
		if _, err := os.Stat(src); err == nil {
			os.Symlink(src, dst)
		}
	}
	return nil
}

func (s *ServiceManager) Disable() error {
	switch s.initType {
	case "systemd":
		runCmd("systemctl", "disable", s.cfg.KernelName)
	case "sysvinit":
		if _, err := exec.LookPath("chkconfig"); err == nil {
			runCmd("chkconfig", s.cfg.KernelName, "off")
		}
	case "openrc":
		runCmd("rc-update", "del", s.cfg.KernelName, "default")
	case "runit":
		os.Remove(filepath.Join("/etc/runit/runsvdir/default", s.cfg.KernelName))
	}
	return nil
}

func runCmd(name string, args ...string) error {
	cmd := exec.Command(name, args...)
	cmd.Stdout = nil
	cmd.Stderr = nil
	return cmd.Run()
}

func (s *ServiceManager) startRaw() error {
	logFile := filepath.Join(s.cfg.ResourcesDir(), s.cfg.KernelName+".log")
	logWriter, _ := os.OpenFile(logFile, os.O_CREATE|os.O_WRONLY|os.O_APPEND, 0644)

	cmd := exec.Command(s.cfg.KernelBin(),
		"-d", s.cfg.ResourcesDir(),
		"-f", s.cfg.RuntimePath(),
	)
	if logWriter != nil {
		cmd.Stdout = logWriter
		cmd.Stderr = logWriter
	}
	cmd.Stdin = nil
	cmd.SysProcAttr = &syscall.SysProcAttr{Setpgid: true}
	return cmd.Start()
}

func (s *ServiceManager) waitReady(timeout time.Duration) error {
	apiPort := s.readAPIPort()
	proxyPort := s.readProxyPort()
	if apiPort == "" {
		apiPort = "9090"
	}
	if proxyPort == "" {
		proxyPort = "7890"
	}
	deadline := time.Now().Add(timeout)
	apiReady := false
	for time.Now().Before(deadline) {
		if !apiReady {
			if conn, err := net.DialTimeout("tcp", "127.0.0.1:"+apiPort, 500*time.Millisecond); err == nil {
				conn.Close()
				apiReady = true
			}
		}
		if conn, err := net.DialTimeout("tcp", "127.0.0.1:"+proxyPort, 500*time.Millisecond); err == nil {
			conn.Close()
			return nil
		}
		if !s.isRunningNohup() {
			return fmt.Errorf("kernel process died, check 'clashctl log'")
		}
		time.Sleep(500 * time.Millisecond)
	}
	if apiReady {
		return fmt.Errorf("timeout waiting for kernel proxy port :%s (API port :%s ready)", proxyPort, apiPort)
	}
	return fmt.Errorf("timeout waiting for kernel on API :%s, proxy :%s", apiPort, proxyPort)
}

func (s *ServiceManager) stopNohup() error {
	exec.Command("pkill", "-9", "-f", s.cfg.KernelBin()).Run()
	exec.Command("pkill", "-9", s.cfg.KernelName).Run()
	time.Sleep(800 * time.Millisecond)
	return nil
}

func (s *ServiceManager) isRunningNohup() bool {
	return exec.Command("pgrep", "-f", s.cfg.KernelBin()).Run() == nil ||
		exec.Command("pgrep", s.cfg.KernelName).Run() == nil
}

func (s *ServiceManager) readProxyPort() string {
	data, err := os.ReadFile(s.cfg.RuntimePath())
	if err != nil {
		return ""
	}
	for _, line := range strings.Split(string(data), "\n") {
		line = strings.TrimSpace(line)
		if strings.HasPrefix(line, "mixed-port:") || strings.HasPrefix(line, "port:") {
			val := strings.TrimPrefix(line, "mixed-port:")
			val = strings.TrimPrefix(val, "port:")
			return strings.TrimSpace(strings.Trim(val, `"`))
		}
	}
	return ""
}

func (s *ServiceManager) readAPIPort() string {
	data, err := os.ReadFile(s.cfg.RuntimePath())
	if err != nil {
		return ""
	}
	for _, line := range strings.Split(string(data), "\n") {
		line = strings.TrimSpace(line)
		if strings.HasPrefix(line, "external-controller:") {
			addr := strings.TrimSpace(strings.TrimPrefix(line, "external-controller:"))
			addr = strings.Trim(addr, `"`)
			if idx := strings.LastIndex(addr, ":"); idx >= 0 {
				return strings.TrimSpace(addr[idx+1:])
			}
		}
	}
	return ""
}

func (s *ServiceManager) ProxyPort() string {
	port := s.readProxyPort()
	if port == "" {
		return "7890"
	}
	return port
}

func (s *ServiceManager) PID() int {
	out, err := exec.Command("pgrep", "-f", s.cfg.KernelBin()).Output()
	if err != nil {
		return 0
	}
	var pid int
	fmt.Sscanf(strings.TrimSpace(string(out)), "%d", &pid)
	return pid
}

func (s *ServiceManager) Uptime() string {
	pid := s.PID()
	if pid == 0 {
		return ""
	}
	data, err := os.ReadFile(fmt.Sprintf("/proc/%d/stat", pid))
	if err != nil {
		return ""
	}
	fields := strings.Fields(string(data))
	if len(fields) < 22 {
		return ""
	}
	var startTicks uint64
	fmt.Sscanf(fields[21], "%d", &startTicks)
	ticksPerSec := int64(100) // Linux default
	nowTicks := time.Now().Unix()
	startSec := int64(startTicks) / ticksPerSec
	if startSec <= 0 {
		return ""
	}
	uptime := time.Duration(nowTicks-startSec) * time.Second
	hours := int(uptime.Hours())
	minutes := int(uptime.Minutes()) % 60
	if hours > 0 {
		return fmt.Sprintf("%dh %dm", hours, minutes)
	}
	return fmt.Sprintf("%dm", minutes)
}
