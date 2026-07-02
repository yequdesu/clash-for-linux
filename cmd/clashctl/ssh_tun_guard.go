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
	Source   string    `json:"source,omitempty"`
	Reason   string    `json:"reason,omitempty"`
}

type sshTunBypassState struct {
	Entries []sshTunBypassEntry `json:"entries"`
}

var (
	runSSHGuardIPCommand = func(args ...string) ([]byte, error) {
		return exec.Command("ip", args...).CombinedOutput()
	}
	sshGuardGetuid              = os.Getuid
	sshGuardGetppid             = os.Getppid
	sshGuardReadFile            = os.ReadFile
	writeSSHTunBypassStateFile  = config.AtomicWriteFile
	detectConnectedRouteEntries = detectConnectedRouteProtectionEntries
	detectWireGuardRouteEntries = detectWireGuardProtectionEntries
)

func allowSSHTunRisk(flag bool) bool {
	if flag {
		return true
	}
	switch strings.ToLower(strings.TrimSpace(os.Getenv("CLASH_ALLOW_ROUTE_RISK"))) {
	case "1", "true", "yes", "on":
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

	info, err := config.ReadRuntimeInfo(cfg.RuntimePath())
	if err != nil && !forceTunEnable {
		return nil
	}
	if !tunStartCanCaptureRoutes(info, forceTunEnable) {
		return nil
	}

	entries, risks := buildTunRouteProtectionPlan()
	if len(entries) == 0 && len(risks) == 0 {
		return nil
	}

	if sshGuardGetuid() == 0 {
		if len(entries) > 0 {
			if err := installSSHTunBypass(cfg, entries...); err != nil {
				return err
			}
			for _, entry := range entries {
				ilog.Info("protected route before TUN: %s (%s) via table %s priority %s", entry.Prefix, entry.Source, entry.Table, entry.Priority)
			}
		}
		if len(risks) > 0 {
			return fmt.Errorf("%s", strings.Join(risks, "; "))
		}
		return nil
	}

	if len(risks) > 0 || protectionRequiresRoot(entries) {
		return fmt.Errorf("%s", routeProtectionGuidance(entries, risks))
	}

	for _, entry := range entries {
		ilog.Info("%s route is direct on %s; TUN auto-route can keep it on the main table", entry.Source, entry.Route.Dev)
	}
	return nil
}

func buildTunRouteProtectionPlan() ([]sshTunBypassEntry, []string) {
	var entries []sshTunBypassEntry
	var risks []string

	session := currentSSHSession()
	if session.Active {
		sshEntries, sshRisks := sshRouteProtectionEntries(session)
		entries = append(entries, sshEntries...)
		risks = append(risks, sshRisks...)
	}

	manualEntries, manualRisks := manualRouteProtectionEntries()
	entries = append(entries, manualEntries...)
	risks = append(risks, manualRisks...)

	detectedEntries, detectedRisks := detectConnectedRouteEntries()
	entries = append(entries, detectedEntries...)
	risks = append(risks, detectedRisks...)

	wgEntries, wgRisks := detectWireGuardRouteEntries()
	entries = append(entries, wgEntries...)
	risks = append(risks, wgRisks...)

	return dedupeRouteProtectionEntries(entries), risks
}

func sshRouteProtectionEntries(session sshSessionInfo) ([]sshTunBypassEntry, []string) {
	if strings.TrimSpace(session.Client) == "" {
		return nil, []string{"SSH session detected, but SSH_CONNECTION does not expose the client IP"}
	}

	entry, err := buildRouteProtectionEntry("ssh-client", session.Client, "protect current SSH client")
	if err != nil {
		return nil, []string{err.Error()}
	}
	return []sshTunBypassEntry{entry}, nil
}

func manualRouteProtectionEntries() ([]sshTunBypassEntry, []string) {
	raw := strings.TrimSpace(os.Getenv("CLASH_TUN_PROTECT_ROUTES"))
	if raw == "" {
		return nil, nil
	}
	var entries []sshTunBypassEntry
	var risks []string
	for _, token := range strings.FieldsFunc(raw, func(r rune) bool {
		return r == ',' || r == ';' || r == '\n' || r == '\t' || r == ' '
	}) {
		token = strings.TrimSpace(token)
		if token == "" {
			continue
		}
		entry, err := buildRouteProtectionEntry("manual", token, "user configured protected route")
		if err != nil {
			risks = append(risks, err.Error())
			continue
		}
		entries = append(entries, entry)
	}
	return entries, risks
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

func tunStartCanCaptureRoutes(info config.RuntimeInfo, forceTunEnable bool) bool {
	if forceTunEnable {
		return true
	}
	return info.TunEnabled && (info.TunAutoRoute || info.TunStrictRoute)
}

func buildSSHTunBypassEntry(client string) (sshTunBypassEntry, error) {
	return buildRouteProtectionEntry("ssh-client", client, "protect current SSH client")
}

func buildRouteProtectionEntry(source, prefixOrAddr, reason string) (sshTunBypassEntry, error) {
	prefix, err := parseRouteProtectionPrefix(prefixOrAddr)
	if err != nil {
		return sshTunBypassEntry{}, fmt.Errorf("invalid %s protected route %q: %w", source, prefixOrAddr, err)
	}
	if isDefaultRoutePrefix(prefix) {
		return sshTunBypassEntry{}, fmt.Errorf("invalid %s protected route %q: default routes cannot be protected because that would bypass the whole TUN", source, prefixOrAddr)
	}
	addr := prefix.Addr()
	route, err := lookupMainRoute(addr)
	if err != nil {
		return sshTunBypassEntry{}, err
	}
	if route.Dev == "" {
		return sshTunBypassEntry{}, fmt.Errorf("cannot determine route device for %s %s: %s", source, prefix, route.Raw)
	}
	if isClashTunDevice(route.Dev) {
		return sshTunBypassEntry{}, fmt.Errorf("main route for %s %s already uses Clash/Mihomo tunnel device %s: %s", source, prefix, route.Dev, route.Raw)
	}

	return sshTunBypassEntry{
		Family:   ipFamily(addr),
		Prefix:   prefix.String(),
		Table:    sshTunBypassTable,
		Priority: sshTunBypassPriority,
		Route:    route,
		Source:   source,
		Reason:   reason,
	}, nil
}

func parseRouteProtectionPrefix(value string) (netip.Prefix, error) {
	value = strings.TrimSpace(value)
	if strings.Contains(value, "/") {
		prefix, err := netip.ParsePrefix(value)
		if err != nil {
			return netip.Prefix{}, err
		}
		return prefix.Masked(), nil
	}
	addr, err := netip.ParseAddr(value)
	if err != nil {
		return netip.Prefix{}, err
	}
	if addr.Is6() {
		return netip.PrefixFrom(addr, 128), nil
	}
	return netip.PrefixFrom(addr, 32), nil
}

func lookupMainRoute(addr netip.Addr) (routeInfo, error) {
	out, err := runSSHGuardIPCommand(append(ipFamilyArgs(addr), "route", "get", addr.String())...)
	if err == nil {
		route := parseRouteGetOutput(out)
		if route.Raw != "" && !isClashTunDevice(route.Dev) {
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
	return route.Dev != "" && route.Via == "" && !isClashTunDevice(route.Dev)
}

func installSSHTunBypass(cfg *config.EnvConfig, entries ...sshTunBypassEntry) error {
	if err := os.MkdirAll(filepath.Dir(sshTunBypassStatePath(cfg)), 0755); err != nil {
		return fmt.Errorf("prepare route guard state dir: %w", err)
	}

	entries = dedupeRouteProtectionEntries(entries)
	var installed []sshTunBypassEntry
	rollback := func() {
		for _, entry := range installed {
			_ = deleteSSHTunBypassEntry(entry)
		}
	}

	for _, entry := range entries {
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
			rollback()
			return fmt.Errorf("install route protection failed: ip %s: %s", strings.Join(routeArgs, " "), strings.TrimSpace(string(out)))
		}

		ruleArgs := append(ipFamilyArgsFor(entry.Family), "rule", "add", "priority", entry.Priority, "to", entry.Prefix, "lookup", entry.Table)
		if out, err := runSSHGuardIPCommand(ruleArgs...); err != nil {
			rollback()
			_ = deleteSSHTunBypassEntry(entry)
			return fmt.Errorf("install route protection rule failed: ip %s: %s", strings.Join(ruleArgs, " "), strings.TrimSpace(string(out)))
		}
		installed = append(installed, entry)
	}

	state := sshTunBypassState{Entries: entries}
	data, err := json.MarshalIndent(state, "", "  ")
	if err != nil {
		rollback()
		return err
	}
	if err := writeSSHTunBypassStateFile(sshTunBypassStatePath(cfg), append(data, '\n'), 0644); err != nil {
		rollback()
		return fmt.Errorf("write route guard state: %w", err)
	}
	return nil
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
		return fmt.Errorf("parse route guard state: %w", err)
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

func dedupeRouteProtectionEntries(entries []sshTunBypassEntry) []sshTunBypassEntry {
	seen := map[string]bool{}
	var out []sshTunBypassEntry
	for _, entry := range entries {
		key := strings.Join([]string{entry.Family, entry.Prefix, entry.Table, entry.Priority}, "|")
		if seen[key] {
			continue
		}
		seen[key] = true
		out = append(out, entry)
	}
	return out
}

func protectionRequiresRoot(entries []sshTunBypassEntry) bool {
	for _, entry := range entries {
		if entry.Source == "ssh-client" && routeIsDirectMainLink(entry.Route) {
			continue
		}
		return true
	}
	return false
}

func routeProtectionGuidance(entries []sshTunBypassEntry, risks []string) string {
	var lines []string
	lines = append(lines, "route-capturing TUN may interrupt existing management or tunnel routes")
	if len(entries) > 0 {
		lines = append(lines, "detected routes that should be protected before enabling TUN:")
		for _, entry := range entries {
			via := entry.Route.Dev
			if entry.Route.Via != "" {
				via = entry.Route.Via + " dev " + entry.Route.Dev
			}
			lines = append(lines, fmt.Sprintf("- %s %s via %s", entry.Source, entry.Prefix, via))
		}
	}
	for _, risk := range risks {
		lines = append(lines, "- "+risk)
	}
	lines = append(lines, "run this command with sudo so clashctl can install high-priority bypass routes, or rerun with --allow-route-risk after confirming another recovery path")
	lines = append(lines, "for custom tunnels, set CLASH_TUN_PROTECT_ROUTES=cidr1,cidr2 before running the command")
	return strings.Join(lines, "\n")
}

func detectConnectedRouteProtectionEntries() ([]sshTunBypassEntry, []string) {
	var entries []sshTunBypassEntry
	var risks []string

	for _, family := range []string{"-4", "-6"} {
		out, err := runSSHGuardIPCommand(family, "route", "show", "table", "main", "scope", "link")
		if err != nil {
			continue
		}
		for _, line := range strings.Split(strings.TrimSpace(string(out)), "\n") {
			line = strings.TrimSpace(line)
			if line == "" {
				continue
			}
			route := parseRouteLine(line)
			if route.Dev == "" || !isLikelyProtectedTunnelInterface(route.Dev) || isClashTunDevice(route.Dev) {
				continue
			}
			dest := strings.Fields(line)[0]
			if dest == "default" {
				continue
			}
			entry, err := routeProtectionEntryFromRoute("connected-route", dest, route, "protect connected tunnel route")
			if err != nil {
				risks = append(risks, err.Error())
				continue
			}
			entries = append(entries, entry)
		}
	}
	return entries, risks
}

func detectWireGuardProtectionEntries() ([]sshTunBypassEntry, []string) {
	var entries []sshTunBypassEntry
	var risks []string

	if out, err := exec.Command("wg", "show", "all", "endpoints").Output(); err == nil {
		for _, line := range strings.Split(strings.TrimSpace(string(out)), "\n") {
			fields := strings.Fields(line)
			if len(fields) < 3 {
				continue
			}
			host := endpointHost(fields[2])
			if host == "" {
				continue
			}
			entry, err := buildRouteProtectionEntry("tunnel-endpoint", host, "protect detected tunnel endpoint")
			if err != nil {
				risks = append(risks, err.Error())
				continue
			}
			entries = append(entries, entry)
		}
	}

	if out, err := exec.Command("wg", "show", "all", "allowed-ips").Output(); err == nil {
		for _, line := range strings.Split(strings.TrimSpace(string(out)), "\n") {
			fields := strings.Fields(line)
			if len(fields) < 3 {
				continue
			}
			for _, allowed := range fields[2:] {
				if prefix, err := parseRouteProtectionPrefix(allowed); err == nil && isDefaultRoutePrefix(prefix) {
					continue
				}
				entry, err := buildRouteProtectionEntry("tunnel-route", allowed, "protect detected tunnel allowed IP route")
				if err != nil {
					risks = append(risks, err.Error())
					continue
				}
				entries = append(entries, entry)
			}
		}
	}

	return entries, risks
}

func routeProtectionEntryFromRoute(source, prefixText string, route routeInfo, reason string) (sshTunBypassEntry, error) {
	prefix, err := parseRouteProtectionPrefix(prefixText)
	if err != nil {
		return sshTunBypassEntry{}, fmt.Errorf("invalid %s protected route %q: %w", source, prefixText, err)
	}
	if isDefaultRoutePrefix(prefix) {
		return sshTunBypassEntry{}, fmt.Errorf("invalid %s protected route %q: default routes cannot be protected because that would bypass the whole TUN", source, prefixText)
	}
	return sshTunBypassEntry{
		Family:   ipFamily(prefix.Addr()),
		Prefix:   prefix.String(),
		Table:    sshTunBypassTable,
		Priority: sshTunBypassPriority,
		Route:    route,
		Source:   source,
		Reason:   reason,
	}, nil
}

func isDefaultRoutePrefix(prefix netip.Prefix) bool {
	return prefix.Bits() == 0
}

func endpointHost(endpoint string) string {
	endpoint = strings.TrimSpace(endpoint)
	if endpoint == "" || endpoint == "(none)" {
		return ""
	}
	if strings.HasPrefix(endpoint, "[") {
		end := strings.Index(endpoint, "]")
		if end > 1 {
			return endpoint[1:end]
		}
	}
	idx := strings.LastIndex(endpoint, ":")
	if idx <= 0 {
		return endpoint
	}
	return endpoint[:idx]
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

func isClashTunDevice(name string) bool {
	lower := strings.ToLower(name)
	return strings.Contains(lower, "mihomo") || strings.Contains(lower, "clash") || strings.Contains(lower, "sakurai")
}

func isLikelyProtectedTunnelInterface(name string) bool {
	lower := strings.ToLower(strings.TrimSpace(name))
	if lower == "" || lower == "lo" || isClashTunDevice(lower) {
		return false
	}
	skipPrefixes := []string{"eth", "en", "wl", "docker", "br-", "veth", "virbr", "cni", "flannel"}
	for _, prefix := range skipPrefixes {
		if strings.HasPrefix(lower, prefix) {
			return false
		}
	}
	protectedTokens := []string{"wg", "tun", "tap", "tailscale", "zt", "zerotier", "vpn", "ppp", "ipsec", "warp", "nebula", "netbird", "openvpn", "ovpn", "tinc"}
	for _, token := range protectedTokens {
		if strings.Contains(lower, token) {
			return true
		}
	}
	return false
}
