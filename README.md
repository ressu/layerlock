# layerlock

Block systemd units and shell scripts from running while your 3D printer is busy.

`layerlock` queries the [Moonraker](https://github.com/Arksine/moonraker) API and exits with a code that tells systemd whether to proceed or skip.

## Why this exists

[Klipper](https://github.com/Klipper3d/klipper) is sensitive to processing delays. If the host system is under load while a print is running — enough to delay gcode reads — the print can fail mid-job.

Two common triggers for exactly that kind of load:

**Backups.** A backup job saturating disk or network IO can spike latency enough to disrupt Klipper's timing. With `layerlock` as an `ExecCondition=`, the backup simply skips itself and tries again next scheduled run.

**Automatic updates.** Updating services like Moonraker or system packages while printing can restart processes that communicate with the printer, dropping the connection and aborting the job. Gating update units behind `layerlock` prevents this without disabling updates entirely.

## How it works

| Printer state         | Exit code | systemd result                  |
| --------------------- | --------- | ------------------------------- |
| printing              | 1         | unit skipped (`Result=skipped`) |
| paused                | 2         | unit skipped                    |
| standby / complete    | 0         | unit runs normally              |
| error / unknown state | 255       | unit fails                      |
| Moonraker unreachable | 255       | unit fails                      |

Use `--fail-open` to treat errors and unknown states as non-blocking (exit 0) instead.

## Installation

### One-liner (verified checksum)

```sh
curl -fsSL https://raw.githubusercontent.com/ressu/layerlock/main/install.sh | sh
```

This installs the latest stable release. The script accepts environment variables if you need something different:

| Variable        | Default | Description                                         |
| --------------- | ------- | --------------------------------------------------- |
| `VERSION`       | latest  | Install a specific version, e.g. `v1.0.0-rc1`      |
| `PRERELEASE`    | `0`     | Set to `1` to install the latest prerelease         |
| `BINARY_SHA256` | —       | SHA-256 of the binary to verify before installing   |

<details>
<summary>Verifying without piping to sh</summary>

If you prefer not to pipe directly to `sh`, download the script first, inspect it, then run it with a checksum you've verified independently from the [Releases](https://github.com/ressu/layerlock/releases) page:

```sh
curl -fsSL https://raw.githubusercontent.com/ressu/layerlock/main/install.sh -o install.sh
# inspect install.sh, then:
BINARY_SHA256="sha256:2fba8d8e86eef0be2bf747f0037dc9a1e025de4ef3ee722f8e9fe3d1b092f6a1" sh install.sh
```

Both the `sha256:<hex>` format shown in the GitHub UI and the bare `<hex>` format from `layerlock_checksums.txt` are accepted. When `BINARY_SHA256` is set the checksums file is not downloaded.

</details>

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
layerlock || exit 1
run-my-task.sh
```

To allow the task to run even when Moonraker is unreachable:

```sh
layerlock --fail-open || exit 1
run-my-task.sh
```

## Flags and Environment Variables

| Flag               | Env var         | Default                 | Description                                        |
| ------------------ | --------------- | ----------------------- | -------------------------------------------------- |
| `--url`            | `MOONRAKER_URL` | `http://localhost:7125` | Moonraker base URL                                 |
| `--timeout`        | —               | `5s`                    | HTTP request timeout                               |
| `--fail-open`      | —               | false                   | Exit 0 on errors and unknown states instead of 255 |
| `--verbose` / `-v` | —               | false                   | Print status to stderr                             |

## Ansible Usage

Use `pre_tasks` to check printer state before any play runs. `--fail-open` ensures the play proceeds if layerlock isn't installed yet or Moonraker is unreachable.

```yaml
- name: "3D printers"
  hosts: printers
  pre_tasks:
    - name: Check if layerlock is installed
      ansible.builtin.stat:
        path: /usr/local/bin/layerlock
      register: layerlock_binary

    - name: Check if printer is idle
      ansible.builtin.command: /usr/local/bin/layerlock --fail-open
      register: layerlock_result
      failed_when: false
      changed_when: false
      when: layerlock_binary.stat.exists

    - name: Skip host if printer is busy
      ansible.builtin.meta: end_host
      when: layerlock_binary.stat.exists and (layerlock_result.rc | default(-1)) in [1, 2]
  roles:
    - common
```

Exit codes 1 (printing) and 2 (paused) both skip the host. Any other result — including errors — allows the play to continue.

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
