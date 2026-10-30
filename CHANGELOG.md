# Changelog

What changes in each release of layerlock. Read it before you update, so you know what to expect.

The release page on GitHub shows the same notes.

## 1.1.0 (2026-10-30)

### Before you update

- The release files have new names: `layerlock-linux-aarch64`, `layerlock-linux-armv7`, `layerlock-linux-armv6`,
  `layerlock-linux-x86_64`, and `layerlock-checksums.txt`. If your own scripts download them, update the names.
  An `install.sh` that you saved before this release can't install 1.1.0, so download it again. The new
  `install.sh` installs both new and old releases.

### New

- A build for ARMv6: the Pi Zero, Pi Zero W, and Pi 1. `install.sh` used to install the ARMv7 build there,
  which doesn't run on them.
- layerlock is smaller and lighter: each binary is about 700 KB instead of 6 MB, and a check uses less than 1 MB
  of memory instead of 6 MB.
- `--help` shows the flags, and `--version` shows the version.
- `install.sh` can check the binary against a checksum that you give it in `BINARY_SHA256`. Then it doesn't
  download the checksums file.

### Changed

- A wrong flag or value exits 255, so systemd fails the unit and you see the error. Before, it exited 2, the
  same code as a paused print, so systemd skipped the unit without telling you.
- layerlock always connects to Moonraker directly, even when `HTTP_PROXY` is set.

## 1.0.0 (2026-04-19)

### New

- layerlock asks Moonraker for the printer's state. It exits 0 when the printer is idle, 1 while it prints, and
  2 while it's paused. Use it in `ExecCondition=` to make systemd skip a unit while the printer is busy.
- On an error or a state that layerlock doesn't know, it exits 255, and systemd fails the unit. With
  `--fail-open`, it exits 0 instead.
- `install.sh` downloads layerlock, checks its SHA-256 checksum, and installs it. Set `PRERELEASE=1` to install
  the latest pre-release.
