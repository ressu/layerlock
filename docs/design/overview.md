# LayerLock — Design Overview

## What is this?

`layerlock` is a small compiled binary that queries the Moonraker API to determine
whether a 3D printer is currently printing, and returns an exit code based on that
state. This allows systemd units and shell scripts to block or skip execution while
the printer is busy.

## Use Cases

- Block systemd units from starting while a print is in progress (`ExecCondition=`)
- Block ad-hoc shell scripts from running while printing
- Lock tasks that should only run *while* printing (see [extended-modes.md](extended-modes.md))
- Wait/block until a print is complete before proceeding (see [extended-modes.md](extended-modes.md))

## Architecture

A thin CLI wrapper around an internal Moonraker HTTP client:

```
cmd/layerlock/main.go          flag parsing, state → exit-code mapping
internal/moonraker/client.go   HTTP GET, JSON decode, typed State return
```

No global state, no concurrency, no framework dependencies — stdlib HTTP and `os.Exit`.

## Key Design Decisions

### Language: Go

- Single static binary, zero runtime dependencies
- Fast startup (5–20ms vs Python's 50–150ms+) — important for the condition check path
- Trivial cross-compilation for ARM/ARM64 (Raspberry Pi target audience)
- HTTP client is stdlib — no external dependencies needed

### systemd Integration: `ExecCondition=`

Introduced in systemd v243. Exit codes have specific semantics:

- Exit `0` → unit starts normally
- Exit `1–254` → unit is **skipped gracefully** (`Result=skipped`), no failure cascade
- Exit `255` → hard failure

`ExecCondition=` is the correct directive for this use case. `ExecStartPre=` would *fail*
the unit rather than skip it.

```ini
[Service]
ExecCondition=/usr/local/bin/layerlock
ExecStart=/usr/bin/your-actual-task
```

### Moonraker API

Print state is queried via:

```
GET http://localhost:7125/printer/objects/query?print_stats
```

Response field: `result.status.print_stats.state`

Known states: `standby`, `printing`, `paused`, `error`, `complete`

### Exit Code Mapping

| State | Exit code | Rationale |
|-------|-----------|-----------|
| `printing` | 1 | block |
| `paused` | 1 | block — print is not done |
| `standby` | 0 | allow |
| `complete` | 0 | allow |
| `error` | 0 | allow (fail open) |
| unknown state | 0 | allow (fail open) |
| unreachable | 0 | allow (fail open) |

### Fail-Open Policy

When Moonraker is unreachable, the binary exits 0 (allow). This is a deliberate
choice — the printer being off or unreachable should not block maintenance tasks.
A future `--on-error` flag will make this configurable (see [extended-modes.md](extended-modes.md)).

### Distribution

- Pre-built binaries published to **GitHub Releases** per architecture
- **SHA256 checksum file** published alongside binaries
- Installer shell script: detects arch, downloads binary, **verifies checksum**, installs
- This is a deliberate improvement over the `wget | sh` pattern common in the
  Klipper community — the script is auditable and only the verified binary is executed
- **GoReleaser** handles cross-compilation and release automation
  - Targets: `linux/amd64`, `linux/arm64`, `linux/armv7`

## Target Platform

- Raspberry Pi (primary): ARM64 (Pi 4/5), ARMv7 (Pi 2/3 32-bit)
- Linux x86_64 (secondary, for testing and non-Pi setups)
- Moonraker assumed to be running on `localhost:7125` by default (configurable via `--url` / `LAYERLOCK_URL`)
