# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```sh
nix develop                     # optional devShell with everything below (direnv loads it via .envrc)
cargo test
cargo test --test cli exit_codes   # one test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
shellcheck install.sh

# Release build for one target (links with rust-lld, see .cargo/config.toml), and run it (ARM needs qemu)
cargo build --release --target aarch64-unknown-linux-musl
qemu-aarch64 target/aarch64-unknown-linux-musl/release/layerlock --version
```

Release targets: `aarch64-unknown-linux-musl`, `armv7-unknown-linux-musleabihf`,
`arm-unknown-linux-musleabihf` (armv6, test with `qemu-arm -cpu arm1176`), `x86_64-unknown-linux-musl`.

Make a release (no push; push when the user asks): set `version` in `Cargo.toml`, run `cargo check` to update
`Cargo.lock`, commit, and tag `v<version>`. The release workflow refuses a tag that doesn't match `Cargo.toml`.
A tag with a pre-release part (`v1.1.0-rc1`) publishes a GitHub pre-release.

## Architecture

**`src/moonraker.rs`** (library) — HTTP client for the Moonraker API. `Client::new(base_url, timeout)`;
`get_state()` queries `GET /printer/objects/query?print_stats` and returns a `State` enum. Errors are returned
only for network/HTTP/decode failures — an unknown state comes back as `State::Other(String)`, not an error.
All exit-code policy lives in `main.rs`, not here.

**`src/main.rs`** (binary) — parses flags by hand (`--url`, `--timeout`, `--fail-open`, `--verbose`/`-v`,
`--help`, `--version`), calls `get_state()`, maps the result to an exit code. Flag parsing stays compatible
with the Go version it replaced: `-flag` equals `--flag`, `--flag=value` works, and `--timeout` takes Go
duration strings (`1m30s`). The mapping: `standby`/`complete` → 0, `printing` → 1, `paused` → 2, everything
else including errors and usage errors → 255 (or 0 with `--fail-open`). The check writes only to stderr;
stdout stays clean. Only `--help` and `--version` print to stdout.

Keep dependencies few: the release binary is ~700 KB and a check takes about a millisecond.

## Tests

`tests/common/mod.rs` is a fake Moonraker on `std::net::TcpListener` (no test dependencies). `tests/moonraker.rs`
tests the client; `tests/cli.rs` runs the built binary (`CARGO_BIN_EXE_layerlock`) and checks exit codes and
stderr — the right level to test `exit()` behaviour. Flag and duration parsing have unit tests in `src/main.rs`.

## Design Docs

- `docs/design/overview.md` — architecture decisions, exit code mapping, systemd `ExecCondition=` semantics
- `docs/design/2026-04-18-exit-codes-and-fail-open.md` — current exit codes and `--fail-open`
- `docs/design/2026-10-30-rust-port.md` — why layerlock moved from Go to Rust, with benchmarks
- `docs/design/extended-modes.md` — planned future modes (wait, require-printing, systemd notify) and open design questions
