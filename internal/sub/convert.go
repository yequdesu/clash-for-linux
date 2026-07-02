package sub

import (
	"bytes"
	"fmt"
	"io"
	"net"
	"net/http"
	"net/url"
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"strings"
	"time"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
	"gopkg.in/yaml.v3"
)

const defaultSubconverterPort = "25500"

func ConvertDownload(cfg *config.EnvConfig, subURL, dest string) error {
	if strings.HasPrefix(subURL, "file://") {
		return nil
	}

	client := &http.Client{Timeout: 2 * time.Second}
	port, alreadyRunning, err := selectSubconverterPort(client, defaultSubconverterPort)
	if err != nil {
		return err
	}

	subStarted := false
	var managed *managedSubconverter
	if !alreadyRunning {
		cmd, err := startManagedSubconverter(cfg, port)
		if err != nil {
			return err
		}
		managed = cmd
		subStarted = true
		defer func() {
			if subStarted {
				stopManagedSubconverter(managed)
			}
		}()

		deadline := time.Now().Add(3 * time.Second)
		for !isSubconverterAlive(client, port) {
			if time.Now().After(deadline) {
				stopManagedSubconverter(managed)
				return fmt.Errorf("subconverter failed to start: %s", tailFile(filepath.Join(cfg.SubconverterDir(), "latest.log"), 4096))
			}
			time.Sleep(300 * time.Millisecond)
		}
	}

	convertURL := fmt.Sprintf("http://127.0.0.1:%s/sub?target=clash&url=%s",
		port, url.QueryEscape(subURL))
	ilog.Info("converting via subconverter...")
	return httpDownload(convertURL, dest, cfg.ClashSubUA)
}

func selectSubconverterPort(client *http.Client, port string) (string, bool, error) {
	if isSubconverterAlive(client, port) {
		return port, true, nil
	}
	if tcpPortOpen(port) {
		free, err := freeTCPPort()
		if err != nil {
			return "", false, fmt.Errorf("subconverter port %s is occupied and no free fallback port was found: %w", port, err)
		}
		ilog.Warn("subconverter port %s is occupied by another service; using %s for managed converter", port, free)
		return free, false, nil
	}
	return port, false, nil
}

func isSubconverterAlive(client *http.Client, port string) bool {
	resp, err := client.Get(fmt.Sprintf("http://127.0.0.1:%s/version", port))
	if err != nil {
		return false
	}
	defer resp.Body.Close()
	io.Copy(io.Discard, io.LimitReader(resp.Body, 512))
	return resp.StatusCode == http.StatusOK
}

func tcpPortOpen(port string) bool {
	conn, err := net.DialTimeout("tcp", net.JoinHostPort("127.0.0.1", port), 300*time.Millisecond)
	if err != nil {
		return false
	}
	conn.Close()
	return true
}

func freeTCPPort() (string, error) {
	ln, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		return "", err
	}
	defer ln.Close()
	addr, ok := ln.Addr().(*net.TCPAddr)
	if !ok {
		return "", fmt.Errorf("unexpected listener address: %s", ln.Addr())
	}
	return strconv.Itoa(addr.Port), nil
}

type managedSubconverter struct {
	cmd     *exec.Cmd
	pidFile string
	restore func()
}

func startManagedSubconverter(cfg *config.EnvConfig, port string) (*managedSubconverter, error) {
	if _, err := os.Stat(cfg.SubconverterBin()); err != nil {
		return nil, fmt.Errorf("subconverter binary not found: %s", cfg.SubconverterBin())
	}
	if err := os.MkdirAll(cfg.SubconverterDir(), 0755); err != nil {
		return nil, err
	}
	restore, err := writeManagedSubconverterPref(cfg, port)
	if err != nil {
		return nil, err
	}

	logPath := filepath.Join(cfg.SubconverterDir(), "latest.log")
	logFile, err := os.OpenFile(logPath, os.O_CREATE|os.O_WRONLY|os.O_APPEND, 0644)
	if err != nil {
		restore()
		return nil, err
	}
	cmd := exec.Command(cfg.SubconverterBin())
	cmd.Dir = cfg.SubconverterDir()
	cmd.Stdout = logFile
	cmd.Stderr = logFile
	if err := cmd.Start(); err != nil {
		logFile.Close()
		restore()
		return nil, fmt.Errorf("subconverter start failed: %w: %s", err, tailFile(logPath, 4096))
	}
	logFile.Close()
	pidFile := filepath.Join(cfg.SubconverterDir(), "subconverter.pid")
	_ = config.AtomicWriteFile(pidFile, []byte(strconv.Itoa(cmd.Process.Pid)+"\n"), 0644)
	return &managedSubconverter{cmd: cmd, pidFile: pidFile, restore: restore}, nil
}

func writeManagedSubconverterPref(cfg *config.EnvConfig, port string) (func(), error) {
	prefPath := cfg.SubconverterConfig()
	backupPath := prefPath + ".clashctl.bak"
	content, err := os.ReadFile(prefPath)
	existed := err == nil
	if err != nil && !os.IsNotExist(err) {
		return nil, err
	}
	if existed {
		if err := config.AtomicWriteFile(backupPath, content, 0644); err != nil {
			return nil, err
		}
	}

	next, err := updateSubconverterPref(content, port)
	if err != nil {
		if existed {
			_ = os.Remove(backupPath)
		}
		return nil, err
	}
	if err := config.AtomicWriteFile(prefPath, next, 0644); err != nil {
		if existed {
			_ = os.Remove(backupPath)
		}
		return nil, err
	}

	return func() {
		if existed {
			backup, err := os.ReadFile(backupPath)
			if err == nil {
				_ = config.AtomicWriteFile(prefPath, backup, 0644)
			}
			_ = os.Remove(backupPath)
			return
		}
		_ = os.Remove(prefPath)
		_ = os.Remove(backupPath)
	}, nil
}

func updateSubconverterPref(content []byte, port string) ([]byte, error) {
	p, err := strconv.Atoi(port)
	if err != nil || p <= 0 || p > 65535 {
		return nil, fmt.Errorf("invalid subconverter port: %s", port)
	}

	var pref map[string]any
	if strings.TrimSpace(string(content)) != "" {
		if err := yaml.Unmarshal(content, &pref); err != nil {
			return nil, fmt.Errorf("parse subconverter pref.yml: %w", err)
		}
	}
	if pref == nil {
		pref = map[string]any{}
	}
	server, ok := pref["server"].(map[string]any)
	if !ok {
		server = map[string]any{}
	}
	server["listen"] = "127.0.0.1"
	server["port"] = p
	pref["server"] = server
	managedConfig, ok := pref["managed_config"].(map[string]any)
	if !ok {
		managedConfig = map[string]any{}
	}
	managedConfig["managed_config_prefix"] = "http://127.0.0.1:" + port
	pref["managed_config"] = managedConfig

	var buf bytes.Buffer
	enc := yaml.NewEncoder(&buf)
	enc.SetIndent(2)
	if err := enc.Encode(pref); err != nil {
		_ = enc.Close()
		return nil, err
	}
	if err := enc.Close(); err != nil {
		return nil, err
	}
	return buf.Bytes(), nil
}

func stopManagedSubconverter(managed *managedSubconverter) {
	if managed == nil {
		return
	}
	if managed.cmd != nil && managed.cmd.Process != nil {
		_ = managed.cmd.Process.Signal(os.Interrupt)
		done := make(chan struct{})
		go func() {
			_ = managed.cmd.Wait()
			close(done)
		}()
		select {
		case <-done:
		case <-time.After(2 * time.Second):
			_ = managed.cmd.Process.Kill()
			<-done
		}
	}
	if managed.pidFile != "" {
		_ = os.Remove(managed.pidFile)
	}
	if managed.restore != nil {
		managed.restore()
	}
}

func tailFile(path string, limit int64) string {
	data, err := os.ReadFile(path)
	if err != nil {
		return ""
	}
	if int64(len(data)) > limit {
		data = data[int64(len(data))-limit:]
	}
	return strings.TrimSpace(string(data))
}
