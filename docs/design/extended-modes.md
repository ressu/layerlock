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

- **`paused` state:** Blocks with its own exit code (2), so a script can already treat it
  differently. Should a flag make it allow (exit 0) for `ExecCondition=` units, which can't
  branch on the code? A paused print is not done but the machine is idle — some tasks may be
  safe to run.

- **Package manager distribution:** AUR, apt PPA — out of scope for now but worth
  pursuing as the project matures.
