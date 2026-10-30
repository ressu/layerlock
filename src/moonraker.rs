//! HTTP client for the Moonraker API.
//!
//! Errors are returned only for network, HTTP and decode failures. An unknown state comes back as
//! [`State::Other`], not as an error: the exit-code policy lives in `main`.

use std::fmt;
use std::time::Duration;

use serde::Deserialize;

/// The `print_stats.state` value from Moonraker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    Standby,
    Printing,
    Paused,
    Error,
    Complete,
    /// A state this version doesn't know, as Moonraker sent it.
    Other(String),
}

impl From<String> for State {
    fn from(s: String) -> Self {
        match s.as_str() {
            "standby" => State::Standby,
            "printing" => State::Printing,
            "paused" => State::Paused,
            "error" => State::Error,
            "complete" => State::Complete,
            _ => State::Other(s),
        }
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            State::Standby => "standby",
            State::Printing => "printing",
            State::Paused => "paused",
            State::Error => "error",
            State::Complete => "complete",
            State::Other(s) => s,
        })
    }
}

/// Why [`Client::get_state`] couldn't get a state.
#[derive(Debug)]
pub enum Error {
    /// Connection failed or timed out.
    Unreachable(ureq::Error),
    /// Moonraker answered with a status other than 200.
    Status(u16),
    /// The response wasn't the JSON we expect.
    Decode(ureq::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Unreachable(e) => write!(f, "moonraker unreachable: {e}"),
            Error::Status(code) => write!(f, "moonraker returned HTTP {code}"),
            Error::Decode(e) => write!(f, "decode response: {e}"),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Deserialize)]
struct Response {
    result: QueryResult,
}

#[derive(Deserialize)]
struct QueryResult {
    status: Status,
}

#[derive(Deserialize)]
struct Status {
    print_stats: PrintStats,
}

#[derive(Deserialize)]
struct PrintStats {
    state: String,
}

/// Queries a Moonraker instance.
pub struct Client {
    base_url: String,
    agent: ureq::Agent,
}

impl Client {
    /// Creates a client for `base_url` (for example `http://localhost:7125`). `timeout` limits the
    /// whole request, from connecting to reading the body.
    pub fn new(base_url: &str, timeout: Duration) -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(timeout))
            // Moonraker is on the local network: never send the request to an HTTP proxy.
            .proxy(None)
            .build()
            .into();
        Client {
            base_url: base_url.trim_end_matches('/').to_string(),
            agent,
        }
    }

    /// Queries Moonraker and returns the current print state.
    pub fn get_state(&self) -> Result<State, Error> {
        let url = format!("{}/printer/objects/query?print_stats", self.base_url);
        let mut resp = self.agent.get(&url).call().map_err(|e| match e {
            ureq::Error::StatusCode(code) => Error::Status(code),
            e => Error::Unreachable(e),
        })?;
        let r: Response = resp.body_mut().read_json().map_err(Error::Decode)?;
        Ok(r.result.status.print_stats.state.into())
    }
}
