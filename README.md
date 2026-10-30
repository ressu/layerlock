# layerlock

Stop systemd units and shell scripts from running while your 3D printer is busy.

`layerlock` asks [Moonraker](https://github.com/Arksine/moonraker) for the printer's state. Its exit code tells systemd or your script whether to go ahead or skip.

## Why this exists

[Klipper](https://github.com/Klipper3d/klipper) is sensitive to delays on the computer it runs on. If that computer gets busy enough to slow down how Klipper reads the G-code, the print can fail partway through.

Two common jobs cause exactly that kind of load:

**Backups.** A backup can keep the disk or network busy long enough to disrupt Klipper's timing. With `layerlock` as an `ExecCondition=`, the backup skips itself while the printer is busy. It tries again at its next scheduled run.

**Automatic updates.** An update of Moonraker or of system packages can restart the services that talk to the printer. That drops the connection and stops the print. With `layerlock` in front of your update units, updates wait for an idle printer, and you don't have to turn them off.

## How it works

| Printer state                            | Exit code | systemd result                  |
| ---------------------------------------- | --------- | ------------------------------- |
| printing                                 | 1         | unit skipped (`Result=skipped`) |
| paused                                   | 2         | unit skipped                    |
| standby or complete                      | 0         | unit runs                       |
| error, or a state layerlock doesn't know | 255       | unit fails                      |
| Moonraker can't be reached               | 255       | unit fails                      |

A unit fails on errors so that you notice a wrong URL or a stopped Moonraker. If your printer is sometimes off on purpose, use `--fail-open`: errors and unknown states then exit 0, and the unit runs.

## Installation

### One-line install (verifies the checksum)

```sh
curl -fsSL https://raw.githubusercontent.com/ressu/layerlock/main/install.sh | sh
```

This installs the latest release. To change what it installs, set these environment variables:

| Variable        | Default          | Description                                       |
| --------------- | ---------------- | ------------------------------------------------- |
| `VERSION`       | latest release   | Version to install, for example `v1.0.0`          |
| `PRERELEASE`    | `0`              | Set to `1` to install the latest pre-release      |
| `INSTALL_DIR`   | `/usr/local/bin` | Folder to install `layerlock` into                |
| `BINARY_SHA256` | none             | SHA-256 checksum the downloaded binary must match |

Put the variables in front of `sh`:

```sh
curl -fsSL https://raw.githubusercontent.com/ressu/layerlock/main/install.sh | VERSION=v1.0.0 sh
```

<details>
<summary>Install without piping to sh</summary>

You can download the script and read it before you run it. To be sure you install the right binary, copy its checksum from the [Releases](https://github.com/ressu/layerlock/releases) page:

```sh
curl -fsSL https://raw.githubusercontent.com/ressu/layerlock/main/install.sh -o install.sh
less install.sh
BINARY_SHA256="<checksum>" sh install.sh
```

Replace `<checksum>` with the binary's SHA-256 checksum. The script accepts both the `sha256:<hex>` form from the GitHub page and the plain `<hex>` form from `layerlock-checksums.txt`. With `BINARY_SHA256` set, the script doesn't download the checksums file.

</details>

### Manual install

Download the binary for your computer from [Releases](https://github.com/ressu/layerlock/releases), check its SHA-256 checksum, and put it in `/usr/local/bin/`. Each binary is a single file that doesn't need any other software.

| Computer                                      | Binary                    |
| --------------------------------------------- | ------------------------- |
| 64-bit ARM (Pi 3, 4, or 5 with a 64-bit OS)   | `layerlock-linux-aarch64` |
| 32-bit ARMv7 (Pi 2, 3, or 4 with a 32-bit OS) | `layerlock-linux-armv7`   |
| ARMv6 (Pi Zero, Pi Zero W, or Pi 1)           | `layerlock-linux-armv6`   |
| x86_64 (most PCs)                             | `layerlock-linux-x86_64`  |

For example, on a Pi with a 64-bit OS:

```sh
curl -LO https://github.com/ressu/layerlock/releases/latest/download/layerlock-linux-aarch64
curl -LO https://github.com/ressu/layerlock/releases/latest/download/layerlock-checksums.txt
grep layerlock-linux-aarch64 layerlock-checksums.txt | sha256sum -c
chmod +x layerlock-linux-aarch64
sudo mv layerlock-linux-aarch64 /usr/local/bin/layerlock
```

Releases up to v1.0.0 use other file names: `layerlock_linux_arm64`, `layerlock_linux_armv7`, `layerlock_linux_amd64`, and `layerlock_checksums.txt`.

## systemd usage

Add `ExecCondition=` to the `[Service]` section of any unit:

```ini
[Service]
ExecCondition=/usr/local/bin/layerlock
ExecStart=/usr/bin/your-task
```

While the printer is busy, systemd skips the unit (`Result=skipped`) instead of failing it. For a complete unit, see [examples/backup.service](examples/backup.service).

## Shell script usage

`layerlock` exits with a non-zero code while the printer is busy, so this script stops before the task:

```sh
layerlock || exit 1
run-my-task.sh
```

To run the task even when Moonraker can't be reached:

```sh
layerlock --fail-open || exit 1
run-my-task.sh
```

## Flags and environment variables

| Flag               | Environment variable | Default                 | Description                                        |
| ------------------ | -------------------- | ----------------------- | -------------------------------------------------- |
| `--url`            | `MOONRAKER_URL`      | `http://localhost:7125` | Moonraker's address                                |
| `--timeout`        | none                 | `5s`                    | How long to wait for Moonraker                     |
| `--fail-open`      | none                 | off                     | Exit 0 instead of 255 on errors and unknown states |
| `--verbose` / `-v` | none                 | off                     | Write the printer state to stderr                  |
| `--help` / `-h`    | none                 |                         | Show the flags                                     |
| `--version`        | none                 |                         | Show the version                                   |

`--timeout` takes a duration such as `500ms`, `5s`, or `1m30s`. You can also write a flag with one dash (`-url`) or give its value after `=` (`--url=http://printer.local:7125`). An unknown flag or a wrong value is an error, and `layerlock` exits 255.

## Ansible usage

To check the printer before a play runs, use `pre_tasks`. The play skips each host whose printer is printing or paused:

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

Every other result lets the play continue. That includes a host where `layerlock` isn't installed yet, and, with `--fail-open`, a host where Moonraker can't be reached.

## Building from source

You need the Rust version in `rust-toolchain.toml`; `rustup` installs it for you. With Nix, `nix develop` gives you that version with all release targets.

```sh
git clone https://github.com/ressu/layerlock
cd layerlock
cargo build --release
```

To build for a Pi with a 64-bit OS, use the musl target. Rust's own linker builds it, so you don't need a C cross toolchain (see `.cargo/config.toml`).

```sh
rustup target add aarch64-unknown-linux-musl
cargo build --release --target aarch64-unknown-linux-musl
```

In `nix develop`, skip the `rustup` line: the target is already installed.
