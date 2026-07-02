package main

import (
	"fmt"
	"os"
	"os/exec"
	"strings"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
)

func allowSSHTunRisk(flag bool) bool {
	if flag {
		return true
	}
	switch strings.ToLower(strings.TrimSpace(os.Getenv("CLASH_ALLOW_SSH_TUN_RISK"))) {
	case "1", "true", "yes", "on":
		return true
	default:
		return false
	}
}

func ensureSafeKernelStartFromSSH(cfg *config.EnvConfig, allowFlag bool, forceTunEnable bool) error {
	if allowSSHTunRisk(allowFlag) {
		return nil
	}
	client, ssh := sshSessionClient()
	if !ssh {
		return nil
	}

	info, err := config.ReadRuntimeInfo(cfg.RuntimePath())
	if err != nil && !forceTunEnable {
		return nil
	}
	if !tunStartCanCaptureRoutes(info, forceTunEnable) {
		return nil
	}

	route := "unknown"
	if client != "" {
		if out, err := exec.Command("ip", "route", "get", client).CombinedOutput(); err == nil {
			route = strings.TrimSpace(string(out))
		}
	}
	if client == "" {
		client = "unknown"
	}

	return fmt.Errorf("refusing to start TUN auto-route from an SSH session\nssh client: %s\ncurrent route: %s\nrun from a local console or pass --allow-ssh-tun-risk after confirming a recovery path", client, route)
}

func sshSessionClient() (string, bool) {
	fields := strings.Fields(os.Getenv("SSH_CONNECTION"))
	if len(fields) > 0 {
		return fields[0], true
	}
	if os.Getenv("SSH_TTY") != "" {
		return "", true
	}
	return "", false
}

func tunStartCanCaptureRoutes(info config.RuntimeInfo, forceTunEnable bool) bool {
	if forceTunEnable {
		return true
	}
	return info.TunEnabled && (info.TunAutoRoute || info.TunStrictRoute)
}
