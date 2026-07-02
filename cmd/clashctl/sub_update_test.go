package main

import (
	"errors"
	"net"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
	"github.com/yequdesu/linux-cli-tui-clash/internal/sub"
)

func TestSaveUpdatedProfileDataUpdatesFileAndMetadata(t *testing.T) {
	cfg, profilePath := testUpdateProfileConfig(t)

	commit, err := saveUpdatedProfileData(cfg, 1, []byte("proxies:\n  - name: new\n"))
	if err != nil {
		t.Fatalf("saveUpdatedProfileData: %v", err)
	}
	if commit.currentUse != 1 {
		t.Fatalf("currentUse = %d, want 1", commit.currentUse)
	}
	data, err := os.ReadFile(profilePath)
	if err != nil {
		t.Fatal(err)
	}
	if string(data) != "proxies:\n  - name: new\n" {
		t.Fatalf("profile data = %q", data)
	}
	meta, err := config.LoadProfiles(cfg.ProfilesMeta())
	if err != nil {
		t.Fatal(err)
	}
	if meta.Profiles[0].Updated == "" {
		t.Fatal("Updated is empty")
	}
}

func TestSaveUpdatedProfileDataRestoresProfileWhenMetadataSaveFails(t *testing.T) {
	cfg, profilePath := testUpdateProfileConfig(t)
	originalMeta, err := os.ReadFile(cfg.ProfilesMeta())
	if err != nil {
		t.Fatal(err)
	}
	originalSaveProfiles := saveProfiles
	saveProfiles = func(string, *config.ProfilesMeta) error {
		return errors.New("forced metadata failure")
	}
	defer func() { saveProfiles = originalSaveProfiles }()

	_, err = saveUpdatedProfileData(cfg, 1, []byte("proxies:\n  - name: broken\n"))
	if err == nil {
		t.Fatal("saveUpdatedProfileData succeeded, want error")
	}
	data, err := os.ReadFile(profilePath)
	if err != nil {
		t.Fatal(err)
	}
	if string(data) != "proxies:\n  - name: old\n" {
		t.Fatalf("profile data = %q, want old data", data)
	}
	metaData, err := os.ReadFile(cfg.ProfilesMeta())
	if err != nil {
		t.Fatal(err)
	}
	if string(metaData) != string(originalMeta) {
		t.Fatalf("profiles.yaml changed on failure:\n%s", metaData)
	}
}

func TestSaveUpdatedProfileDataRollbackRestoresFileAndMetadata(t *testing.T) {
	cfg, profilePath := testUpdateProfileConfig(t)
	originalMeta, err := os.ReadFile(cfg.ProfilesMeta())
	if err != nil {
		t.Fatal(err)
	}

	commit, err := saveUpdatedProfileData(cfg, 1, []byte("proxies:\n  - name: new\n"))
	if err != nil {
		t.Fatalf("saveUpdatedProfileData: %v", err)
	}
	commit.rollback()

	data, err := os.ReadFile(profilePath)
	if err != nil {
		t.Fatal(err)
	}
	if string(data) != "proxies:\n  - name: old\n" {
		t.Fatalf("profile data = %q, want old data", data)
	}
	metaData, err := os.ReadFile(cfg.ProfilesMeta())
	if err != nil {
		t.Fatal(err)
	}
	if string(metaData) != string(originalMeta) {
		t.Fatalf("profiles.yaml = %q, want original %q", metaData, originalMeta)
	}
}

func TestActiveUpdateRollbackRestoresProfileWhenSwitchFails(t *testing.T) {
	cfg, profilePath := testUpdateProfileConfig(t)
	if err := os.MkdirAll(cfg.ResourcesDir(), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(cfg.ConfigPath(), []byte("proxies:\n  - name: old-config\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(cfg.RuntimePath(), []byte("mixed-port: 7890\n"), 0o644); err != nil {
		t.Fatal(err)
	}

	commit, err := saveUpdatedProfileData(cfg, 1, []byte("proxies:\n  - name: invalid-new\n"))
	if err != nil {
		t.Fatalf("saveUpdatedProfileData: %v", err)
	}
	if commit.currentUse != 1 {
		t.Fatalf("currentUse = %d, want 1", commit.currentUse)
	}
	err = switchSubscription(cfg, 1)
	if err == nil || !strings.Contains(err.Error(), "restored previous config") {
		t.Fatalf("switchSubscription error = %v, want restored previous config", err)
	}
	commit.rollback()

	data, err := os.ReadFile(profilePath)
	if err != nil {
		t.Fatal(err)
	}
	if string(data) != "proxies:\n  - name: old\n" {
		t.Fatalf("profile data = %q, want old data", data)
	}
}

func TestUpdateSubscriptionReturnsErrorWhenNoSubscriptions(t *testing.T) {
	base := t.TempDir()
	oldCfg := cfg
	cfg = &config.EnvConfig{ClashBaseDir: base, KernelName: "missing-kernel"}
	defer func() { cfg = oldCfg }()

	err := updateSubscription(nil)
	if err == nil || !strings.Contains(err.Error(), "no subscriptions") {
		t.Fatalf("updateSubscription error = %v, want no subscriptions", err)
	}
}

func TestUpdateSubscriptionReturnsErrorForInvalidID(t *testing.T) {
	err := updateSubscription([]string{"not-a-number"})
	if err == nil || !strings.Contains(err.Error(), "invalid subscription id") {
		t.Fatalf("updateSubscription error = %v, want invalid subscription id", err)
	}
}

func TestSubscriptionDownloadOptionsHonorsUpdateProxyModes(t *testing.T) {
	cfg, closeCore := testConfigWithRuntimeProxy(t, true)
	defer closeCore()

	opts, label, err := subscriptionDownloadOptions(cfg, config.Profile{UpdateProxy: "direct"})
	if err != nil {
		t.Fatalf("direct options: %v", err)
	}
	if opts.ProxyMode != sub.ProxyModeDirect || label != "direct" {
		t.Fatalf("direct opts=%+v label=%q", opts, label)
	}

	opts, label, err = subscriptionDownloadOptions(cfg, config.Profile{UpdateProxy: "system"})
	if err != nil {
		t.Fatalf("system options: %v", err)
	}
	if opts.ProxyMode != sub.ProxyModeSystem || label != "system proxy env" {
		t.Fatalf("system opts=%+v label=%q", opts, label)
	}

	opts, label, err = subscriptionDownloadOptions(cfg, config.Profile{UpdateProxy: "core"})
	if err != nil {
		t.Fatalf("core options: %v", err)
	}
	if opts.ProxyMode != sub.ProxyModeCore || !strings.HasPrefix(opts.ProxyURL, "http://127.0.0.1:") || !strings.HasPrefix(label, "core ") {
		t.Fatalf("core opts=%+v label=%q", opts, label)
	}

	opts, _, err = subscriptionDownloadOptions(cfg, config.Profile{UpdateProxy: "auto"})
	if err != nil {
		t.Fatalf("auto core options: %v", err)
	}
	if opts.ProxyMode != sub.ProxyModeCore {
		t.Fatalf("auto opts=%+v, want core when port is listening", opts)
	}
}

func TestSubscriptionDownloadOptionsAutoFallbackAndCoreFailure(t *testing.T) {
	cfg, closeCore := testConfigWithRuntimeProxy(t, false)
	defer closeCore()

	opts, label, err := subscriptionDownloadOptions(cfg, config.Profile{UpdateProxy: "auto"})
	if err != nil {
		t.Fatalf("auto fallback options: %v", err)
	}
	if opts.ProxyMode != sub.ProxyModeSystem || !strings.Contains(label, "core port not listening") {
		t.Fatalf("auto fallback opts=%+v label=%q", opts, label)
	}

	_, _, err = subscriptionDownloadOptions(cfg, config.Profile{UpdateProxy: "core"})
	if err == nil || !strings.Contains(err.Error(), "requires kernel proxy port") {
		t.Fatalf("core unavailable error = %v", err)
	}
}

func TestHandleSubUpdateErrorExitsForCron(t *testing.T) {
	oldCron := subUpdateCron
	oldExit := exitProcess
	defer func() {
		subUpdateCron = oldCron
		exitProcess = oldExit
	}()

	subUpdateCron = true
	exitCode := -1
	exitProcess = func(code int) {
		exitCode = code
	}

	handleSubUpdateError(errors.New("forced update failure"))
	if exitCode != 1 {
		t.Fatalf("exit code = %d, want 1", exitCode)
	}
}

func TestHandleSubUpdateErrorIsSilentForCron(t *testing.T) {
	oldCron := subUpdateCron
	oldExit := exitProcess
	defer func() {
		subUpdateCron = oldCron
		exitProcess = oldExit
	}()

	subUpdateCron = true
	exitProcess = func(int) {}
	stdout, stderr := captureStdoutStderr(t, func() {
		handleSubUpdateError(errors.New("forced update failure"))
	})
	if stdout != "" || stderr != "" {
		t.Fatalf("cron error output stdout=%q stderr=%q, want silent", stdout, stderr)
	}
}

func TestHandleSubUpdateErrorExitsForInteractiveMode(t *testing.T) {
	oldCron := subUpdateCron
	oldExit := exitProcess
	defer func() {
		subUpdateCron = oldCron
		exitProcess = oldExit
	}()

	subUpdateCron = false
	exitCode := -1
	exitProcess = func(code int) {
		exitCode = code
	}

	handleSubUpdateError(errors.New("forced update failure"))
	if exitCode != 1 {
		t.Fatalf("exit code = %d, want 1", exitCode)
	}
}

func TestSubscriptionQuietSuppressesTerminalOutput(t *testing.T) {
	stdout, stderr := captureStdoutStderr(t, func() {
		if err := withSubscriptionQuiet(true, func() error {
			subscriptionInfo("info")
			subscriptionOk("ok")
			subscriptionWarn("warn")
			return nil
		}); err != nil {
			t.Fatal(err)
		}
	})
	if stdout != "" || stderr != "" {
		t.Fatalf("quiet output stdout=%q stderr=%q, want silent", stdout, stderr)
	}
}

func TestSubscriptionQuietRestoresPreviousState(t *testing.T) {
	oldQuiet := subscriptionQuiet
	defer func() { subscriptionQuiet = oldQuiet }()
	subscriptionQuiet = false
	if err := withSubscriptionQuiet(true, func() error { return nil }); err != nil {
		t.Fatal(err)
	}
	if subscriptionQuiet {
		t.Fatal("subscriptionQuiet was not restored")
	}
}

func TestMergeAutoUpdateCrontabAddsCronLine(t *testing.T) {
	got, changed := mergeAutoUpdateCrontab("MAILTO=ops@example.test\n", "*/10 * * * * /usr/local/bin/clashctl sub update --scheduled --cron")
	if !changed {
		t.Fatal("mergeAutoUpdateCrontab changed = false, want true")
	}
	if !strings.Contains(got, "MAILTO=ops@example.test\n") {
		t.Fatalf("existing crontab entry was not preserved:\n%s", got)
	}
	if !strings.Contains(got, "/usr/local/bin/clashctl sub update --scheduled --cron\n") {
		t.Fatalf("cron update line missing:\n%s", got)
	}
}

func TestMergeAutoUpdateCrontabReplacesStaleLine(t *testing.T) {
	got, changed := mergeAutoUpdateCrontab("0 */12 * * * clashctl sub update\n", "*/10 * * * * /usr/local/bin/clashctl sub update --scheduled --cron")
	if !changed {
		t.Fatal("mergeAutoUpdateCrontab changed = false, want true")
	}
	if strings.Contains(got, "clashctl sub update\n") {
		t.Fatalf("stale cron line remained:\n%s", got)
	}
	if !strings.Contains(got, "/usr/local/bin/clashctl sub update --scheduled --cron\n") {
		t.Fatalf("replacement cron line missing:\n%s", got)
	}
}

func TestMergeAutoUpdateCrontabIgnoresCommentedLine(t *testing.T) {
	got, changed := mergeAutoUpdateCrontab("# 0 */12 * * * clashctl sub update --cron\n", "*/10 * * * * /usr/local/bin/clashctl sub update --scheduled --cron")
	if !changed {
		t.Fatal("mergeAutoUpdateCrontab changed = false, want true")
	}
	if !strings.Contains(got, "# 0 */12 * * * clashctl sub update --cron\n") {
		t.Fatalf("commented cron line was not preserved:\n%s", got)
	}
	if !strings.Contains(got, "\n*/10 * * * * /usr/local/bin/clashctl sub update --scheduled --cron\n") {
		t.Fatalf("active cron line missing:\n%s", got)
	}
}

func TestMergeAutoUpdateCrontabKeepsExistingEnabledLine(t *testing.T) {
	existing := "*/10 * * * * clashctl sub update --scheduled --cron\n"
	got, changed := mergeAutoUpdateCrontab(existing, "*/10 * * * * /usr/local/bin/clashctl sub update --scheduled --cron")
	if changed {
		t.Fatal("mergeAutoUpdateCrontab changed = true, want false")
	}
	if got != existing {
		t.Fatalf("crontab = %q, want %q", got, existing)
	}
}

func testUpdateProfileConfig(t *testing.T) (*config.EnvConfig, string) {
	t.Helper()
	base := t.TempDir()
	cfg := &config.EnvConfig{
		ClashBaseDir: base,
		KernelName:   "missing-kernel",
		ClashSubUA:   "test",
	}
	if err := os.MkdirAll(cfg.ProfilesDir(), 0o755); err != nil {
		t.Fatal(err)
	}
	profilePath := filepath.Join(cfg.ProfilesDir(), "1.yaml")
	if err := os.WriteFile(profilePath, []byte("proxies:\n  - name: old\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	meta := &config.ProfilesMeta{
		Use: 1,
		Profiles: []config.Profile{
			{ID: 1, Path: profilePath, URL: "file://profile.yaml", Updated: "old-time"},
		},
	}
	if err := config.SaveProfiles(cfg.ProfilesMeta(), meta); err != nil {
		t.Fatal(err)
	}
	return cfg, profilePath
}

func testConfigWithRuntimeProxy(t *testing.T, keepListening bool) (*config.EnvConfig, func()) {
	t.Helper()
	base := t.TempDir()
	cfg := &config.EnvConfig{ClashBaseDir: base}
	if err := os.MkdirAll(cfg.ResourcesDir(), 0o755); err != nil {
		t.Fatal(err)
	}
	ln, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	_, port, err := net.SplitHostPort(ln.Addr().String())
	if err != nil {
		ln.Close()
		t.Fatal(err)
	}
	closeCore := func() { _ = ln.Close() }
	if !keepListening {
		closeCore()
		closeCore = func() {}
	}
	if err := os.WriteFile(cfg.RuntimePath(), []byte("mixed-port: "+port+"\n"), 0o644); err != nil {
		closeCore()
		t.Fatal(err)
	}
	return cfg, closeCore
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
