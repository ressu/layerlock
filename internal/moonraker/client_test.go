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
		if r.URL.Path != "/printer/objects/query" || !r.URL.Query().Has("print_stats") {
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
