//go:build windows

package main

import (
	"fmt"
	"os/exec"
	"strings"
)

func configureDetachedCommand(cmd *exec.Cmd) {
	_ = cmd
}

func processRunning(pid int) bool {
	if pid <= 0 {
		return false
	}
	out, err := exec.Command("tasklist", "/FI", fmt.Sprintf("PID eq %d", pid), "/FO", "CSV", "/NH").Output()
	if err != nil {
		return false
	}
	return strings.Contains(string(out), fmt.Sprintf("\"%d\"", pid))
}

func stopProcess(pid int) error {
	return exec.Command("taskkill", "/PID", fmt.Sprintf("%d", pid), "/T", "/F").Run()
}
