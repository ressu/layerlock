# Exit Codes and `--fail-open` Flag

## Summary

Change the default error behaviour from fail-open (exit 0) to fail-closed (exit 255),
add a `--fail-open` flag to restore the old behaviour, and introduce distinct exit codes
for `printing` vs `paused` to make the tool more composable in scripts and systemd units.

## Exit Code Mapping

| State                           | Exit code | Notes                        |
|---------------------------------|-----------|------------------------------|
| `printing`                      | `1`       | block — unchanged            |
| `paused`                        | `2`       | block — new distinct code    |
| `standby`                       | `0`       | allow — unchanged            |
| `complete`                      | `0`       | allow — unchanged            |
| `error`                         | `255`     | hard error — was `0`         |
| unknown state                   | `255`     | hard error — was `0`         |
| unreachable / HTTP / decode err | `255`     | hard error — was `0`         |

With `--fail-open`, all `255` paths return `0` instead.

## `--fail-open` Flag

```
--fail-open    Treat connection errors and unknown states as non-blocking (exit 0)
```

No shorthand. The long form makes intent explicit in scripts and systemd unit files.

## Verbose Output and stderr Behaviour

Error messages are always written to stderr regardless of `--verbose`. This ensures
errors appear in the systemd journal (`journalctl -u <unit>`) without requiring the
unit file to pass `--verbose`. Silently returning 255 with no message would make
failures hard to diagnose.

Informational state messages (printer is printing, printer state is complete, etc.)
remain behind `--verbose` — they are noise in normal operation.

| Situation                  | `--verbose` off          | `--verbose` on               |
|----------------------------|--------------------------|------------------------------|
| `printing` / `paused`      | silent                   | logs state + action          |
| `standby` / `complete`     | silent                   | logs state + action          |
| error, `--fail-open` off   | always logs error        | always logs error            |
| error, `--fail-open` on    | always logs warning      | always logs warning          |

`--fail-open` errors always log a warning even when the flag is set, so that
misconfiguration (e.g. wrong URL) is never silently swallowed.

## Rationale

### Why fail-closed by default?

The previous fail-open default was a reasonable starting point, but it silently masks
misconfiguration — a wrong `--url` or a Moonraker that never started looks identical
to a healthy idle printer. Fail-closed surfaces the problem immediately.

Users who genuinely want fail-open (printer is sometimes off by design) can opt in
explicitly with `--fail-open`.

### Why separate exit codes for `printing` vs `paused`?

Callers that only care about "is anything blocking" can still treat any non-zero
exit as blocking. Callers that want finer control (e.g. a script that is safe to
run during a paused print) can branch on `$?` without needing extra flags.

### Why `255` for errors?

In `ExecCondition=` semantics: exit `1–254` skips the unit gracefully, exit `255`
causes a hard failure. A connection error should not silently skip a maintenance
unit — it should fail loudly so the operator investigates. `255` maps directly to
this intent.
