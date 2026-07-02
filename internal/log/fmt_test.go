package log

import (
	"os"
	"testing"
)

func TestWithQuietSuppressesOutputAndRestoresState(t *testing.T) {
	stdout, stderr := captureStdoutStderr(t, func() {
		if err := WithQuiet(true, func() error {
			Info("info")
			Ok("ok")
			Warn("warn")
			Section("section")
			return nil
		}); err != nil {
			t.Fatal(err)
		}
	})
	if stdout != "" || stderr != "" {
		t.Fatalf("quiet output stdout=%q stderr=%q, want silent", stdout, stderr)
	}

	stdout, _ = captureStdoutStderr(t, func() {
		Info("visible")
	})
	if stdout == "" {
		t.Fatal("WithQuiet did not restore visible output")
	}
}

func captureStdoutStderr(t *testing.T, fn func()) (string, string) {
	t.Helper()
	oldStdout := os.Stdout
	oldStderr := os.Stderr
	stdoutReader, stdoutWriter, err := os.Pipe()
	if err != nil {
		t.Fatal(err)
	}
	stderrReader, stderrWriter, err := os.Pipe()
	if err != nil {
		t.Fatal(err)
	}
	os.Stdout = stdoutWriter
	os.Stderr = stderrWriter
	defer func() {
		os.Stdout = oldStdout
		os.Stderr = oldStderr
	}()

	fn()

	if err := stdoutWriter.Close(); err != nil {
		t.Fatal(err)
	}
	if err := stderrWriter.Close(); err != nil {
		t.Fatal(err)
	}
	stdoutBytes := make([]byte, 4096)
	stderrBytes := make([]byte, 4096)
	stdoutN, _ := stdoutReader.Read(stdoutBytes)
	stderrN, _ := stderrReader.Read(stderrBytes)
	_ = stdoutReader.Close()
	_ = stderrReader.Close()
	return string(stdoutBytes[:stdoutN]), string(stderrBytes[:stderrN])
}
