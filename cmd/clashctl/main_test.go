package main

import (
	"io"
	"testing"

	"github.com/spf13/cobra"
)

func TestShowHelpAndExitUsesNonZeroExit(t *testing.T) {
	oldExit := exitProcess
	defer func() { exitProcess = oldExit }()

	exitCode := -1
	exitProcess = func(code int) {
		exitCode = code
	}

	cmd := &cobra.Command{Use: "test"}
	cmd.SetOut(io.Discard)
	cmd.SetErr(io.Discard)
	showHelpAndExit(cmd)

	if exitCode != 1 {
		t.Fatalf("exit code = %d, want 1", exitCode)
	}
}
