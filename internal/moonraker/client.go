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
