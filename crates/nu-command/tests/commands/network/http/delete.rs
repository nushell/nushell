use std::{thread, time::Duration};

use mockito::Server;
use nu_test_support::{nu, pipeline};

#[test]
fn http_delete_is_success() {
    let mut server = Server::new();

    let _mock = server.mock("DELETE", "/").create();

    let actual = nu!(pipeline(
        format!(
            r#"
        http delete {url}
        "#,
            url = server.url()
        )
        .as_str()
    ));

    assert!(actual.out.is_empty())
}

#[test]
fn http_delete_is_success_pipeline() {
    let mut server = Server::new();

    let _mock = server.mock("DELETE", "/").create();

    let actual = nu!(pipeline(
        format!(
            r#"
        "foo" | http delete {url}
        "#,
            url = server.url()
        )
        .as_str()
    ));

    assert!(actual.out.is_empty())
}

#[test]
fn http_delete_failed_due_to_server_error() {
    let mut server = Server::new();

    let _mock = server.mock("DELETE", "/").with_status(400).create();

    let actual = nu!(pipeline(
        format!(
            r#"
        http delete {url}
        "#,
            url = server.url()
        )
        .as_str()
    ));

    assert!(
        actual.err.contains("Bad request (400)"),
        "unexpected error: {:?}",
        actual.err
    )
}

#[test]
fn http_delete_follows_redirect() {
    let mut server = Server::new();

    let _mock = server.mock("GET", "/bar").with_body("bar").create();
    let _mock = server
        .mock("DELETE", "/foo")
        .with_status(301)
        .with_header("Location", "/bar")
        .create();

    let actual = nu!(pipeline(
        format!("http delete {url}/foo", url = server.url()).as_str()
    ));

    assert_eq!(&actual.out, "bar");
}

#[test]
fn http_delete_redirect_mode_manual() {
    let mut server = Server::new();

    let _mock = server
        .mock("DELETE", "/foo")
        .with_status(301)
        .with_body("foo")
        .with_header("Location", "/bar")
        .create();

    let actual = nu!(pipeline(
        format!(
            "http delete --redirect-mode manual {url}/foo",
            url = server.url()
        )
        .as_str()
    ));

    assert_eq!(&actual.out, "foo");
}

#[test]
fn http_delete_redirect_mode_error() {
    let mut server = Server::new();

    let _mock = server
        .mock("DELETE", "/foo")
        .with_status(301)
        .with_body("foo")
        .with_header("Location", "/bar")
        .create();

    let actual = nu!(pipeline(
        format!(
            "http delete --redirect-mode error {url}/foo",
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
fn http_delete_timeout() {
    let mut server = Server::new();
    let _mock = server
        .mock("DELETE", "/")
        .with_chunked_body(|w| {
            thread::sleep(Duration::from_secs(10));
            w.write_all(b"Delayed response!")
        })
        .create();

    let actual = nu!(pipeline(
        format!("http delete --max-time 100ms {url}", url = server.url()).as_str()
    ));

    assert!(
        &actual.err.contains("nu::shell::io::timed_out"),
        "unexpected error : {:?}",
        actual.err
    );
    assert!(
        &actual.err.contains("Timed out"),
        "unexpected error : {:?}",
        actual.err
    );
}
