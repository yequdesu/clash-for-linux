//go:build windows

package kernel

import (
	"os"
	"syscall"
)

func processAttrs() *syscall.SysProcAttr {
	return nil
}

func processAliveNative(int) bool {
	return false
}

func terminateProcess(pid int) error {
	proc, err := os.FindProcess(pid)
	if err != nil {
		return err
	}
	return proc.Kill()
}

func killProcess(pid int) error {
	proc, err := os.FindProcess(pid)
	if err != nil {
		return err
	}
	return proc.Kill()
}
