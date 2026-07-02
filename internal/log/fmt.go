package log

import (
	"fmt"
	"os"
	"sync/atomic"
)

var quietDepth atomic.Int32

func WithQuiet(quiet bool, fn func() error) error {
	if !quiet {
		return fn()
	}
	quietDepth.Add(1)
	defer quietDepth.Add(-1)
	return fn()
}

func quiet() bool {
	return quietDepth.Load() > 0
}

func Ok(format string, args ...any) {
	if quiet() {
		return
	}
	fmt.Printf("\033[32m[+]\033[0m "+format+"\r\n", args...)
}

func Info(format string, args ...any) {
	if quiet() {
		return
	}
	fmt.Printf("\033[36m[i]\033[0m "+format+"\r\n", args...)
}

func Warn(format string, args ...any) {
	if quiet() {
		return
	}
	fmt.Fprintf(os.Stderr, "\033[33m[!]\033[0m "+format+"\r\n", args...)
}

func Fatal(format string, args ...any) {
	fmt.Fprintf(os.Stderr, "\r\n\033[31m[x]\033[0m "+format+"\r\n\r\n", args...)
	os.Exit(1)
}

func Section(title string) {
	if quiet() {
		return
	}
	fmt.Printf("\r\n=== %s ===\r\n\r\n", title)
}
