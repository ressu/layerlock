# LayerLock — Extended Modes

These modes are planned future work, not yet implemented.

## Wait Mode (`--mode wait`)

Instead of exiting immediately, poll until the printer is idle, then exit 0.

- Configurable poll interval (`--interval`, default 30s)
- Configurable max wait (`--max-wait`, 0 = infinite)
- Useful as a script wrapper: `layerlock --mode wait && run-backup.sh`

## Require-Printing Mode (`--mode require-printing`)

Inverts the logic: exit 0 only if the printer *is* printing.

- Use case: tasks that should only run during a print (monitoring, logging, timelapse capture)

## Systemd Notify Integration (`--mode watch`)

Run as a long-lived service, emit `sd_notify` signals when print state changes.
Allows dependents to be started/stopped reactively via `BindsTo=` or `PartOf=`.

## Open Design Questions

- **`paused` state:** Currently blocks (exit 1). Should this be configurable?
  A paused print is not done but the machine is idle — some tasks may be safe to run.

- **Fail-open vs fail-closed:** Currently unreachable Moonraker → exit 0 (allow).
  A `--on-error allow|block` flag would let users choose. `block` is the safer default
  for setups where the printer is always on.

- **Package manager distribution:** AUR, apt PPA — out of scope for now but worth
  pursuing as the project matures.
