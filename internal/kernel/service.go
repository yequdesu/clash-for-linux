package kernel

import (
	"fmt"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strconv"
	"strings"
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
	if s.initType == "systemd" && unitExists(s.cfg.ServiceName+".service") {
		if err := runCmd("systemctl", "start", s.cfg.ServiceName); err != nil {
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
	if s.initType == "systemd" && unitExists(s.cfg.ServiceName+".service") {
		if err := runCmd("systemctl", "stop", s.cfg.ServiceName); err != nil {
			return err
		}
		if s.isRunningNohup() {
			return s.stopNohup()
		}
		return nil
	}
	return s.stopNohup()
}

func (s *ServiceManager) IsRunning() bool {
	if s.initType == "systemd" && unitExists(s.cfg.ServiceName+".service") {
		return runCmd("systemctl", "is-active", "--quiet", s.cfg.ServiceName) == nil || s.isRunningNohup()
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
		runCmd("systemctl", "enable", s.cfg.ServiceName)
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
		runCmd("systemctl", "disable", s.cfg.ServiceName)
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
	_ = os.MkdirAll(s.cfg.LogDir(), 0755)
	_ = os.MkdirAll(filepath.Dir(s.cfg.PidFile()), 0755)
	logFile := s.cfg.LogFile()
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
	cmd.SysProcAttr = processAttrs()
	if err := cmd.Start(); err != nil {
		if logWriter != nil {
			_ = logWriter.Close()
		}
		return err
	}
	if logWriter != nil {
		_ = logWriter.Close()
	}
	_ = config.AtomicWriteFile(s.cfg.PidFile(), []byte(fmt.Sprintf("%d\n", cmd.Process.Pid)), 0644)
	return nil
}

func (s *ServiceManager) waitReady(timeout time.Duration) error {
	info := config.LoadRuntimeInfo(s.cfg)
	proxyPort := info.ProxyPort
	if proxyPort == "" {
		proxyPort = "7890"
	}
	apiAddress := info.APIAddress("9090")
	proxyAddress := net.JoinHostPort("127.0.0.1", proxyPort)
	deadline := time.Now().Add(timeout)
	apiReady := false
	for time.Now().Before(deadline) {
		if !apiReady {
			if conn, err := net.DialTimeout("tcp", apiAddress, 500*time.Millisecond); err == nil {
				conn.Close()
				apiReady = true
			}
		}
		if conn, err := net.DialTimeout("tcp", proxyAddress, 500*time.Millisecond); err == nil {
			conn.Close()
			return nil
		}
		if !s.isRunningForWait() {
			return fmt.Errorf("kernel process died, check 'clashctl log'")
		}
		time.Sleep(500 * time.Millisecond)
	}
	if apiReady {
		return fmt.Errorf("timeout waiting for kernel proxy port :%s (API %s ready)", proxyPort, apiAddress)
	}
	return fmt.Errorf("timeout waiting for kernel on API %s, proxy :%s", apiAddress, proxyPort)
}

func (s *ServiceManager) isRunningForWait() bool {
	if s.initType == "systemd" && unitExists(s.cfg.ServiceName+".service") {
		return runCmd("systemctl", "is-active", "--quiet", s.cfg.ServiceName) == nil || s.isRunningNohup()
	}
	return s.isRunningNohup()
}

func (s *ServiceManager) stopNohup() error {
	pid, err := s.pidFromFile()
	if err != nil {
		if s.isRunningNohup() {
			return fmt.Errorf("kernel appears to be running but pid file is missing: %s", s.cfg.PidFile())
		}
		return nil
	}
	if !processMatchesExecutable(pid, s.cfg.KernelBin()) {
		_ = os.Remove(s.cfg.PidFile())
		return nil
	}
	if err := stopProcessByPID(pid, s.cfg.PidFile()); err != nil {
		return err
	}
	return nil
}

func (s *ServiceManager) isRunningNohup() bool {
	if pid, err := s.pidFromFile(); err == nil {
		return processAlive(pid) && processMatchesExecutable(pid, s.cfg.KernelBin())
	}
	return findKernelPID(s.cfg.KernelBin()) > 0
}

func (s *ServiceManager) ProxyPort() string {
	port := config.LoadRuntimeInfo(s.cfg).ProxyPort
	if port == "" {
		return "7890"
	}
	return port
}

func (s *ServiceManager) PID() int {
	if pid, err := s.pidFromFile(); err == nil && processAlive(pid) && processMatchesExecutable(pid, s.cfg.KernelBin()) {
		return pid
	}
	return findKernelPID(s.cfg.KernelBin())
}

func findKernelPID(kernelBin string) int {
	if runtime.GOOS == "windows" || kernelBin == "" {
		return 0
	}
	entries, err := os.ReadDir("/proc")
	if err != nil {
		return 0
	}
	for _, entry := range entries {
		if !entry.IsDir() {
			continue
		}
		pid, err := strconv.Atoi(entry.Name())
		if err != nil {
			continue
		}
		if processMatchesExecutable(pid, kernelBin) {
			return pid
		}
	}
	return 0
}

func (s *ServiceManager) pidFromFile() (int, error) {
	data, err := os.ReadFile(s.cfg.PidFile())
	if err != nil {
		return 0, err
	}
	pid, err := strconv.Atoi(strings.TrimSpace(string(data)))
	if err != nil || pid <= 0 {
		return 0, fmt.Errorf("invalid pid file: %s", s.cfg.PidFile())
	}
	return pid, nil
}

func stopProcessByPID(pid int, pidFile string) error {
	if !processAlive(pid) {
		_ = os.Remove(pidFile)
		return nil
	}
	if err := terminateProcess(pid); err != nil {
		return fmt.Errorf("send SIGTERM to pid %d: %w", pid, err)
	}
	if waitForProcessExit(pid, 5*time.Second) {
		_ = os.Remove(pidFile)
		return nil
	}
	if err := killProcess(pid); err != nil {
		return fmt.Errorf("send SIGKILL to pid %d: %w", pid, err)
	}
	if !waitForProcessExit(pid, 2*time.Second) {
		return fmt.Errorf("pid %d still running after SIGKILL", pid)
	}
	_ = os.Remove(pidFile)
	return nil
}

func waitForProcessExit(pid int, timeout time.Duration) bool {
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		if !processAlive(pid) {
			return true
		}
		time.Sleep(200 * time.Millisecond)
	}
	return !processAlive(pid)
}

func processAlive(pid int) bool {
	if pid <= 0 {
		return false
	}
	if runtime.GOOS == "linux" && linuxProcessState(pid) == 'Z' {
		return false
	}
	return processAliveNative(pid)
}

func linuxProcessState(pid int) byte {
	data, err := os.ReadFile(fmt.Sprintf("/proc/%d/stat", pid))
	if err != nil {
		return 0
	}
	closeParen := strings.LastIndex(string(data), ") ")
	if closeParen < 0 || closeParen+2 >= len(data) {
		return 0
	}
	return data[closeParen+2]
}

func processMatchesExecutable(pid int, expectedPath string) bool {
	if pid <= 0 || expectedPath == "" || runtime.GOOS != "linux" {
		return false
	}
	expected := cleanExecutablePath(expectedPath)
	if expected == "" {
		return false
	}
	exe, err := os.Readlink(fmt.Sprintf("/proc/%d/exe", pid))
	if err == nil && executablePathMatches(exe, expected, expectedPath) {
		return true
	}
	cmdline, err := os.ReadFile(fmt.Sprintf("/proc/%d/cmdline", pid))
	if err != nil || len(cmdline) == 0 {
		return false
	}
	cmd0 := strings.SplitN(string(cmdline), "\x00", 2)[0]
	return executablePathMatches(cmd0, expected, expectedPath)
}

func executablePathMatches(actual, expected, rawExpected string) bool {
	actual = strings.TrimSuffix(actual, " (deleted)")
	if actual == expected || actual == rawExpected {
		return true
	}
	return cleanExecutablePath(actual) == expected
}

func cleanExecutablePath(path string) string {
	if path == "" {
		return ""
	}
	if resolved, err := filepath.EvalSymlinks(path); err == nil {
		return resolved
	}
	if abs, err := filepath.Abs(path); err == nil {
		return abs
	}
	return path
}

func (s *ServiceManager) Uptime() string {
	pid := s.PID()
	if pid == 0 {
		return ""
	}
	// starttime (field 22) is clock ticks since system boot, NOT epoch
	data, err := os.ReadFile(fmt.Sprintf("/proc/%d/stat", pid))
	if err != nil {
		return ""
	}
	fields := strings.Fields(string(data))
	if len(fields) < 22 {
		return ""
	}
	// Find starttime — it's after the `)` which may contain spaces (comm field)
	// The stat format is: pid (comm) state ... the 22nd field after comm is starttime
	closeParen := strings.LastIndex(string(data), ")")
	if closeParen < 0 {
		return ""
	}
	afterComm := strings.Fields(string(data)[closeParen+2:])
	if len(afterComm) < 20 {
		return ""
	}
	var startTicks uint64
	fmt.Sscanf(afterComm[19], "%d", &startTicks) // starttime is field 20 after comm
	ticksPerSec := int64(100)

	// Read system uptime to compute boot time
	uptimeData, _ := os.ReadFile("/proc/uptime")
	var sysUptime float64
	fmt.Sscanf(string(uptimeData), "%f", &sysUptime)

	nowSec := time.Now().Unix()
	bootTime := nowSec - int64(sysUptime)
	startSec := bootTime + int64(startTicks)/ticksPerSec

	uptime := time.Duration(nowSec-startSec) * time.Second
	hours := int(uptime.Hours())
	minutes := int(uptime.Minutes()) % 60
	if hours > 0 {
		return fmt.Sprintf("%dh %dm", hours, minutes)
	}
	return fmt.Sprintf("%dm", minutes)
}
