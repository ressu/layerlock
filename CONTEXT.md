# LayerLock — Project Context

## What is this?

`layerlock` is a small compiled binary that queries the Moonraker API to determine
whether a 3D printer is currently printing, and returns an exit code based on that
state. This allows systemd units and shell scripts to block or skip execution while
the printer is busy.

## Core Use Cases

- Block systemd units from starting while a print is in progress (`ExecCondition=`)
- Block ad-hoc shell scripts from running while printing
- (Planned) Lock tasks that should only run *while* printing
- (Planned) Wait/block until a print is complete before proceeding

## Key Design Decisions

### Language: Go
- Single static binary, zero runtime dependencies
- Fast startup (5–20ms vs Python's 50–150ms+), important for condition check path
- Trivial cross-compilation for ARM/ARM64 (Raspberry Pi target audience)
- HTTP client is stdlib — no external dependencies needed

### systemd Integration: `ExecCondition=`
- Introduced in systemd v243
- Exit `0` → unit starts normally
- Exit `1–254` → unit is **skipped gracefully** (`Result=skipped`), no failure cascade
- Exit `255` → hard failure
- This is the correct directive (not `ExecStartPre=`, which would *fail* the unit)

Example unit usage:
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
Relevant states: `standby`, `printing`, `paused`, `error`, `complete`

Decision: if Moonraker is **unreachable**, fail open (exit 0, allow execution).
This is a deliberate choice — printer being off shouldn't block maintenance tasks.
This should be configurable via a flag in future.

### Distribution
- Pre-built binaries published to **GitHub Releases** per architecture
- **SHA256 checksum file** published alongside binaries
- Installer shell script: detects arch, downloads binary, **verifies checksum**, installs
- This is a deliberate improvement over the `wget | sh` pattern common in the
  Klipper community — the script itself is auditable and small; only the verified
  binary is executed
- **GoReleaser** handles cross-compilation and release automation
  - Targets: `linux/amd64`, `linux/arm64`, `linux/armv7`
  - Checksum algorithm: SHA256

### Project Name
**LayerLock** — chosen over SpoolLock, SpoolGuard, PrintFence, PrintGuard.
- Alliterative, memorable, CLI-friendly
- "Spool" was rejected as it implies filament spool management
- Minor "layer inspection" ambiguity considered acceptable — locking semantics dominate

Binary name: `layerlock`
Repository: `github.com/<user>/layerlock`

## Target Platform
- Raspberry Pi (primary): ARM64 (Pi 4/5), ARMv7 (Pi 2/3 32-bit)
- Linux x86_64 (secondary, for testing and non-Pi setups)
- Moonraker assumed to be running on `localhost:7125` by default (configurable)
