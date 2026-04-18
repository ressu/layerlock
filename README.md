# layerlock

Block systemd units and shell scripts from running while your 3D printer is busy.

`layerlock` queries the [Moonraker](https://github.com/Arksine/moonraker) API and exits with a code that tells systemd whether to proceed or skip.

## Why this exists

[Klipper](https://github.com/Klipper3d/klipper) is sensitive to processing delays. If the host system is under load while a print is running — enough to delay gcode reads — the print can fail mid-job.

Two common triggers for exactly that kind of load:

**Backups.** A backup job saturating disk or network IO can spike latency enough to disrupt Klipper's timing. With `layerlock` as an `ExecCondition=`, the backup simply skips itself and tries again next scheduled run.

**Automatic updates.** Updating services like Moonraker or system packages while printing can restart processes that communicate with the printer, dropping the connection and aborting the job. Gating update units behind `layerlock` prevents this without disabling updates entirely.

## How it works

| Printer state              | Exit code | systemd result                  |
| -------------------------- | --------- | ------------------------------- |
| printing                   | 1         | unit skipped (`Result=skipped`) |
| paused                     | 1         | unit skipped                    |
| standby / complete / error | 0         | unit runs normally              |
| Moonraker unreachable      | 0         | unit runs normally (fail open)  |

## Installation

### One-liner (verified checksum)

```sh
curl -fsSL https://raw.githubusercontent.com/ressu/layerlock/main/install.sh | sh
```

### Manual

Download the binary for your architecture from [Releases](https://github.com/ressu/layerlock/releases), verify the SHA256 checksum, and place it in `/usr/local/bin/`.

```sh
# Example for arm64
curl -LO https://github.com/ressu/layerlock/releases/latest/download/layerlock_linux_arm64
curl -LO https://github.com/ressu/layerlock/releases/latest/download/layerlock_checksums.txt
grep layerlock_linux_arm64 layerlock_checksums.txt | sha256sum -c
chmod +x layerlock_linux_arm64
sudo mv layerlock_linux_arm64 /usr/local/bin/layerlock
```

## systemd Usage

Add `ExecCondition=` to any `.service` unit:

```ini
[Service]
ExecCondition=/usr/local/bin/layerlock
ExecStart=/usr/bin/your-task
```

When the printer is busy, systemd skips the unit cleanly (`Result=skipped`) — no failure, no cascade, no noise. See `examples/backup.service` for a complete example.

## Shell Script Usage

```sh
layerlock || exit 0
run-my-task.sh
```

## Flags and Environment Variables

| Flag               | Env var         | Default                 | Description            |
| ------------------ | --------------- | ----------------------- | ---------------------- |
| `--url`            | `MOONRAKER_URL` | `http://localhost:7125` | Moonraker base URL     |
| `--timeout`        | —               | `5s`                    | HTTP request timeout   |
| `--verbose` / `-v` | —               | false                   | Print status to stderr |

## Building from Source

Requires Go 1.21+.

```sh
git clone https://github.com/ressu/layerlock
cd layerlock
go build ./cmd/layerlock/
```

Cross-compile for Raspberry Pi (arm64):

```sh
GOOS=linux GOARCH=arm64 go build -o layerlock-arm64 ./cmd/layerlock/
```
