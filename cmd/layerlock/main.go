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
	url := flag.String("url", envOr("MOONRAKER_URL", defaultURL), "Moonraker base URL")
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
	case moonraker.StatePrinting:
		if *verbose {
			fmt.Fprintf(os.Stderr, "layerlock: printer state: %s — blocking\n", state)
		}
		os.Exit(1)
	case moonraker.StatePaused:
		if *verbose {
			fmt.Fprintf(os.Stderr, "layerlock: printer state: %s — blocking\n", state)
		}
		os.Exit(2)
	default:
		if *verbose {
			fmt.Fprintf(os.Stderr, "layerlock: printer state: %s — allowing\n", state)
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
