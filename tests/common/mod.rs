//! A fake Moonraker on a free local port. Each server runs on its own thread for the rest of the test
//! run, so tests can run in parallel.

#![allow(dead_code)] // each test file uses a different part

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

const QUERY: &str = "/printer/objects/query?print_stats";

/// Starts a fake Moonraker that reports `state`, and returns its base URL.
pub fn moonraker(state: &str) -> String {
    let body = format!(
        r#"{{"result":{{"eventtime":1.0,"status":{{"print_stats":{{"state":"{state}","filename":"x.gcode"}}}}}}}}"#
    );
    serve(200, body)
}

/// Starts a server that answers the print state query with `status` and `body`, and anything else with
/// 404. Returns its base URL.
pub fn serve(status: u16, body: String) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            respond(stream, status, &body);
        }
    });
    url
}

/// Starts a server that accepts connections and never answers. Returns its base URL.
pub fn silent() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    thread::spawn(move || {
        let mut open = Vec::new();
        for stream in listener.incoming().flatten() {
            open.push(stream);
        }
    });
    url
}

/// A base URL where nothing listens.
pub fn unreachable() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    format!("http://{}", listener.local_addr().unwrap())
}

fn respond(stream: TcpStream, status: u16, body: &str) {
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
    let mut reader = BufReader::new(&stream);
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() {
        return;
    }
    // Read the headers up to the empty line.
    let mut line = String::new();
    while reader.read_line(&mut line).is_ok_and(|n| n > 0) && line != "\r\n" {
        line.clear();
    }

    let path = request_line.split_whitespace().nth(1).unwrap_or("");
    let (status, body) = if path == QUERY {
        (status, body)
    } else {
        (404, "not found")
    };
    let response = format!(
        "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    (&stream).write_all(response.as_bytes()).ok();
}
