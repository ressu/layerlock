# layerlock

Block systemd units and shell scripts from running while your 3D printer is busy.

`layerlock` queries the [Moonraker](https://github.com/Arksine/moonraker) API and exits with a code that tells systemd whether to proceed or skip.

## How it works

| Printer state | Exit code | systemd result |
|---------------|-----------|----------------|
| printing | 1 | unit skipped (`Result=skipped`) |
| paused | 1 | unit skipped |
| standby / complete / error | 0 | unit runs normally |
| Moonraker unreachable | 0 | unit runs normally (fail open) |

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

### Skip if printing
```sh
layerlock || { echo "Printer is busy, skipping."; exit 0; }
run-my-task.sh
```

### Abort if printing
```sh
layerlock || { echo "Printer is busy, aborting."; exit 1; }
run-my-task.sh
```

## Flags and Environment Variables

| Flag | Env var | Default | Description |
|------|---------|---------|-------------|
| `--url` | `LAYERLOCK_URL` | `http://localhost:7125` | Moonraker base URL |
| `--timeout` | — | `5s` | HTTP request timeout |
| `--verbose` / `-v` | — | false | Print status to stderr |

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
