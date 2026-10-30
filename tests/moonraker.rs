mod common;

use std::time::{Duration, Instant};

use layerlock::moonraker::{Client, Error, State};

fn client(url: &str) -> Client {
    Client::new(url, Duration::from_secs(5))
}

#[test]
fn all_states() {
    let cases = [
        ("standby", State::Standby),
        ("printing", State::Printing),
        ("paused", State::Paused),
        ("error", State::Error),
        ("complete", State::Complete),
        (
            "unknown_future_state",
            State::Other("unknown_future_state".to_string()),
        ),
    ];
    for (moonraker_state, want) in cases {
        let url = common::moonraker(moonraker_state);
        let got = client(&url).get_state().unwrap();
        assert_eq!(got, want, "{moonraker_state}");
        assert_eq!(got.to_string(), moonraker_state);
    }
}

#[test]
fn trailing_slash_in_url() {
    let url = common::moonraker("printing");
    assert_eq!(
        client(&format!("{url}/")).get_state().unwrap(),
        State::Printing
    );
}

#[test]
fn unreachable() {
    let err = client(&common::unreachable()).get_state().unwrap_err();
    assert!(matches!(err, Error::Unreachable(_)), "{err:?}");
}

#[test]
fn non_ok_status() {
    let url = common::serve(404, "not found".to_string());
    let err = client(&url).get_state().unwrap_err();
    assert!(matches!(err, Error::Status(404)), "{err:?}");
    assert_eq!(err.to_string(), "moonraker returned HTTP 404");
}

#[test]
fn malformed_json() {
    for body in ["not json", r#"{"result":{}}"#] {
        let url = common::serve(200, body.to_string());
        let err = client(&url).get_state().unwrap_err();
        assert!(matches!(err, Error::Decode(_)), "{body}: {err:?}");
    }
}

#[test]
fn timeout() {
    let url = common::silent();
    let start = Instant::now();
    let err = Client::new(&url, Duration::from_millis(200))
        .get_state()
        .unwrap_err();
    assert!(matches!(err, Error::Unreachable(_)), "{err:?}");
    assert!(
        start.elapsed() < Duration::from_secs(2),
        "{:?}",
        start.elapsed()
    );
}
