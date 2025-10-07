use std::{thread, time::Duration};

use mockito::Server;
use nu_test_support::nu;

#[test]
fn http_options_is_success() {
    let mut server = Server::new();

    let _mock = server
        .mock("OPTIONS", "/")
        .with_header("Allow", "OPTIONS, GET")
        .create();

    let actual = nu!(format!(r#"http options {url}"#, url = server.url()));

    assert!(!actual.out.is_empty())
}

#[test]
fn http_options_failed_due_to_server_error() {
    let mut server = Server::new();

    let _mock = server.mock("OPTIONS", "/").with_status(400).create();

    let actual = nu!(format!(r#"http options {url}"#, url = server.url()));

    assert!(actual.err.contains("Bad request (400)"))
}

#[test]
fn http_options_timeout() {
    let mut server = Server::new();
    let _mock = server
        .mock("OPTIONS", "/")
        .with_chunked_body(|w| {
            thread::sleep(Duration::from_secs(10));
            w.write_all(b"Delayed response!")
        })
        .create();

    let actual = nu!(format!(
        "http options --max-time 100ms {url}",
        url = server.url()
    ));

    assert!(&actual.err.contains("nu::shell::io::timed_out"));
    assert!(&actual.err.contains("Timed out"));
}
