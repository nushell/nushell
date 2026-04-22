use std::{thread, time::Duration};

use mockito::Server;
use nu_protocol::shell_error;
use nu_test_support::prelude::*;

#[test]
fn http_options_default_shows_response_headers() -> Result {
    let mut server = Server::new();

    let _mock = server
        .mock("OPTIONS", "/")
        .with_header("Allow", "OPTIONS, GET")
        .create();

    let code = format!(
        "http options {url} | where name == allow | get value.0",
        url = server.url()
    );

    test().run(code).expect_value_eq("OPTIONS, GET")
}

#[test]
fn http_options_full_response_includes_response_headers() -> Result {
    let mut server = Server::new();

    let _mock = server
        .mock("OPTIONS", "/")
        .with_header("Allow", "OPTIONS, GET")
        .create();

    let code = format!(
        "http options --full {url} | get headers.response | length",
        url = server.url()
    );

    let outcome: i64 = test().run(code)?;
    assert!(outcome > 0);
    Ok(())
}

#[test]
fn http_options_with_allow_errors() -> Result {
    let mut server = Server::new();

    let _mock = server
        .mock("OPTIONS", "/")
        .with_status(400)
        .with_header("x-error-header", "present")
        .create();

    let code = format!(
        "http options -e {url} | where name == x-error-header | get value.0",
        url = server.url()
    );

    test().run(code).expect_value_eq("present")
}

#[test]
fn http_options_failed_due_to_server_error() -> Result {
    let mut server = Server::new();

    let _mock = server.mock("OPTIONS", "/").with_status(400).create();

    let code = format!("http options {url}", url = server.url());
    let err = test().run(code).expect_shell_error()?;

    match err {
        ShellError::NetworkFailure { msg, .. } => {
            assert_contains("Bad request (400)", msg);
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn http_options_timeout() -> Result {
    let mut server = Server::new();
    let _mock = server
        .mock("OPTIONS", "/")
        .with_chunked_body(|w| {
            thread::sleep(Duration::from_secs(10));
            w.write_all(b"Delayed response!")
        })
        .create();

    let code = format!("http options --max-time 100ms {url}", url = server.url());
    let err = test().run(code).expect_io_error()?;
    assert!(matches!(
        err.kind,
        shell_error::io::ErrorKind::Std(std::io::ErrorKind::TimedOut, ..)
    ));

    Ok(())
}
