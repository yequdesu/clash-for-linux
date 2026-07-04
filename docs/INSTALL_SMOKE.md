# Install Smoke Validation

This project validates installer behavior with `.github/workflows/install-smoke.yml`.

The `Release artifact installer` job runs `scripts/smoke/release_install.sh`. It builds a local `clashctl` release artifact, creates `SHA256SUMS`, serves both over a local HTTP server, and verifies:

- `install.sh --with-tui` downloads the release tarball
- `SHA256SUMS` verification is required
- `clashctl` and `clash-tui` are installed from the artifact
- `install-state.json` records the artifact URL for both components
- `uninstall.sh` removes installed binaries and state

The `systemd service lifecycle` job runs `scripts/smoke/systemd_service.sh` on an Ubuntu runner with systemd. It seeds a fake Mihomo binary that validates configs and listens on the configured API/proxy ports, then verifies:

- `install.sh` writes `/etc/systemd/system/clashctl.service`
- the unit points at the configured kernel path
- `clashctl start` starts the service through systemd
- `systemctl status clashctl` reports the service
- `clashctl status` sees the running kernel
- `clashctl doctor` reaches the fake API and verifies auth
- `clashctl stop` stops the service

The workflow runs `scripts/smoke/distro_install.sh` in these Linux containers:

- Ubuntu 24.04
- Debian 12
- Fedora latest
- Arch Linux base-devel

The smoke test intentionally seeds fake `mihomo`, `yq`, geodata, and `subconverter` binaries so it can verify installer behavior without depending on external release downloads. It checks:

- local `clashctl` build and install path
- `install.sh --force --skip-cli`
- `install-state.json` generation
- `runtime.yaml` generation
- `clashctl status`
- `clashctl config set-port`
- `clashctl config set-api`
- `clashctl config set-dns-mode`
- `clashctl config set-lan`
- `clashctl proxy on` shell output
- `clashctl proxy desktop status` reports desktop support state without changing shell env
- `clashctl doctor`
- `clashctl config doctor`
- `uninstall.sh` cleanup

Warnings from `doctor` are allowed because the smoke environment does not start a real proxy listener. Fatal doctor results fail the smoke test.

Local run example with Docker:

```bash
docker run --rm -v "$PWD:/repo" -w /repo ubuntu:24.04 bash -lc '
  apt-get update &&
  apt-get install -y --no-install-recommends bash ca-certificates curl git procps sed coreutils gzip tar &&
  curl -fsSL https://go.dev/dl/go1.24.1.linux-amd64.tar.gz -o /tmp/go.tar.gz &&
  tar -C /usr/local -xzf /tmp/go.tar.gz &&
  export PATH=/usr/local/go/bin:$PATH &&
  bash scripts/smoke/distro_install.sh
'
```

This smoke test does not replace real VM/systemd validation. `v0.2.0-rc.16` has already passed a real clean-machine install path with Mihomo/yq/geodata downloads; repeat the same validation for the final release tag.

For the full external validation sequence, including real install, release artifact, TUN, desktop proxy, upgrade, geodata, and uninstall checks, use `docs/EXTERNAL_VALIDATION_CHECKLIST.md`.
