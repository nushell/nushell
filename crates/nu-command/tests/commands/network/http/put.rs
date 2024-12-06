use std::{thread, time::Duration};

use mockito::Server;
use nu_test_support::{nu, pipeline};

#[test]
fn http_put_is_success() {
    let mut server = Server::new();

    let _mock = server.mock("PUT", "/").match_body("foo").create();

    let actual = nu!(pipeline(
        format!(
            r#"
        http put {url} "foo"
        "#,
            url = server.url()
        )
        .as_str()
    ));

    assert!(actual.out.is_empty())
}

#[test]
fn http_put_is_success_pipeline() {
    let mut server = Server::new();

    let _mock = server.mock("PUT", "/").match_body("foo").create();

    let actual = nu!(pipeline(
        format!(
            r#"
        "foo" | http put {url} 
        "#,
            url = server.url()
        )
        .as_str()
    ));

    assert!(actual.out.is_empty())
}

#[test]
fn http_put_failed_due_to_server_error() {
    let mut server = Server::new();

    let _mock = server.mock("PUT", "/").with_status(400).create();

    let actual = nu!(pipeline(
        format!(
            r#"
        http put {url} "body"
        "#,
            url = server.url()
        )
        .as_str()
    ));

    assert!(actual.err.contains("Bad request (400)"))
}

#[test]
fn http_put_failed_due_to_missing_body() {
    let mut server = Server::new();

    let _mock = server.mock("PUT", "/").create();

    let actual = nu!(pipeline(
        format!(
            r#"
        http put {url}
        "#,
            url = server.url()
        )
        .as_str()
    ));

    assert!(actual
        .err
        .contains("Data must be provided either through pipeline or positional argument"))
}

#[test]
fn http_put_failed_due_to_unexpected_body() {
    let mut server = Server::new();

    let _mock = server.mock("PUT", "/").match_body("foo").create();

    let actual = nu!(pipeline(
        format!(
            r#"
        http put {url} "bar"
        "#,
            url = server.url()
        )
        .as_str()
    ));

    assert!(actual.err.contains("Cannot make request"))
}

#[test]
fn http_put_follows_redirect() {
    let mut server = Server::new();

    let _mock = server.mock("GET", "/bar").with_body("bar").create();
    let _mock = server
        .mock("PUT", "/foo")
        .with_status(301)
        .with_header("Location", "/bar")
        .create();

    let actual = nu!(pipeline(
        format!("http put {url}/foo putbody", url = server.url()).as_str()
    ));

    assert_eq!(&actual.out, "bar");
}

#[test]
fn http_put_redirect_mode_manual() {
    let mut server = Server::new();

    let _mock = server
        .mock("PUT", "/foo")
        .with_status(301)
        .with_body("foo")
        .with_header("Location", "/bar")
        .create();

    let actual = nu!(pipeline(
        format!(
            "http put --redirect-mode manual {url}/foo putbody",
            url = server.url()
        )
        .as_str()
    ));

    assert_eq!(&actual.out, "foo");
}

#[test]
fn http_put_redirect_mode_error() {
    let mut server = Server::new();

    let _mock = server
        .mock("PUT", "/foo")
        .with_status(301)
        .with_body("foo")
        .with_header("Location", "/bar")
        .create();

    let actual = nu!(pipeline(
        format!(
            "http put --redirect-mode error {url}/foo putbody",
            url = server.url()
        )
        .as_str()
    ));

    assert!(&actual.err.contains("nu::shell::network_failure"));
    assert!(&actual.err.contains(
        "Redirect encountered when redirect handling mode was 'error' (301 Moved Permanently)"
    ));
}

#[test]
fn http_put_timeout() {
    let mut server = Server::new();
    let _mock = server
        .mock("PUT", "/")
        .with_chunked_body(|w| {
            thread::sleep(Duration::from_secs(10));
            w.write_all(b"Delayed response!")
        })
        .create();

    let actual = nu!(pipeline(
        format!(
            "http put --max-time 100ms {url} putbody",
            url = server.url()
        )
        .as_str()
    ));

    assert!(&actual.err.contains("nu::shell::network_failure"));

    #[cfg(not(target_os = "windows"))]
    assert!(&actual.err.contains("timed out reading response"));
    #[cfg(target_os = "windows")]
    assert!(&actual
        .err
        .contains("did not properly respond after a period of time"));
}
