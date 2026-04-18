# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```sh
# Run all tests
go test ./...

# Run tests with race detector (preferred)
go test ./... -race

# Run a single test
go test ./internal/moonraker/... -run TestGetState_Printing -v
go test ./cmd/layerlock/... -run TestExitCode_Printing -v

# Build the binary
go build ./cmd/layerlock/

# Cross-compile for Raspberry Pi targets
GOOS=linux GOARCH=arm64 go build -o layerlock-arm64 ./cmd/layerlock/
GOOS=linux GOARCH=arm GOARM=7 go build -o layerlock-armv7 ./cmd/layerlock/

# Validate GoReleaser config
goreleaser check

# Dry-run release build (produces dist/)
goreleaser release --snapshot --clean
```

## Architecture

Two packages:

**`internal/moonraker`** — HTTP client for the Moonraker API. `NewClient(baseURL, timeout)` returns a `Client`; `GetState()` queries `GET /printer/objects/query?print_stats` and returns a typed `State`. Errors are returned only for network/HTTP/decode failures — unknown state strings come back as-is (not an error). All exit-code policy lives in `main`, not here.

**`cmd/layerlock`** — CLI entrypoint. Parses flags (`--url`, `--timeout`, `--verbose`/`-v`), calls `GetState()`, maps the result to an exit code, and calls `os.Exit`. The mapping: `printing`/`paused` → 1 (block), everything else including errors → 0 (fail open). All output goes to stderr; stdout is intentionally clean.

## Tests

`internal/moonraker/client_test.go` uses `httptest.NewServer` to mock the Moonraker HTTP server. No external dependencies.

`cmd/layerlock/main_test.go` uses `TestMain` to compile the binary once into a temp file, then runs it via `exec.Command` against httptest servers to verify actual exit codes. This is the correct level to test `os.Exit` behaviour — don't try to unit-test `main` directly.

## Design Docs

- `docs/design/overview.md` — architecture decisions, exit code mapping, fail-open rationale, systemd `ExecCondition=` semantics
- `docs/design/extended-modes.md` — planned future modes (wait, require-printing, systemd notify) and open design questions
