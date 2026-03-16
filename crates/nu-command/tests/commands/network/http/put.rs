use std::{thread, time::Duration};

use mockito::Server;
use nu_protocol::shell_error;
use nu_test_support::prelude::*;

#[test]
fn http_put_is_success() -> Result {
    let mut server = Server::new();
    let _mock = server.mock("PUT", "/").match_body("foo").create();
    let code = format!(r#"http put {url} "foo""#, url = server.url());
    test().run(code).expect_value_eq("")
}

#[test]
fn http_put_is_success_pipeline() -> Result {
    let mut server = Server::new();
    let _mock = server.mock("PUT", "/").match_body("foo").create();
    let code = format!(r#""foo" | http put {url} "#, url = server.url());
    test().run(code).expect_value_eq("")
}

#[test]
fn http_put_failed_due_to_server_error() -> Result {
    let mut server = Server::new();

    let _mock = server.mock("PUT", "/").with_status(400).create();

    let code = format!(r#"http put {url} "body""#, url = server.url());
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
fn http_put_failed_due_to_missing_body() -> Result {
    let mut server = Server::new();

    let _mock = server.mock("PUT", "/").create();

    let code = format!("http put {url}", url = server.url());
    let err = test().run(code).expect_shell_error()?.generic_error()?;
    assert_eq!(
        err,
        "Data must be provided either through pipeline or positional argument"
    );
    Ok(())
}

#[test]
fn http_put_failed_due_to_unexpected_body() -> Result {
    let mut server = Server::new();

    let _mock = server.mock("PUT", "/").match_body("foo").create();

    let code = format!(r#"http put {url} "bar""#, url = server.url());
    let err = test().run(code).expect_shell_error()?;

    match err {
        ShellError::NetworkFailure { msg, .. } => {
            assert_contains("Cannot make request", msg);
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn http_put_follows_redirect() -> Result {
    let mut server = Server::new();

    let _mock = server.mock("GET", "/bar").with_body("bar").create();
    let _mock = server
        .mock("PUT", "/foo")
        .with_status(301)
        .with_header("Location", "/bar")
        .create();

    let code = format!("http put {url}/foo putbody", url = server.url());
    test().run(code).expect_value_eq("bar")
}

#[test]
fn http_put_redirect_mode_manual() -> Result {
    let mut server = Server::new();

    let _mock = server
        .mock("PUT", "/foo")
        .with_status(301)
        .with_body("foo")
        .with_header("Location", "/bar")
        .create();

    let code = format!(
        "http put --redirect-mode manual {url}/foo putbody",
        url = server.url()
    );
    test().run(code).expect_value_eq("foo")
}

#[test]
fn http_put_redirect_mode_error() -> Result {
    let mut server = Server::new();

    let _mock = server
        .mock("PUT", "/foo")
        .with_status(301)
        .with_body("foo")
        .with_header("Location", "/bar")
        .create();

    let code = format!(
        "http put --redirect-mode error {url}/foo putbody",
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
fn http_put_timeout() -> Result {
    let mut server = Server::new();
    let _mock = server
        .mock("PUT", "/")
        .with_chunked_body(|w| {
            thread::sleep(Duration::from_secs(10));
            w.write_all(b"Delayed response!")
        })
        .create();

    let code = format!(
        "http put --max-time 100ms {url} putbody",
        url = server.url()
    );
    let err = test().run(code).expect_io_error()?;
    assert!(matches!(
        err.kind,
        shell_error::io::ErrorKind::Std(std::io::ErrorKind::TimedOut, ..)
    ));
    Ok(())
}
