# LayerLock — Implementation Plan

## Phase 1: Core Binary (MVP)

### 1.1 Project Scaffolding
- [ ] Initialise Go module: `go mod init github.com/<user>/layerlock`
- [ ] Set up directory structure:
  ```
  layerlock/
  ├── cmd/layerlock/main.go   # entrypoint
  ├── internal/
  │   └── moonraker/
  │       └── client.go       # API client
  ├── .goreleaser.yaml
  ├── .github/workflows/
  │   └── release.yml
  ├── install.sh
  ├── CONTEXT.md
  ├── PLAN.md
  └── README.md
  ```

### 1.2 Moonraker Client (`internal/moonraker/client.go`)
- [ ] HTTP GET to `/printer/objects/query?print_stats`
- [ ] Parse `result.status.print_stats.state`
- [ ] Return typed state + error
- [ ] Configurable base URL (default: `http://localhost:7125`)
- [ ] Configurable timeout (default: 5s)

### 1.3 Main Logic (`cmd/layerlock/main.go`)
- [ ] Flag parsing:
  - `--url` / `LAYERLOCK_URL` env var for Moonraker base URL
  - `--timeout` for HTTP timeout
  - `--verbose` / `-v` for stderr output (silent by default)
  - `--mode` for future use (default: `block-while-printing`)
- [ ] Exit code logic:
  - `printing` or `paused` → exit 1 (blocked)
  - `standby`, `complete`, `error` → exit 0 (allowed)
  - Moonraker unreachable → exit 0 (fail open), log warning if verbose
  - Unknown state → exit 0 (fail open), log warning if verbose
- [ ] stderr-only output (stdout must stay clean for pipe-friendliness)

### 1.4 Testing
- [ ] Unit tests for state → exit code mapping
- [ ] Unit tests for HTTP client with a mock server
- [ ] Test unreachable Moonraker behaviour

---

## Phase 2: Distribution

### 2.1 GoReleaser Setup (`.goreleaser.yaml`)
- [ ] Targets: `linux/amd64`, `linux/arm64`, `linux/arm` (GOARM=7)
- [ ] `CGO_ENABLED=0`, strip debug info (`-s -w` ldflags)
- [ ] SHA256 checksum file
- [ ] Binary named `layerlock` (not `layerlock_linux_amd64` for install simplicity)

### 2.2 GitHub Actions Release Workflow
- [ ] Trigger on `git tag v*`
- [ ] Run GoReleaser
- [ ] Publish binaries + checksums to GitHub Releases

### 2.3 Installer Script (`install.sh`)
- [ ] Detect architecture (`uname -m` → amd64/arm64/armv7)
- [ ] Resolve latest version from GitHub API if not specified
- [ ] Download binary + checksum file to `/tmp`
- [ ] Verify SHA256 checksum before any installation
- [ ] `chmod +x` and move to `/usr/local/bin/layerlock`
- [ ] Print installed version confirmation
- [ ] Honour `VERSION` and `INSTALL_DIR` env var overrides
- [ ] Clean up temp files on exit (trap)

---

## Phase 3: Documentation

### 3.1 README.md
- [ ] What it does and why
- [ ] Installation (one-liner + manual)
- [ ] systemd `ExecCondition=` usage example
- [ ] Shell script usage examples (abort and wait-until-idle patterns)
- [ ] All flags and environment variables documented
- [ ] Building from source instructions

### 3.2 Example Unit Files
- [ ] `examples/backup.service` — a unit that skips when printing
- [ ] `examples/maintenance.service` — a unit that waits for idle (Phase 4)

---

## Phase 4: Extended Modes (Future)

### 4.1 Wait Mode (`--mode wait`)
- Instead of exiting immediately, poll until printer is idle, then exit 0
- Configurable poll interval (`--interval`, default 30s)
- Configurable max wait (`--max-wait`, 0 = infinite)
- Useful as a script wrapper: `layerlock --mode wait && run-backup.sh`

### 4.2 Invert Mode / Lock-While-Printing (`--mode require-printing`)
- Exit 0 only if printer *is* printing
- Use case: tasks that should only run during a print (e.g. monitoring, logging)

### 4.3 Systemd Notify Integration (stretch)
- `--mode watch`: run as a long-lived service, emit `systemd-notify` signals
  when print state changes, allowing dependents to be started/stopped reactively

---

## Open Questions
- Should `paused` state block (current plan: yes) or allow? Consider making it
  a configurable flag.
- Should unreachable Moonraker be configurable (fail open vs fail closed)?
  Likely worth a `--on-error` flag: `allow` (default) or `block`.
- Package manager distribution (AUR, apt PPA) — out of scope for now but worth
  tracking as the project matures.
