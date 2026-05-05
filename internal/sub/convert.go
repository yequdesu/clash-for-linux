package sub

import (
	"fmt"
	"net/http"
	"net/url"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"time"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

func ConvertDownload(cfg *config.EnvConfig, subURL, dest string) error {
	if strings.HasPrefix(subURL, "file://") {
		return nil
	}

	port := "25500"
	checkAlive := func() bool {
		resp, err := http.Get(fmt.Sprintf("http://localhost:%s/version", port))
		if err != nil {
			return false
		}
		resp.Body.Close()
		return resp.StatusCode == 200
	}

	subStarted := false
	if !checkAlive() {
		logFile, _ := os.OpenFile(
			filepath.Join(cfg.SubconverterDir(), "latest.log"),
			os.O_CREATE|os.O_WRONLY|os.O_APPEND, 0644,
		)
		cmd := exec.Command(cfg.SubconverterBin())
		cmd.Dir = cfg.SubconverterDir()
		cmd.Stdout = logFile
		cmd.Stderr = logFile
		cmd.Start()
		subStarted = true
		defer func() {
			if subStarted {
				exec.Command("pkill", "-9", "-f", cfg.SubconverterBin()).Run()
			}
		}()

		deadline := time.Now().Add(3 * time.Second)
		for !checkAlive() {
			if time.Now().After(deadline) {
				return fmt.Errorf("subconverter failed to start")
			}
			time.Sleep(300 * time.Millisecond)
		}
	}

	convertURL := fmt.Sprintf("http://127.0.0.1:%s/sub?target=clash&url=%s",
		port, url.QueryEscape(subURL))
	ilog.Info("converting via subconverter...")
	return httpDownload(convertURL, dest, cfg.ClashSubUA)
}
