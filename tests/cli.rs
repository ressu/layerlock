//! Runs the built binary against fake Moonraker servers and checks its exit code and output.

mod common;

use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_layerlock"))
        .args(args)
        .env_remove("MOONRAKER_URL")
        .output()
        .unwrap()
}

fn code(args: &[&str]) -> i32 {
    run(args).status.code().unwrap()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn exit_codes() {
    let cases = [
        ("printing", 1),
        ("paused", 2),
        ("standby", 0),
        ("complete", 0),
        ("error", 255),
        ("some_new_state", 255),
    ];
    for (state, want) in cases {
        let url = common::moonraker(state);
        assert_eq!(code(&["--url", &url]), want, "{state}");
    }
}

#[test]
fn unreachable_fails() {
    assert_eq!(code(&["--url", &common::unreachable()]), 255);
}

#[test]
fn fail_open_allows_errors_and_unknown_states() {
    assert_eq!(code(&["--url", &common::unreachable(), "--fail-open"]), 0);
    for state in ["error", "some_new_state"] {
        let url = common::moonraker(state);
        assert_eq!(code(&["--url", &url, "--fail-open"]), 0, "{state}");
    }
}

#[test]
fn fail_open_still_blocks() {
    for (state, want) in [("printing", 1), ("paused", 2)] {
        let url = common::moonraker(state);
        assert_eq!(code(&["--url", &url, "--fail-open"]), want, "{state}");
    }
}

#[test]
fn error_always_on_stderr() {
    let out = run(&["--url", &common::unreachable()]);
    assert_eq!(out.status.code(), Some(255));
    assert!(
        stderr(&out).contains("layerlock: error: moonraker unreachable"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn fail_open_warning_always_on_stderr() {
    let out = run(&["--url", &common::unreachable(), "--fail-open"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(
        stderr(&out).contains("layerlock: warning:"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn printing_is_silent_without_verbose() {
    let out = run(&["--url", &common::moonraker("printing")]);
    assert_eq!(stderr(&out), "");
    assert!(out.stdout.is_empty());
}

#[test]
fn verbose_shows_state() {
    let url = common::moonraker("printing");
    for flag in ["--verbose", "-v"] {
        let out = run(&["--url", &url, flag]);
        assert_eq!(
            stderr(&out),
            "layerlock: printer state: printing — blocking\n",
            "{flag}"
        );
        assert!(out.stdout.is_empty());
    }
}

#[test]
fn url_from_environment() {
    let status = Command::new(env!("CARGO_BIN_EXE_layerlock"))
        .env("MOONRAKER_URL", common::moonraker("printing"))
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(1));
}

#[test]
fn timeout_flag() {
    let start = std::time::Instant::now();
    assert_eq!(
        code(&["--url", &common::silent(), "--timeout", "200ms"]),
        255
    );
    assert!(start.elapsed() < std::time::Duration::from_secs(2));
}

#[test]
fn usage_error_fails() {
    let out = run(&["--no-such-flag"]);
    assert_eq!(out.status.code(), Some(255));
    assert!(
        stderr(&out).contains("unknown flag --no-such-flag"),
        "{}",
        stderr(&out)
    );
    assert!(stderr(&out).contains("Usage:"));
}

#[test]
fn help_and_version() {
    let out = run(&["--help"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("Usage: layerlock"));

    let out = run(&["--version"]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        format!("layerlock {}\n", env!("CARGO_PKG_VERSION"))
    );
}
