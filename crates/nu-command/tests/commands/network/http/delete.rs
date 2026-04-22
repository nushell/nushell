use std::{thread, time::Duration};

use mockito::Server;
use nu_protocol::shell_error;
use nu_test_support::prelude::*;

#[test]
fn http_delete_is_success() -> Result {
    let mut server = Server::new();
    let _mock = server.mock("DELETE", "/").create();
    let code = format!("http delete {url}", url = server.url());
    test().run(code).expect_value_eq("")
}

#[test]
fn http_delete_is_success_pipeline() -> Result {
    let mut server = Server::new();
    let _mock = server.mock("DELETE", "/").create();
    let code = format!(r#""foo" | http delete {url}"#, url = server.url());
    test().run(code).expect_value_eq("")
}

#[test]
fn http_delete_failed_due_to_server_error() -> Result {
    let mut server = Server::new();
    let _mock = server.mock("DELETE", "/").with_status(400).create();
    let code = format!("http delete {url}", url = server.url());
    let err = test().run(code).expect_error()?;
    match err {
        ShellError::NetworkFailure { msg, .. } => {
            assert_contains("Bad request (400)", msg);
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn http_delete_follows_redirect() -> Result {
    let mut server = Server::new();

    let _mock = server.mock("GET", "/bar").with_body("bar").create();
    let _mock = server
        .mock("DELETE", "/foo")
        .with_status(301)
        .with_header("Location", "/bar")
        .create();

    let code = format!("http delete {url}/foo", url = server.url());
    test().run(code).expect_value_eq("bar")
}

#[test]
fn http_delete_redirect_mode_manual() -> Result {
    let mut server = Server::new();

    let _mock = server
        .mock("DELETE", "/foo")
        .with_status(301)
        .with_body("foo")
        .with_header("Location", "/bar")
        .create();

    let code = format!(
        "http delete --redirect-mode manual {url}/foo",
        url = server.url()
    );

    test().run(code).expect_value_eq("foo")
}

#[test]
fn http_delete_redirect_mode_error() -> Result {
    let mut server = Server::new();

    let _mock = server
        .mock("DELETE", "/foo")
        .with_status(301)
        .with_body("foo")
        .with_header("Location", "/bar")
        .create();

    let code = format!(
        "http delete --redirect-mode error {url}/foo",
        url = server.url()
    );

    let err = test().run(code).expect_shell_error()?;
    match err {
        ShellError::NetworkFailure { msg, .. } => {
            assert_eq!(
                msg,
                "Redirect encountered when redirect handling mode was 'error' (301 Moved Permanently)"
            );
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn http_delete_timeout() -> Result {
    let mut server = Server::new();

    let _mock = server
        .mock("DELETE", "/")
        .with_chunked_body(|w| {
            thread::sleep(Duration::from_secs(10));
            w.write_all(b"Delayed response!")
        })
        .create();

    let code = format!("http delete --max-time 100ms {url}", url = server.url());

    let err = test().run(code).expect_io_error()?;
    assert!(matches!(
        err.kind,
        shell_error::io::ErrorKind::Std(std::io::ErrorKind::TimedOut, ..)
    ));

    Ok(())
}
