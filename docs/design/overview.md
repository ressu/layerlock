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
src/main.rs        flag parsing, state → exit-code mapping
src/moonraker.rs   HTTP GET, JSON decode, typed State return
```

No global state, no concurrency, no framework — a small blocking HTTP client (`ureq`) and `exit()`.

## Key Design Decisions

### Language: Rust

Originally Go; ported to Rust in October 2026 (see [2026-10-30-rust-port.md](2026-10-30-rust-port.md)).

- Single static binary (musl), zero runtime dependencies, ~700 KB
- A check takes about a millisecond (Go: about 3 ms) — important for the condition check path
- Cross-compiles for all Pi targets with Rust's own linker and musl, no C cross toolchain
- Same build setup as printerkeep

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
| `paused` | 2 | block — print is not done; distinct code so scripts can tell it apart |
| `standby` | 0 | allow |
| `complete` | 0 | allow |
| `error` | 255 | fail |
| unknown state | 255 | fail |
| unreachable, HTTP or decode error | 255 | fail |
| usage error (unknown flag, bad value) | 255 | fail |

Details and rationale: [2026-04-18-exit-codes-and-fail-open.md](2026-04-18-exit-codes-and-fail-open.md).

### Fail-Closed by Default

Errors and unknown states exit 255, which fails an `ExecCondition=` unit instead of
skipping it. A wrong `--url` or a Moonraker that never started should not look like an
idle printer. Setups where the printer is sometimes off by design opt in with
`--fail-open`, which turns every 255 into 0. Errors always go to stderr, and with
`--fail-open` they still log a warning, so a misconfiguration is never silent.

### Distribution

- Pre-built binaries published to **GitHub Releases** per architecture
- **SHA256 checksum file** published alongside binaries
- Installer shell script: detects arch, downloads binary, **verifies checksum**, installs
- This is a deliberate improvement over the `wget | sh` pattern common in the
  Klipper community — the script is auditable and only the verified binary is executed
- A GitHub Actions matrix (`.github/workflows/release.yml`) builds, smoke-tests (under qemu) and publishes
  - Targets: `aarch64`, `armv7`, `armv6`, `x86_64` (all `linux-musl`)

## Target Platform

- Raspberry Pi (primary): ARM64 (Pi 3/4/5), ARMv7 (Pi 2/3/4 32-bit), ARMv6 (Pi Zero, Pi 1)
- Linux x86_64 (secondary, for testing and non-Pi setups)
- Moonraker assumed to be running on `localhost:7125` by default (configurable via `--url` / `MOONRAKER_URL`)
