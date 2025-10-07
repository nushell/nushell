use std::{thread, time::Duration};

use mockito::Server;
use nu_test_support::nu;

#[test]
fn http_patch_is_success() {
    let mut server = Server::new();

    let _mock = server.mock("PATCH", "/").match_body("foo").create();

    let actual = nu!(format!(r#"http patch {url} "foo""#, url = server.url()));

    assert!(actual.out.is_empty())
}

#[test]
fn http_patch_is_success_pipeline() {
    let mut server = Server::new();

    let _mock = server.mock("PATCH", "/").match_body("foo").create();

    let actual = nu!(format!(r#""foo" | http patch {url}"#, url = server.url()));

    assert!(actual.out.is_empty())
}

#[test]
fn http_patch_failed_due_to_server_error() {
    let mut server = Server::new();

    let _mock = server.mock("PATCH", "/").with_status(400).create();

    let actual = nu!(format!(r#"http patch {url} "body""#, url = server.url()));

    assert!(actual.err.contains("Bad request (400)"))
}

#[test]
fn http_patch_failed_due_to_missing_body() {
    let mut server = Server::new();

    let _mock = server.mock("PATCH", "/").create();

    let actual = nu!(format!(r#"http patch {url}"#, url = server.url()));

    assert!(
        actual
            .err
            .contains("Data must be provided either through pipeline or positional argument")
    )
}

#[test]
fn http_patch_failed_due_to_unexpected_body() {
    let mut server = Server::new();

    let _mock = server.mock("PATCH", "/").match_body("foo").create();

    let actual = nu!(format!(r#"http patch {url} "bar""#, url = server.url()));

    assert!(actual.err.contains("Cannot make request"))
}

#[test]
fn http_patch_follows_redirect() {
    let mut server = Server::new();

    let _mock = server.mock("GET", "/bar").with_body("bar").create();
    let _mock = server
        .mock("PATCH", "/foo")
        .with_status(301)
        .with_header("Location", "/bar")
        .create();

    let actual = nu!(format!(
        "http patch {url}/foo patchbody",
        url = server.url()
    ));

    assert_eq!(&actual.out, "bar");
}

#[test]
fn http_patch_redirect_mode_manual() {
    let mut server = Server::new();

    let _mock = server
        .mock("PATCH", "/foo")
        .with_status(301)
        .with_body("foo")
        .with_header("Location", "/bar")
        .create();

    let actual = nu!(format!(
        "http patch --redirect-mode manual {url}/foo patchbody",
        url = server.url()
    ));

    assert_eq!(&actual.out, "foo");
}

#[test]
fn http_patch_redirect_mode_error() {
    let mut server = Server::new();

    let _mock = server
        .mock("PATCH", "/foo")
        .with_status(301)
        .with_body("foo")
        .with_header("Location", "/bar")
        .create();

    let actual = nu!(format!(
        "http patch --redirect-mode error {url}/foo patchbody",
        url = server.url()
    ));

    assert!(&actual.err.contains("nu::shell::network_failure"));
    assert!(&actual.err.contains(
        "Redirect encountered when redirect handling mode was 'error' (301 Moved Permanently)"
    ));
}

#[test]
fn http_patch_timeout() {
    let mut server = Server::new();
    let _mock = server
        .mock("PATCH", "/")
        .with_chunked_body(|w| {
            thread::sleep(Duration::from_secs(10));
            w.write_all(b"Delayed response!")
        })
        .create();

    let actual = nu!(format!(
        "http patch --max-time 100ms {url} patchbody",
        url = server.url()
    ));

    assert!(&actual.err.contains("nu::shell::io::timed_out"));
    assert!(&actual.err.contains("Timed out"));
}
