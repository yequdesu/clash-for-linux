package log

import (
	"fmt"
	"os"
)

func Ok(format string, args ...any) {
	fmt.Printf("\033[32m[+]\033[0m "+format+"\r\n", args...)
}

func Info(format string, args ...any) {
	fmt.Printf("\033[36m[i]\033[0m "+format+"\r\n", args...)
}

func Warn(format string, args ...any) {
	fmt.Fprintf(os.Stderr, "\033[33m[!]\033[0m "+format+"\r\n", args...)
}

func Fatal(format string, args ...any) {
	fmt.Fprintf(os.Stderr, "\r\n\033[31m[x]\033[0m "+format+"\r\n\r\n", args...)
	os.Exit(1)
}

func Section(title string) {
	fmt.Printf("\r\n=== %s ===\r\n\r\n", title)
}
