package main

import (
	"encoding/json"
	"fmt"
	"net/netip"
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"strings"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

const (
	sshTunBypassPriority = "8000"
	sshTunBypassTable    = "2021"
)

type sshSessionInfo struct {
	Client     string
	ClientPort string
	Server     string
	ServerPort string
	Active     bool
}

type routeInfo struct {
	Raw string `json:"raw"`
	Via string `json:"via,omitempty"`
	Dev string `json:"dev,omitempty"`
	Src string `json:"src,omitempty"`
}

type sshTunBypassEntry struct {
	Family   string    `json:"family"`
	Prefix   string    `json:"prefix"`
	Table    string    `json:"table"`
	Priority string    `json:"priority"`
	Route    routeInfo `json:"route"`
}

type sshTunBypassState struct {
	Entries []sshTunBypassEntry `json:"entries"`
}

var (
	runSSHGuardIPCommand = func(args ...string) ([]byte, error) {
		return exec.Command("ip", args...).CombinedOutput()
	}
	sshGuardGetuid   = os.Getuid
	sshGuardGetppid  = os.Getppid
	sshGuardReadFile = os.ReadFile
)

func allowSSHTunRisk(flag bool) bool {
	if flag {
		return true
	}
	switch strings.ToLower(strings.TrimSpace(os.Getenv("CLASH_ALLOW_SSH_TUN_RISK"))) {
	case "1", "true", "yes", "on":
		return true
	default:
		return false
	}
}

func ensureSafeKernelStartFromSSH(cfg *config.EnvConfig, allowFlag bool, forceTunEnable bool) error {
	if allowSSHTunRisk(allowFlag) {
		return nil
	}

	session := currentSSHSession()
	if !session.Active {
		return nil
	}

	info, err := config.ReadRuntimeInfo(cfg.RuntimePath())
	if err != nil && !forceTunEnable {
		return nil
	}
	if !tunStartCanCaptureRoutes(info, forceTunEnable) {
		return nil
	}

	if strings.TrimSpace(session.Client) == "" {
		return fmt.Errorf("SSH session detected, but SSH_CONNECTION does not expose the client IP; run from a local console or pass --allow-ssh-tun-risk after confirming a recovery path")
	}

	entry, err := buildSSHTunBypassEntry(session.Client)
	if err != nil {
		return err
	}

	if sshGuardGetuid() == 0 {
		if err := installSSHTunBypass(cfg, entry); err != nil {
			return err
		}
		ilog.Info("protected SSH client route before TUN: %s via table %s priority %s", entry.Prefix, entry.Table, entry.Priority)
		return nil
	}

	if routeIsDirectMainLink(entry.Route) {
		ilog.Info("SSH client route is direct on %s; TUN auto-route can keep this session on the main table", entry.Route.Dev)
		return nil
	}

	return fmt.Errorf("SSH client %s is reached through gateway route %q; run this command with sudo so clashctl can install an SSH bypass route before enabling TUN, or pass --allow-ssh-tun-risk after confirming another recovery path", session.Client, entry.Route.Raw)
}

func currentSSHSession() sshSessionInfo {
	if session := sshSessionFromEnv(os.Getenv("SSH_CONNECTION"), os.Getenv("SSH_TTY")); session.Active {
		return session
	}
	if session := sshSessionFromProcessTree(); session.Active {
		return session
	}
	return sshSessionInfo{}
}

func sshSessionFromEnv(sshConnection, sshTTY string) sshSessionInfo {
	fields := strings.Fields(sshConnection)
	if len(fields) >= 4 {
		return sshSessionInfo{
			Client:     fields[0],
			ClientPort: fields[1],
			Server:     fields[2],
			ServerPort: fields[3],
			Active:     true,
		}
	}
	if len(fields) > 0 {
		return sshSessionInfo{Client: fields[0], Active: true}
	}
	if strings.TrimSpace(sshTTY) != "" {
		return sshSessionInfo{Active: true}
	}
	return sshSessionInfo{}
}

func sshSessionFromProcessTree() sshSessionInfo {
	pid := sshGuardGetppid()
	seen := map[int]bool{}
	for depth := 0; pid > 1 && depth < 32 && !seen[pid]; depth++ {
		seen[pid] = true
		if env, err := readProcessEnv(pid); err == nil {
			if session := sshSessionFromEnv(env["SSH_CONNECTION"], env["SSH_TTY"]); session.Active {
				return session
			}
		}
		parent, err := readParentPID(pid)
		if err != nil || parent <= 0 || parent == pid {
			break
		}
		pid = parent
	}
	return sshSessionInfo{}
}

func readProcessEnv(pid int) (map[string]string, error) {
	data, err := sshGuardReadFile(fmt.Sprintf("/proc/%d/environ", pid))
	if err != nil {
		return nil, err
	}
	env := map[string]string{}
	for _, entry := range strings.Split(string(data), "\x00") {
		if entry == "" {
			continue
		}
		key, value, ok := strings.Cut(entry, "=")
		if ok {
			env[key] = value
		}
	}
	return env, nil
}

func readParentPID(pid int) (int, error) {
	data, err := sshGuardReadFile(fmt.Sprintf("/proc/%d/status", pid))
	if err != nil {
		return 0, err
	}
	for _, line := range strings.Split(string(data), "\n") {
		line = strings.TrimSpace(line)
		if !strings.HasPrefix(line, "PPid:") {
			continue
		}
		return strconv.Atoi(strings.TrimSpace(strings.TrimPrefix(line, "PPid:")))
	}
	return 0, fmt.Errorf("PPid not found for pid %d", pid)
}

func sshSessionClient() (string, bool) {
	session := currentSSHSession()
	return session.Client, session.Active
}

func tunStartCanCaptureRoutes(info config.RuntimeInfo, forceTunEnable bool) bool {
	if forceTunEnable {
		return true
	}
	return info.TunEnabled && (info.TunAutoRoute || info.TunStrictRoute)
}

func buildSSHTunBypassEntry(client string) (sshTunBypassEntry, error) {
	addr, err := netip.ParseAddr(strings.TrimSpace(client))
	if err != nil {
		return sshTunBypassEntry{}, fmt.Errorf("invalid SSH client IP %q: %w", client, err)
	}
	route, err := lookupMainRoute(addr)
	if err != nil {
		return sshTunBypassEntry{}, err
	}
	if route.Dev == "" {
		return sshTunBypassEntry{}, fmt.Errorf("cannot determine route device for SSH client %s: %s", addr, route.Raw)
	}
	if isLikelyTunnelDevice(route.Dev) {
		return sshTunBypassEntry{}, fmt.Errorf("main route for SSH client %s already uses tunnel device %s: %s", addr, route.Dev, route.Raw)
	}

	return sshTunBypassEntry{
		Family:   ipFamily(addr),
		Prefix:   hostPrefix(addr),
		Table:    sshTunBypassTable,
		Priority: sshTunBypassPriority,
		Route:    route,
	}, nil
}

func lookupMainRoute(addr netip.Addr) (routeInfo, error) {
	out, err := runSSHGuardIPCommand(append(ipFamilyArgs(addr), "route", "get", addr.String())...)
	if err == nil {
		route := parseRouteGetOutput(out)
		if route.Raw != "" && !isLikelyTunnelDevice(route.Dev) {
			return route, nil
		}
	}

	mainOut, mainErr := runSSHGuardIPCommand(append(ipFamilyArgs(addr), "route", "show", "table", "main", "match", addr.String())...)
	if mainErr == nil {
		if route, parseErr := parseMainTableRoute(mainOut, addr); parseErr == nil {
			return route, nil
		}
	}

	if err != nil {
		return routeInfo{}, fmt.Errorf("cannot inspect route to SSH client %s: %s", addr, strings.TrimSpace(string(out)))
	}
	route := parseRouteGetOutput(out)
	if route.Raw == "" {
		return routeInfo{}, fmt.Errorf("empty route result for SSH client %s", addr)
	}
	return route, nil
}

func parseRouteGetOutput(out []byte) routeInfo {
	raw := strings.TrimSpace(string(out))
	return parseRouteLine(raw)
}

func parseRouteLine(raw string) routeInfo {
	fields := strings.Fields(raw)
	route := routeInfo{Raw: raw}
	for i := 0; i < len(fields)-1; i++ {
		switch fields[i] {
		case "via":
			route.Via = fields[i+1]
		case "dev":
			route.Dev = fields[i+1]
		case "src":
			route.Src = fields[i+1]
		}
	}
	return route
}

func parseMainTableRoute(out []byte, addr netip.Addr) (routeInfo, error) {
	bestBits := -1
	var best routeInfo
	for _, line := range strings.Split(strings.TrimSpace(string(out)), "\n") {
		line = strings.TrimSpace(line)
		if line == "" {
			continue
		}
		fields := strings.Fields(line)
		if len(fields) == 0 {
			continue
		}
		bits, ok := routeDestinationBits(fields[0], addr)
		if !ok || bits < bestBits {
			continue
		}
		route := parseRouteLine(line)
		if route.Dev == "" {
			continue
		}
		bestBits = bits
		best = route
	}
	if bestBits < 0 {
		return routeInfo{}, fmt.Errorf("no main table route matches %s", addr)
	}
	return best, nil
}

func routeDestinationBits(dest string, addr netip.Addr) (int, bool) {
	if dest == "default" {
		return 0, true
	}
	if strings.Contains(dest, "/") {
		prefix, err := netip.ParsePrefix(dest)
		if err != nil || prefix.Addr().Is4() != addr.Is4() || !prefix.Contains(addr) {
			return 0, false
		}
		return prefix.Bits(), true
	}
	routeAddr, err := netip.ParseAddr(dest)
	if err != nil || routeAddr.Is4() != addr.Is4() || routeAddr != addr {
		return 0, false
	}
	if addr.Is6() {
		return 128, true
	}
	return 32, true
}

func routeIsDirectMainLink(route routeInfo) bool {
	return route.Dev != "" && route.Via == "" && !isLikelyTunnelDevice(route.Dev)
}

func installSSHTunBypass(cfg *config.EnvConfig, entry sshTunBypassEntry) error {
	if err := os.MkdirAll(filepath.Dir(sshTunBypassStatePath(cfg)), 0755); err != nil {
		return fmt.Errorf("prepare ssh bypass state dir: %w", err)
	}

	_ = deleteSSHTunBypassEntry(entry)

	routeArgs := append(ipFamilyArgsFor(entry.Family), "route", "replace", entry.Prefix, "table", entry.Table)
	if entry.Route.Via != "" {
		routeArgs = append(routeArgs, "via", entry.Route.Via)
	}
	routeArgs = append(routeArgs, "dev", entry.Route.Dev)
	if entry.Route.Src != "" {
		routeArgs = append(routeArgs, "src", entry.Route.Src)
	}
	if out, err := runSSHGuardIPCommand(routeArgs...); err != nil {
		return fmt.Errorf("install SSH bypass route failed: ip %s: %s", strings.Join(routeArgs, " "), strings.TrimSpace(string(out)))
	}

	ruleArgs := append(ipFamilyArgsFor(entry.Family), "rule", "add", "priority", entry.Priority, "to", entry.Prefix, "lookup", entry.Table)
	if out, err := runSSHGuardIPCommand(ruleArgs...); err != nil {
		return fmt.Errorf("install SSH bypass rule failed: ip %s: %s", strings.Join(ruleArgs, " "), strings.TrimSpace(string(out)))
	}

	state := sshTunBypassState{Entries: []sshTunBypassEntry{entry}}
	data, err := json.MarshalIndent(state, "", "  ")
	if err != nil {
		return err
	}
	return config.AtomicWriteFile(sshTunBypassStatePath(cfg), append(data, '\n'), 0644)
}

func cleanupSSHTunBypass(cfg *config.EnvConfig) error {
	path := sshTunBypassStatePath(cfg)
	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			return nil
		}
		return err
	}
	var state sshTunBypassState
	if err := json.Unmarshal(data, &state); err != nil {
		_ = os.Remove(path)
		return fmt.Errorf("parse ssh bypass state: %w", err)
	}
	for _, entry := range state.Entries {
		_ = deleteSSHTunBypassEntry(entry)
	}
	_ = os.Remove(path)
	return nil
}

func deleteSSHTunBypassEntry(entry sshTunBypassEntry) error {
	ruleArgs := append(ipFamilyArgsFor(entry.Family), "rule", "del", "priority", entry.Priority, "to", entry.Prefix, "lookup", entry.Table)
	_, ruleErr := runSSHGuardIPCommand(ruleArgs...)
	routeArgs := append(ipFamilyArgsFor(entry.Family), "route", "del", entry.Prefix, "table", entry.Table)
	_, routeErr := runSSHGuardIPCommand(routeArgs...)
	if ruleErr != nil {
		return ruleErr
	}
	return routeErr
}

func sshTunBypassStatePath(cfg *config.EnvConfig) string {
	return filepath.Join(filepath.Dir(cfg.PidFile()), "ssh-tun-bypass.json")
}

func ipFamily(addr netip.Addr) string {
	if addr.Is6() {
		return "ipv6"
	}
	return "ipv4"
}

func ipFamilyArgs(addr netip.Addr) []string {
	return ipFamilyArgsFor(ipFamily(addr))
}

func ipFamilyArgsFor(family string) []string {
	if family == "ipv6" {
		return []string{"-6"}
	}
	return []string{"-4"}
}

func hostPrefix(addr netip.Addr) string {
	if addr.Is6() {
		return addr.String() + "/128"
	}
	return addr.String() + "/32"
}

func isLikelyTunnelDevice(name string) bool {
	lower := strings.ToLower(name)
	return strings.Contains(lower, "tun") || strings.Contains(lower, "tap") || strings.Contains(lower, "mihomo") || strings.Contains(lower, "clash")
}
