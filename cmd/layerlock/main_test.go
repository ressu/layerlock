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
	if got := runLayerlock(t, srv.URL); got != 2 {
		t.Errorf("paused: got exit %d, want 2", got)
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
	if got := runLayerlock(t, srv.URL); got != 255 {
		t.Errorf("error state: got exit %d, want 255", got)
	}
}

func TestExitCode_UnknownState(t *testing.T) {
	srv := moonrakerServer("some_new_state")
	defer srv.Close()
	if got := runLayerlock(t, srv.URL); got != 255 {
		t.Errorf("unknown state: got exit %d, want 255", got)
	}
}

func TestExitCode_Unreachable(t *testing.T) {
	// Nothing listening on this port.
	if got := runLayerlock(t, "http://127.0.0.1:19998"); got != 255 {
		t.Errorf("unreachable: got exit %d, want 255", got)
	}
}

func runLayerlockArgs(t *testing.T, args ...string) int {
	t.Helper()
	cmd := exec.Command(binaryPath, args...)
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

func TestFailOpen_Unreachable(t *testing.T) {
	if got := runLayerlockArgs(t, "--url", "http://127.0.0.1:19998", "--fail-open"); got != 0 {
		t.Errorf("unreachable --fail-open: got exit %d, want 0", got)
	}
}

func TestFailOpen_UnknownState(t *testing.T) {
	srv := moonrakerServer("some_new_state")
	defer srv.Close()
	if got := runLayerlockArgs(t, "--url", srv.URL, "--fail-open"); got != 0 {
		t.Errorf("unknown state --fail-open: got exit %d, want 0", got)
	}
}

func TestFailOpen_ErrorState(t *testing.T) {
	srv := moonrakerServer("error")
	defer srv.Close()
	if got := runLayerlockArgs(t, "--url", srv.URL, "--fail-open"); got != 0 {
		t.Errorf("error state --fail-open: got exit %d, want 0", got)
	}
}

func TestFailOpen_DoesNotAffectPrinting(t *testing.T) {
	srv := moonrakerServer("printing")
	defer srv.Close()
	if got := runLayerlockArgs(t, "--url", srv.URL, "--fail-open"); got != 1 {
		t.Errorf("printing --fail-open: got exit %d, want 1", got)
	}
}

func TestFailOpen_DoesNotAffectPaused(t *testing.T) {
	srv := moonrakerServer("paused")
	defer srv.Close()
	if got := runLayerlockArgs(t, "--url", srv.URL, "--fail-open"); got != 2 {
		t.Errorf("paused --fail-open: got exit %d, want 2", got)
	}
}

func TestEnvVar_URL(t *testing.T) {
	srv := moonrakerServer("printing")
	defer srv.Close()

	cmd := exec.Command(binaryPath)
	cmd.Env = append(os.Environ(), "MOONRAKER_URL="+srv.URL)
	err := cmd.Run()
	if exitErr, ok := err.(*exec.ExitError); ok {
		if exitErr.ExitCode() != 1 {
			t.Errorf("MOONRAKER_URL: got exit %d, want 1", exitErr.ExitCode())
		}
		return
	}
	t.Errorf("expected exit 1 via env var, got nil error")
}
