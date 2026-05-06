package log

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"time"
)

const (
	LevelDebug = "DEBUG"
	LevelInfo  = "INFO"
	LevelWarn  = "WARN"
	LevelError = "ERROR"
)

var (
	colorReset  = "\033[0m"
	colorGray   = "\033[90m"
	colorGreen  = "\033[32m"
	colorYellow = "\033[33m"
	colorRed    = "\033[31m"
	colorCyan   = "\033[36m"
)

// LogEntry represents a single log line
type LogEntry struct {
	Timestamp time.Time
	Level     string
	Message   string
	Raw       string
}

// WriteClashLog writes a log entry to the clashctl log file
func WriteClashLog(logsDir, level, message string) {
	os.MkdirAll(logsDir, 0755)
	logPath := filepath.Join(logsDir, "clashctl.log")

	now := time.Now().Format("2006-01-02 15:04:05")
	entry := fmt.Sprintf("[%s] [%s] %s\n", now, level, message)

	f, err := os.OpenFile(logPath, os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0644)
	if err != nil {
		return
	}
	defer f.Close()
	f.WriteString(entry)
}

// WriteProfileLog writes a subscription operation log
func WriteProfileLog(resourcesDir, format string, args ...interface{}) {
	logPath := filepath.Join(resourcesDir, "profiles.log")
	msg := fmt.Sprintf(format, args...)
	timestamp := time.Now().Format("2006-01-02 15:04:05")
	entry := fmt.Sprintf("[%s] %s\n", timestamp, msg)

	f, err := os.OpenFile(logPath, os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0644)
	if err != nil {
		return
	}
	defer f.Close()
	f.WriteString(entry)
}

// ColorizeMihomoLog adds color to a Mihomo log line based on level
func ColorizeMihomoLog(line string) string {
	if NoColor() {
		return line
	}

	lower := strings.ToLower(line)
	switch {
	case strings.Contains(lower, "error") || strings.Contains(lower, "fail"):
		return colorRed + line + colorReset
	case strings.Contains(lower, "warn") || strings.Contains(lower, "timeout"):
		return colorYellow + line + colorReset
	case strings.Contains(lower, "debug"):
		return colorGray + line + colorReset
	case strings.Contains(lower, "info"):
		return colorGreen + line + colorReset
	default:
		return line
	}
}

// NoColor checks if NO_COLOR env is set
func NoColor() bool {
	return os.Getenv("NO_COLOR") != ""
}

// ParseMihomoLogLine parses a log line into a structured entry
func ParseMihomoLogLine(line string) LogEntry {
	entry := LogEntry{Raw: line}

	// Try to parse timestamp and level
	if len(line) > 20 {
		tsStr := strings.TrimSpace(line[:19])
		if t, err := time.Parse("2006-01-02T15:04:05", tsStr); err == nil {
			entry.Timestamp = t
			rest := line[19:]

			upper := strings.ToUpper(rest)
			for _, level := range []string{LevelDebug, LevelInfo, LevelWarn, LevelError} {
				if strings.Contains(upper, level) {
					entry.Level = level
					if idx := strings.Index(upper, level); idx >= 0 {
						entry.Message = rest[idx+len(level)+1:]
					}
					break
				}
			}
		}
	}

	// Fallback: extract level from anywhere
	if entry.Level == "" {
		upper := strings.ToUpper(line)
		for _, level := range []string{LevelDebug, LevelInfo, LevelWarn, LevelError} {
			if strings.Contains(upper, level) {
				entry.Level = level
				break
			}
		}
	}

	return entry
}
