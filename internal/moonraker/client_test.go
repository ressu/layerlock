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
