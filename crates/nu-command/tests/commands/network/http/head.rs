use mockito::Server;
use nu_test_support::{nu, pipeline};

#[test]
fn http_head_is_success() {
    let mut server = Server::new();

    let _mock = server.mock("HEAD", "/").with_header("foo", "bar").create();

    let actual = nu!(pipeline(
        format!(
            r#"
        http head {url}
        "#,
            url = server.url()
        )
        .as_str()
    ));

    assert!(actual.out.contains("foo"));
    assert!(actual.out.contains("bar"));
}

#[test]
fn http_head_failed_due_to_server_error() {
    let mut server = Server::new();

    let _mock = server.mock("HEAD", "/").with_status(400).create();

    let actual = nu!(pipeline(
        format!(
            r#"
        http head {url}
        "#,
            url = server.url()
        )
        .as_str()
    ));
    assert!(
        actual.err.contains("Bad request (400)"),
        "Unexpected error: {:?}",
        actual.err
    )
}

#[test]
fn http_head_follows_redirect() {
    let mut server = Server::new();

    let _mock = server
        .mock("HEAD", "/bar")
        .with_header("bar", "bar")
        .create();
    let _mock = server
        .mock("HEAD", "/foo")
        .with_status(301)
        .with_header("Location", "/bar")
        .create();

    let actual = nu!(pipeline(
        format!(
            "http head {url}/foo | (where name == bar).0.value",
            url = server.url()
        )
        .as_str()
    ));

    assert_eq!(&actual.out, "bar");
}

#[test]
fn http_head_redirect_mode_manual() {
    let mut server = Server::new();

    let _mock = server
        .mock("HEAD", "/foo")
        .with_status(301)
        .with_body("foo")
        .with_header("Location", "/bar")
        .create();

    let actual = nu!(pipeline(
        format!(
            "http head --redirect-mode manual {url}/foo | (where name == location).0.value",
            url = server.url()
        )
        .as_str()
    ));

    assert_eq!(&actual.out, "/bar");
}

#[test]
fn http_head_redirect_mode_error() {
    let mut server = Server::new();

    let _mock = server
        .mock("HEAD", "/foo")
        .with_status(301)
        .with_body("foo")
        .with_header("Location", "/bar")
        .create();

    let actual = nu!(pipeline(
        format!(
            "http head --redirect-mode error {url}/foo",
            url = server.url()
        )
        .as_str()
    ));

    assert!(&actual.err.contains("nu::shell::network_failure"));
    assert!(&actual.err.contains(
        "Redirect encountered when redirect handling mode was 'error' (301 Moved Permanently)"
    ));
}
