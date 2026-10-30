//! CLI entrypoint: parses flags, asks Moonraker for the print state, and maps it to an exit code.
//!
//! The check writes only to stderr; stdout stays clean. `--help` and `--version` print to stdout.

use std::process::exit;
use std::time::Duration;

use layerlock::moonraker::{Client, State};

const DEFAULT_URL: &str = "http://localhost:7125";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

/// Exit code for errors: systemd's `ExecCondition=` treats 255 as a failure, not a skip.
const EXIT_ERROR: i32 = 255;

const USAGE: &str = "\
Usage: layerlock [flags]

Exits 0 when the printer is idle, 1 while it prints, 2 while it's paused, and 255 on errors.

Flags:
  --url <url>          Moonraker base URL (default: $MOONRAKER_URL or http://localhost:7125)
  --timeout <duration> HTTP request timeout, for example 5s or 500ms (default: 5s)
  --fail-open          Exit 0 on errors and unknown states instead of 255
  -v, --verbose        Write the printer state to stderr
  -h, --help           Show this help
  --version            Show the version
";

#[derive(Debug, PartialEq)]
struct Options {
    url: String,
    timeout: Duration,
    verbose: bool,
    fail_open: bool,
}

#[derive(Debug, PartialEq)]
enum Command {
    Check(Options),
    Help,
    Version,
}

/// Parses the arguments like Go's `flag` package did, so that existing unit files keep working:
/// `-flag` and `--flag` are the same, and a value goes in the next argument or after `=`.
fn parse_args(
    args: impl IntoIterator<Item = String>,
    env_url: Option<String>,
) -> Result<Command, String> {
    let mut opts = Options {
        url: env_url
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| DEFAULT_URL.to_string()),
        timeout: DEFAULT_TIMEOUT,
        verbose: false,
        fail_open: false,
    };

    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let Some(flag) = arg.strip_prefix("--").or_else(|| arg.strip_prefix('-')) else {
            return Err(format!("unexpected argument {arg:?}"));
        };
        let (name, inline) = match flag.split_once('=') {
            Some((name, value)) => (name, Some(value.to_string())),
            None => (flag, None),
        };
        let mut value = || {
            inline
                .clone()
                .or_else(|| args.next())
                .ok_or_else(|| format!("{arg} needs a value"))
        };
        match name {
            "url" => opts.url = value()?,
            "timeout" => {
                let v = value()?;
                opts.timeout =
                    parse_duration(&v).ok_or_else(|| format!("invalid --timeout {v:?}"))?;
            }
            "verbose" | "v" => opts.verbose = true,
            "fail-open" => opts.fail_open = true,
            "help" | "h" => return Ok(Command::Help),
            "version" => return Ok(Command::Version),
            _ => return Err(format!("unknown flag {arg}")),
        }
    }
    Ok(Command::Check(opts))
}

/// Parses a Go duration string such as `5s`, `500ms` or `1m30s`.
fn parse_duration(s: &str) -> Option<Duration> {
    let s = s.strip_prefix('+').unwrap_or(s);
    if s == "0" {
        return Some(Duration::ZERO);
    }
    if s.is_empty() {
        return None;
    }
    let mut rest = s;
    let mut secs = 0.0;
    while !rest.is_empty() {
        let num_len = rest
            .find(|c: char| !(c.is_ascii_digit() || c == '.'))
            .unwrap_or(rest.len());
        let num: f64 = rest[..num_len].parse().ok()?;
        rest = &rest[num_len..];
        let unit_len = rest
            .find(|c: char| c.is_ascii_digit() || c == '.')
            .unwrap_or(rest.len());
        let scale = match &rest[..unit_len] {
            "ns" => 1e-9,
            "us" | "µs" | "μs" => 1e-6,
            "ms" => 1e-3,
            "s" => 1.0,
            "m" => 60.0,
            "h" => 3600.0,
            _ => return None,
        };
        rest = &rest[unit_len..];
        secs += num * scale;
    }
    Duration::try_from_secs_f64(secs).ok()
}

fn main() {
    let opts = match parse_args(
        std::env::args().skip(1),
        std::env::var("MOONRAKER_URL").ok(),
    ) {
        Ok(Command::Check(opts)) => opts,
        Ok(Command::Help) => {
            print!("{USAGE}");
            exit(0)
        }
        Ok(Command::Version) => {
            println!("layerlock {}", env!("CARGO_PKG_VERSION"));
            exit(0)
        }
        Err(e) => {
            eprint!("layerlock: error: {e}\n\n{USAGE}");
            exit(EXIT_ERROR)
        }
    };

    let state = match Client::new(&opts.url, opts.timeout).get_state() {
        Ok(state) => state,
        Err(e) if opts.fail_open => {
            eprintln!("layerlock: warning: {e} (--fail-open set, allowing)");
            exit(0)
        }
        Err(e) => {
            eprintln!("layerlock: error: {e} (use --fail-open to allow)");
            exit(EXIT_ERROR)
        }
    };

    let (code, verdict) = match state {
        State::Printing => (1, "blocking"),
        State::Paused => (2, "blocking"),
        State::Standby | State::Complete => (0, "allowing"),
        State::Error | State::Other(_) if opts.fail_open => {
            eprintln!(
                "layerlock: warning: unexpected printer state \"{state}\" (--fail-open set, allowing)"
            );
            exit(0)
        }
        State::Error | State::Other(_) => {
            eprintln!(
                "layerlock: error: unexpected printer state \"{state}\" (use --fail-open to allow)"
            );
            exit(EXIT_ERROR)
        }
    };
    if opts.verbose {
        eprintln!("layerlock: printer state: {state} — {verdict}");
    }
    exit(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    fn check(a: &[&str]) -> Options {
        match parse_args(args(a), None) {
            Ok(Command::Check(opts)) => opts,
            other => panic!("expected options for {a:?}, got {other:?}"),
        }
    }

    #[test]
    fn defaults() {
        assert_eq!(
            check(&[]),
            Options {
                url: DEFAULT_URL.to_string(),
                timeout: DEFAULT_TIMEOUT,
                verbose: false,
                fail_open: false,
            }
        );
    }

    #[test]
    fn go_flag_spellings() {
        for a in [
            &["--url", "http://x"][..],
            &["-url", "http://x"],
            &["--url=http://x"],
            &["-url=http://x"],
        ] {
            assert_eq!(check(a).url, "http://x", "{a:?}");
        }
        assert!(check(&["-v"]).verbose);
        assert!(check(&["--verbose"]).verbose);
        assert!(check(&["-fail-open"]).fail_open);
        assert_eq!(
            check(&["--timeout", "250ms"]).timeout,
            Duration::from_millis(250)
        );
    }

    #[test]
    fn env_url_and_flag_precedence() {
        let env = || Some("http://env".to_string());
        let Ok(Command::Check(opts)) = parse_args(args(&[]), env()) else {
            panic!()
        };
        assert_eq!(opts.url, "http://env");
        let Ok(Command::Check(opts)) = parse_args(args(&["--url", "http://flag"]), env()) else {
            panic!()
        };
        assert_eq!(opts.url, "http://flag");
        let Ok(Command::Check(opts)) = parse_args(args(&[]), Some(String::new())) else {
            panic!()
        };
        assert_eq!(opts.url, DEFAULT_URL);
    }

    #[test]
    fn help_and_version() {
        assert_eq!(parse_args(args(&["-h"]), None), Ok(Command::Help));
        assert_eq!(parse_args(args(&["--help"]), None), Ok(Command::Help));
        assert_eq!(parse_args(args(&["--version"]), None), Ok(Command::Version));
    }

    #[test]
    fn usage_errors() {
        for a in [
            &["--nope"][..],
            &["--url"],
            &["--timeout", "5"],
            &["positional"],
        ] {
            assert!(parse_args(args(a), None).is_err(), "{a:?}");
        }
    }

    #[test]
    fn durations() {
        let ok = [
            ("0", Duration::ZERO),
            ("5s", Duration::from_secs(5)),
            ("500ms", Duration::from_millis(500)),
            ("1.5s", Duration::from_millis(1500)),
            ("1m30s", Duration::from_secs(90)),
            ("2h", Duration::from_secs(7200)),
            ("100us", Duration::from_micros(100)),
            ("100µs", Duration::from_micros(100)),
            ("+3s", Duration::from_secs(3)),
        ];
        for (s, want) in ok {
            let got = parse_duration(s).unwrap_or_else(|| panic!("{s} didn't parse"));
            assert!(
                got.abs_diff(want) < Duration::from_nanos(10),
                "{s}: {got:?}"
            );
        }
        for s in ["", "5", "s", "-5s", "5x", "1..5s", "5 s"] {
            assert_eq!(parse_duration(s), None, "{s}");
        }
    }
}
