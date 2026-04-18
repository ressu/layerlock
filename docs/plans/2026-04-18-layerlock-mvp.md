# LayerLock MVP Implementation Plan

> **For agentic workers:** Execute this plan task-by-task using TDD. Steps use checkbox (`- [ ]`) syntax for tracking. Complete each step before moving to the next. Commit after each task.

**Goal:** Build a single static Go binary that queries Moonraker's print state and exits with a code suitable for use in `systemd ExecCondition=` directives.

**Architecture:** A thin CLI wrapper around an internal Moonraker HTTP client. The client returns a typed state; `main` maps that state to an exit code. No global state, no concurrency, no framework dependencies — just stdlib HTTP and `os.Exit`.

**Tech Stack:** Go (stdlib only), GoReleaser for cross-compilation, GitHub Actions for CI/release.

---

## File Map

| Path | Responsibility |
|------|---------------|
| `cmd/layerlock/main.go` | Flag parsing, state → exit-code mapping, `os.Exit` |
| `internal/moonraker/client.go` | HTTP GET, JSON decode, return typed `State` + `error` |
| `internal/moonraker/client_test.go` | httptest mock server tests for client |
| `cmd/layerlock/main_test.go` | Integration tests for exit-code logic (via `exec.Command`) |
| `go.mod` | Module declaration — no external deps |
| `.goreleaser.yaml` | Cross-compile targets, checksum, strip flags |
| `.github/workflows/release.yml` | Tag-triggered GoReleaser run |
| `install.sh` | Arch detection, download, checksum verify, install |
| `examples/backup.service` | Reference systemd unit using `ExecCondition=` |

---

## Task 1: Go Module Initialisation

**Files:**
- Create: `go.mod`

- [ ] **Step 1: Initialise module**

```bash
cd /home/ressu/sources/layerlock
go mod init github.com/ressu/layerlock
```

Expected output:
```
go: creating new go.mod: module github.com/ressu/layerlock
```

- [ ] **Step 2: Verify go.mod**

```bash
cat go.mod
```

Expected:
```
module github.com/ressu/layerlock

go 1.21
```

- [ ] **Step 3: Commit**

```bash
git add go.mod
git commit -m "chore: initialise Go module"
```

---

## Task 2: Moonraker Client — Types and Interface

**Files:**
- Create: `internal/moonraker/client.go`
- Create: `internal/moonraker/client_test.go`

The Moonraker endpoint returns:

```json
{
  "result": {
    "status": {
      "print_stats": {
        "state": "standby"
      }
    }
  }
}
```

Known states: `standby`, `printing`, `paused`, `error`, `complete`.

- [ ] **Step 1: Write the failing test for a `printing` response**

Create `internal/moonraker/client_test.go`:

```go
package moonraker_test

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	"github.com/ressu/layerlock/internal/moonraker"
)

func mockServer(state string) *httptest.Server {
	return httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/printer/objects/query" || r.URL.Query().Get("print_stats") == "" {
			http.NotFound(w, r)
			return
		}
		body := map[string]any{
			"result": map[string]any{
				"status": map[string]any{
					"print_stats": map[string]any{
						"state": state,
					},
				},
			},
		}
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(body)
	}))
}

func TestGetState_Printing(t *testing.T) {
	srv := mockServer("printing")
	defer srv.Close()

	c := moonraker.NewClient(srv.URL, 5*time.Second)
	state, err := c.GetState()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if state != moonraker.StatePrinting {
		t.Errorf("got %q, want %q", state, moonraker.StatePrinting)
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
go test ./internal/moonraker/...
```

Expected: compile error — `moonraker` package does not exist yet.

- [ ] **Step 3: Write the minimal client implementation**

Create `internal/moonraker/client.go`:

```go
package moonraker

import (
	"encoding/json"
	"fmt"
	"net/http"
	"time"
)

// State represents the print_stats.state value from Moonraker.
type State string

const (
	StateStandby  State = "standby"
	StatePrinting State = "printing"
	StatePaused   State = "paused"
	StateError    State = "error"
	StateComplete State = "complete"
)

// Client queries a Moonraker instance.
type Client struct {
	baseURL    string
	httpClient *http.Client
}

// NewClient creates a Client with the given base URL and request timeout.
func NewClient(baseURL string, timeout time.Duration) *Client {
	return &Client{
		baseURL: baseURL,
		httpClient: &http.Client{Timeout: timeout},
	}
}

type response struct {
	Result struct {
		Status struct {
			PrintStats struct {
				State State `json:"state"`
			} `json:"print_stats"`
		} `json:"status"`
	} `json:"result"`
}

// GetState queries Moonraker and returns the current print state.
// Returns an error only for HTTP/network failures or malformed responses.
// An unknown state string is returned as-is (not an error).
func (c *Client) GetState() (State, error) {
	url := c.baseURL + "/printer/objects/query?print_stats"
	resp, err := c.httpClient.Get(url)
	if err != nil {
		return "", fmt.Errorf("moonraker unreachable: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return "", fmt.Errorf("moonraker returned HTTP %d", resp.StatusCode)
	}

	var r response
	if err := json.NewDecoder(resp.Body).Decode(&r); err != nil {
		return "", fmt.Errorf("decode response: %w", err)
	}

	return r.Result.Status.PrintStats.State, nil
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
go test ./internal/moonraker/... -v
```

Expected:
```
--- PASS: TestGetState_Printing (0.00s)
PASS
```

- [ ] **Step 5: Commit**

```bash
git add internal/moonraker/client.go internal/moonraker/client_test.go
git commit -m "feat: add Moonraker HTTP client"
```

---

## Task 3: Moonraker Client — Full State Coverage

**Files:**
- Modify: `internal/moonraker/client_test.go`

- [ ] **Step 1: Add tests for all states and error cases**

Append to `internal/moonraker/client_test.go`:

```go
func TestGetState_AllStates(t *testing.T) {
	cases := []struct {
		moonrakerState string
		wantState      moonraker.State
	}{
		{"standby", moonraker.StateStandby},
		{"printing", moonraker.StatePrinting},
		{"paused", moonraker.StatePaused},
		{"error", moonraker.StateError},
		{"complete", moonraker.StateComplete},
		{"unknown_future_state", moonraker.State("unknown_future_state")},
	}

	for _, tc := range cases {
		t.Run(tc.moonrakerState, func(t *testing.T) {
			srv := mockServer(tc.moonrakerState)
			defer srv.Close()

			c := moonraker.NewClient(srv.URL, 5*time.Second)
			state, err := c.GetState()
			if err != nil {
				t.Fatalf("unexpected error: %v", err)
			}
			if state != tc.wantState {
				t.Errorf("got %q, want %q", state, tc.wantState)
			}
		})
	}
}

func TestGetState_Unreachable(t *testing.T) {
	// Point at a port nothing is listening on.
	c := moonraker.NewClient("http://127.0.0.1:19999", 500*time.Millisecond)
	_, err := c.GetState()
	if err == nil {
		t.Fatal("expected error for unreachable server, got nil")
	}
}

func TestGetState_NonOKStatus(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		http.Error(w, "not found", http.StatusNotFound)
	}))
	defer srv.Close()

	c := moonraker.NewClient(srv.URL, 5*time.Second)
	_, err := c.GetState()
	if err == nil {
		t.Fatal("expected error for non-200 status, got nil")
	}
}
```

- [ ] **Step 2: Run tests**

```bash
go test ./internal/moonraker/... -v
```

Expected: all tests pass.

- [ ] **Step 3: Commit**

```bash
git add internal/moonraker/client_test.go
git commit -m "test: full state and error coverage for Moonraker client"
```

---

## Task 4: Main Binary — Exit Code Logic

**Files:**
- Create: `cmd/layerlock/main.go`

Exit code mapping (from CONTEXT.md):

| State | Exit code | Meaning |
|-------|-----------|---------|
| `printing` | 1 | blocked |
| `paused` | 1 | blocked |
| `standby` | 0 | allowed |
| `complete` | 0 | allowed |
| `error` | 0 | allowed (fail open) |
| unknown state | 0 | allowed (fail open) |
| unreachable | 0 | allowed (fail open) |

- [ ] **Step 1: Write main.go**

```go
package main

import (
	"flag"
	"fmt"
	"os"
	"time"

	"github.com/ressu/layerlock/internal/moonraker"
)

const defaultURL = "http://localhost:7125"
const defaultTimeout = 5 * time.Second

func main() {
	url := flag.String("url", envOr("LAYERLOCK_URL", defaultURL), "Moonraker base URL")
	timeout := flag.Duration("timeout", defaultTimeout, "HTTP request timeout")
	verbose := flag.Bool("verbose", false, "Write status to stderr")
	flag.BoolVar(verbose, "v", false, "Write status to stderr (shorthand)")
	flag.Parse()

	client := moonraker.NewClient(*url, *timeout)
	state, err := client.GetState()

	if err != nil {
		if *verbose {
			fmt.Fprintf(os.Stderr, "layerlock: warning: %v (failing open)\n", err)
		}
		os.Exit(0)
	}

	switch state {
	case moonraker.StatePrinting, moonraker.StatePaused:
		if *verbose {
			fmt.Fprintf(os.Stderr, "layerlock: printer is %s — blocking\n", state)
		}
		os.Exit(1)
	default:
		if *verbose {
			fmt.Fprintf(os.Stderr, "layerlock: printer is %s — allowing\n", state)
		}
		os.Exit(0)
	}
}

func envOr(key, fallback string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return fallback
}
```

- [ ] **Step 2: Build and sanity-check**

```bash
go build ./cmd/layerlock/
./layerlock --help
```

Expected: usage text printed, exit 0.

- [ ] **Step 3: Commit**

```bash
git add cmd/layerlock/main.go
git commit -m "feat: add main binary with exit-code logic and flags"
```

---

## Task 5: Main Binary — Integration Tests

**Files:**
- Create: `cmd/layerlock/main_test.go`

These tests compile the binary and run it against a mock HTTP server via `exec.Command`, verifying the actual exit code. This is the correct level to test the full behaviour — unit-testing `os.Exit` is messy and fragile.

- [ ] **Step 1: Write integration tests**

```go
package main_test

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os"
	"os/exec"
	"testing"
)

// buildBinary compiles the layerlock binary to a temp file and returns its path.
// It is called once per test run via TestMain.
var binaryPath string

func TestMain(m *testing.M) {
	tmp, err := os.CreateTemp("", "layerlock-test-*")
	if err != nil {
		panic(err)
	}
	tmp.Close()
	binaryPath = tmp.Name()

	out, err := exec.Command("go", "build", "-o", binaryPath, ".").CombinedOutput()
	if err != nil {
		panic("build failed: " + string(out))
	}
	defer os.Remove(binaryPath)

	os.Exit(m.Run())
}

func moonrakerServer(state string) *httptest.Server {
	return httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		body := map[string]any{
			"result": map[string]any{
				"status": map[string]any{
					"print_stats": map[string]any{"state": state},
				},
			},
		}
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(body)
	}))
}

func runLayerlock(t *testing.T, url string) int {
	t.Helper()
	cmd := exec.Command(binaryPath, "--url", url)
	err := cmd.Run()
	if err == nil {
		return 0
	}
	if exitErr, ok := err.(*exec.ExitError); ok {
		return exitErr.ExitCode()
	}
	t.Fatalf("unexpected error running binary: %v", err)
	return -1
}

func TestExitCode_Printing(t *testing.T) {
	srv := moonrakerServer("printing")
	defer srv.Close()
	if got := runLayerlock(t, srv.URL); got != 1 {
		t.Errorf("printing: got exit %d, want 1", got)
	}
}

func TestExitCode_Paused(t *testing.T) {
	srv := moonrakerServer("paused")
	defer srv.Close()
	if got := runLayerlock(t, srv.URL); got != 1 {
		t.Errorf("paused: got exit %d, want 1", got)
	}
}

func TestExitCode_Standby(t *testing.T) {
	srv := moonrakerServer("standby")
	defer srv.Close()
	if got := runLayerlock(t, srv.URL); got != 0 {
		t.Errorf("standby: got exit %d, want 0", got)
	}
}

func TestExitCode_Complete(t *testing.T) {
	srv := moonrakerServer("complete")
	defer srv.Close()
	if got := runLayerlock(t, srv.URL); got != 0 {
		t.Errorf("complete: got exit %d, want 0", got)
	}
}

func TestExitCode_Error(t *testing.T) {
	srv := moonrakerServer("error")
	defer srv.Close()
	if got := runLayerlock(t, srv.URL); got != 0 {
		t.Errorf("error state: got exit %d, want 0", got)
	}
}

func TestExitCode_UnknownState(t *testing.T) {
	srv := moonrakerServer("some_new_state")
	defer srv.Close()
	if got := runLayerlock(t, srv.URL); got != 0 {
		t.Errorf("unknown state: got exit %d, want 0", got)
	}
}

func TestExitCode_Unreachable(t *testing.T) {
	// Nothing listening on this port.
	if got := runLayerlock(t, "http://127.0.0.1:19998"); got != 0 {
		t.Errorf("unreachable: got exit %d, want 0", got)
	}
}

func TestEnvVar_URL(t *testing.T) {
	srv := moonrakerServer("printing")
	defer srv.Close()

	cmd := exec.Command(binaryPath)
	cmd.Env = append(os.Environ(), "LAYERLOCK_URL="+srv.URL)
	err := cmd.Run()
	if exitErr, ok := err.(*exec.ExitError); ok {
		if exitErr.ExitCode() != 1 {
			t.Errorf("LAYERLOCK_URL: got exit %d, want 1", exitErr.ExitCode())
		}
		return
	}
	t.Errorf("expected exit 1 via env var, got nil error")
}
```

- [ ] **Step 2: Run all tests**

```bash
go test ./... -v
```

Expected: all tests pass.

- [ ] **Step 3: Commit**

```bash
git add cmd/layerlock/main_test.go
git commit -m "test: integration tests for exit-code logic and env var"
```

---

## Task 6: GoReleaser Configuration

**Files:**
- Create: `.goreleaser.yaml`

- [ ] **Step 1: Write GoReleaser config**

```yaml
# .goreleaser.yaml
version: 2

builds:
  - id: layerlock
    main: ./cmd/layerlock
    binary: layerlock
    env:
      - CGO_ENABLED=0
    ldflags:
      - -s -w
    goos:
      - linux
    goarch:
      - amd64
      - arm64
      - arm
    goarm:
      - "7"

archives:
  - id: layerlock
    builds: [layerlock]
    format: binary  # ship raw binaries, not tarballs
    name_template: "layerlock_{{ .Os }}_{{ .Arch }}{{ if .Arm }}v{{ .Arm }}{{ end }}"

checksum:
  name_template: "layerlock_checksums.txt"
  algorithm: sha256

release:
  draft: false
  prerelease: auto
```

- [ ] **Step 2: Validate the config (requires goreleaser installed)**

```bash
goreleaser check
```

Expected: `• config is valid`

If goreleaser is not installed:
```bash
go install github.com/goreleaser/goreleaser/v2@latest
goreleaser check
```

- [ ] **Step 3: Commit**

```bash
git add .goreleaser.yaml
git commit -m "chore: add GoReleaser config for linux/amd64/arm64/armv7"
```

---

## Task 7: GitHub Actions Release Workflow

**Files:**
- Create: `.github/workflows/release.yml`

- [ ] **Step 1: Write the workflow**

```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    tags:
      - "v*"

permissions:
  contents: write  # needed to create GitHub Releases

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0  # GoReleaser needs full git history for changelog

      - uses: actions/setup-go@v5
        with:
          go-version-file: go.mod
          cache: true

      - name: Run tests
        run: go test ./...

      - uses: goreleaser/goreleaser-action@v6
        with:
          version: latest
          args: release --clean
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: add GoReleaser release workflow on v* tags"
```

---

## Task 8: Installer Script

**Files:**
- Create: `install.sh`

- [ ] **Step 1: Write install.sh**

```bash
#!/usr/bin/env sh
# install.sh — Download, verify, and install layerlock
# Usage: sh install.sh [VERSION] [INSTALL_DIR]
#   VERSION     defaults to latest GitHub release
#   INSTALL_DIR defaults to /usr/local/bin
set -eu

REPO="ressu/layerlock"
VERSION="${VERSION:-}"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
TMP_DIR="$(mktemp -d)"

cleanup() { rm -rf "$TMP_DIR"; }
trap cleanup EXIT

# Detect architecture
case "$(uname -m)" in
  x86_64)             ARCH="amd64"  ;;
  aarch64|arm64)      ARCH="arm64"  ;;
  armv7*|armv6*)      ARCH="armv7"  ;;
  *) echo "Unsupported architecture: $(uname -m)" >&2; exit 1 ;;
esac

# Resolve latest version if not specified
if [ -z "$VERSION" ]; then
  VERSION="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep '"tag_name"' | sed 's/.*"tag_name": *"\(.*\)".*/\1/')"
fi

echo "Installing layerlock ${VERSION} (${ARCH})..."

BASE_URL="https://github.com/${REPO}/releases/download/${VERSION}"
BINARY_NAME="layerlock_linux_${ARCH}"
CHECKSUM_FILE="layerlock_checksums.txt"

# Download binary and checksum file
curl -fsSL "${BASE_URL}/${BINARY_NAME}" -o "${TMP_DIR}/layerlock"
curl -fsSL "${BASE_URL}/${CHECKSUM_FILE}" -o "${TMP_DIR}/${CHECKSUM_FILE}"

# Verify checksum
cd "$TMP_DIR"
grep "${BINARY_NAME}" "${CHECKSUM_FILE}" | sha256sum -c -

chmod +x layerlock
mv layerlock "${INSTALL_DIR}/layerlock"

echo "Installed to ${INSTALL_DIR}/layerlock"
"${INSTALL_DIR}/layerlock" --version 2>/dev/null || true
```

- [ ] **Step 2: Make it executable and lint with shellcheck (if available)**

```bash
chmod +x install.sh
shellcheck install.sh || true
```

- [ ] **Step 3: Commit**

```bash
git add install.sh
git commit -m "feat: add installer script with SHA256 checksum verification"
```

---

## Task 9: Example systemd Unit

**Files:**
- Create: `examples/backup.service`

- [ ] **Step 1: Write example unit**

```ini
# examples/backup.service
# A systemd service that skips execution while a 3D print is in progress.
# Install to: /etc/systemd/system/backup.service
#
# layerlock exits 1 while printing (systemd treats 1–254 as "skip gracefully").
# The actual backup runs only when the printer is idle.

[Unit]
Description=Nightly backup (skips while printing)
After=network-online.target

[Service]
Type=oneshot
ExecCondition=/usr/local/bin/layerlock
ExecStart=/usr/bin/your-backup-script.sh

[Install]
WantedBy=multi-user.target
```

- [ ] **Step 2: Commit**

```bash
git add examples/backup.service
git commit -m "docs: add example systemd unit using ExecCondition"
```

---

## Task 10: README

**Files:**
- Create: `README.md`

- [ ] **Step 1: Write README.md**

```markdown
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
```

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs: add README with installation, usage, and flag reference"
```

---

## Final Verification

- [ ] **Run the full test suite**

```bash
go test ./... -v -race
```

Expected: all tests pass, no race conditions.

- [ ] **Build for all target architectures**

```bash
GOOS=linux GOARCH=amd64   go build ./cmd/layerlock/ && echo "amd64 ok"
GOOS=linux GOARCH=arm64   go build ./cmd/layerlock/ && echo "arm64 ok"
GOOS=linux GOARCH=arm GOARM=7 go build ./cmd/layerlock/ && echo "armv7 ok"
```

Expected: all three print `ok`.

- [ ] **Dry-run GoReleaser**

```bash
goreleaser release --snapshot --clean
```

Expected: binaries and checksum file in `dist/`.
