package main

import (
	"bufio"
	"fmt"
	"os"
	"strings"

	"github.com/spf13/cobra"
)

var (
	logLines   int
	logFollow  bool
	logLevel   string
)

var logCmd = &cobra.Command{
	Use:   "log",
	Short: "View Mihomo kernel logs",
	Long:  "View Mihomo kernel log file with optional filtering and following.",
	Run:   runLog,
}

func init() {
	logCmd.Flags().IntVarP(&logLines, "lines", "n", 50, "Number of lines to show")
	logCmd.Flags().BoolVarP(&logFollow, "follow", "f", false, "Follow log output (tail -f)")
	logCmd.Flags().StringVar(&logLevel, "level", "", "Filter by log level (info, warn, error, debug)")
	rootCmd.AddCommand(logCmd)
}

func runLog(cmd *cobra.Command, args []string) {
	logFile := logFilePath()
	if !fileExists(logFile) {
		fmt.Printf("%s Log file not found: %s\n", yellow("⚠"), logFile)
		return
	}

	if logFollow {
		followLog(logFile)
		return
	}

	// Read last N lines
	lines := readLastLines(logFile, logLines)
	for _, line := range lines {
		if logLevel != "" {
			// Simple level filter
			lowerLine := strings.ToLower(line)
			if !strings.Contains(lowerLine, strings.ToLower(logLevel)) {
				continue
			}
		}
		// Colorize log levels
		printed := line
		printed = colorizeLog(printed)
		fmt.Println(printed)
	}
}

func readLastLines(path string, n int) []string {
	f, err := os.Open(path)
	if err != nil {
		return nil
	}
	defer f.Close()

	var lines []string
	scanner := bufio.NewScanner(f)
	for scanner.Scan() {
		lines = append(lines, scanner.Text())
	}

	if len(lines) > n {
		lines = lines[len(lines)-n:]
	}
	return lines
}

func followLog(path string) {
	f, err := os.Open(path)
	if err != nil {
		fmt.Printf("%s Cannot open log: %v\n", red("✗"), err)
		return
	}
	defer f.Close()

	// Seek to end
	f.Seek(0, 2)
	scanner := bufio.NewScanner(f)
	for scanner.Scan() {
		line := scanner.Text()
		if logLevel != "" {
			lowerLine := strings.ToLower(line)
			if !strings.Contains(lowerLine, strings.ToLower(logLevel)) {
				continue
			}
		}
		fmt.Println(colorizeLog(line))
	}
}

func colorizeLog(line string) string {
	lower := strings.ToLower(line)
	if strings.Contains(lower, "error") || strings.Contains(lower, "fail") {
		return red(line)
	}
	if strings.Contains(lower, "warn") || strings.Contains(lower, "timeout") {
		return yellow(line)
	}
	if strings.Contains(lower, "info") {
		return line
	}
	if strings.Contains(lower, "debug") {
		return gray(line)
	}
	return line
}
