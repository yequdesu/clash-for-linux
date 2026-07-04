package main

import (
	"bufio"
	"fmt"
	"os"
	"os/user"
	"path/filepath"
	"strings"
)

func userHomeDir() string {
	if sudoUser := os.Getenv("SUDO_USER"); sudoUser != "" && runningAsRoot() {
		if u, err := user.Lookup(sudoUser); err == nil && u.HomeDir != "" {
			return u.HomeDir
		}
		if sudoUser != "root" {
			return filepath.Join("/home", sudoUser)
		}
	}
	if home, err := os.UserHomeDir(); err == nil && home != "" {
		return home
	}
	return "."
}

func confirm(prompt string, def bool) bool {
	suffix := " [y/N] "
	if def {
		suffix = " [Y/n] "
	}
	fmt.Fprint(os.Stderr, prompt+suffix)
	reader := bufio.NewReader(os.Stdin)
	answer, _ := reader.ReadString('\n')
	answer = strings.ToLower(strings.TrimSpace(answer))
	if answer == "" {
		return def
	}
	return answer == "y" || answer == "yes"
}

func upsertManagedBlock(path, name, body string) error {
	start := "# " + name + " START"
	end := "# " + name + " END"
	existing := ""
	if data, err := os.ReadFile(path); err == nil {
		existing = string(data)
	}
	cleaned := removeBlock(existing, start, end)
	if strings.TrimSpace(cleaned) != "" && !strings.HasSuffix(cleaned, "\n") {
		cleaned += "\n"
	}
	cleaned += start + "\n" + strings.TrimRight(body, "\n") + "\n" + end + "\n"
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		return err
	}
	return os.WriteFile(path, []byte(cleaned), 0o644)
}

func removeManagedBlock(path, name string) error {
	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			return nil
		}
		return err
	}
	start := "# " + name + " START"
	end := "# " + name + " END"
	next := removeBlock(string(data), start, end)
	if next == string(data) {
		return nil
	}
	return os.WriteFile(path, []byte(next), 0o644)
}

func removeBlock(content, start, end string) string {
	var out []string
	skipping := false
	for _, line := range strings.Split(content, "\n") {
		trimmed := strings.TrimSpace(line)
		if trimmed == start {
			skipping = true
			continue
		}
		if skipping {
			if trimmed == end {
				skipping = false
			}
			continue
		}
		out = append(out, line)
	}
	return strings.TrimRight(strings.Join(out, "\n"), "\n") + "\n"
}

func shellQuote(value string) string {
	return "'" + strings.ReplaceAll(value, "'", "'\\''") + "'"
}

func removeSafePath(path string) error {
	clean := filepath.Clean(path)
	if clean == "." || clean == "/" || clean == filepath.Clean(userHomeDir()) || len(clean) < 5 {
		return fmt.Errorf("refusing to remove unsafe path: %s", path)
	}
	return os.RemoveAll(clean)
}

func uniqueStrings(values []string) []string {
	seen := map[string]bool{}
	var out []string
	for _, value := range values {
		if value == "" || seen[value] {
			continue
		}
		seen[value] = true
		out = append(out, value)
	}
	return out
}
