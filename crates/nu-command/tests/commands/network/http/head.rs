use mockito::Server;
use nu_test_support::nu;

#[test]
fn http_head_is_success() {
    let mut server = Server::new();

    let _mock = server.mock("HEAD", "/").with_header("foo", "bar").create();

    let actual = nu!(format!(r#"http head {url}"#, url = server.url()));

    assert!(actual.out.contains("foo"));
    assert!(actual.out.contains("bar"));
}

#[test]
fn http_head_failed_due_to_server_error() {
    let mut server = Server::new();

    let _mock = server.mock("HEAD", "/").with_status(400).create();

    let actual = nu!(format!(r#"http head {url}"#, url = server.url()));
    assert!(
        actual.err.contains("Bad request (400)"),
        "Unexpected error: {:?}",
        actual.err
    )
}

#[test]
fn http_head_with_accept_errors() {
    let mut server = Server::new();

    let _mock = server
        .mock("HEAD", "/")
        .with_status(400)
        .with_header("x-error-header", "present")
        .create();

    let actual = nu!(format!(r#"http head -e {url}"#, url = server.url()));

    // When allowing errors, the command should not fail, and headers should still be available.
    assert!(
        actual.err.is_empty(),
        "Expected no error, got: {:?}",
        actual.err
    );
    assert!(actual.out.contains("x-error-header"));
}

#[test]
fn http_head_full_response_includes_status_and_headers() {
    let mut server = Server::new();

    let _mock = server
        .mock("HEAD", "/")
        .with_status(204)
        .with_header("x-custom-header", "value")
        .create();

    let actual = nu!(format!(
        r#"
            http head --full {url}
            | to json
        "#,
        url = server.url()
    ));

    let output: serde_json::Value = serde_json::from_str(&actual.out).unwrap();

    assert_eq!(output["status"], 204);

    let headers = &output["headers"]["response"];
    assert!(
        headers
            .as_array()
            .unwrap()
            .iter()
            .any(|h| h["name"] == "x-custom-header" && h["value"] == "value")
    );
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

    let actual = nu!(format!(
        "http head {url}/foo | (where name == bar).0.value",
        url = server.url()
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

    let actual = nu!(format!(
        "http head --redirect-mode manual {url}/foo | (where name == location).0.value",
        url = server.url()
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

    let actual = nu!(format!(
        "http head --redirect-mode error {url}/foo",
        url = server.url()
    ));

    assert!(&actual.err.contains("nu::shell::network_failure"));
    assert!(&actual.err.contains(
        "Redirect encountered when redirect handling mode was 'error' (301 Moved Permanently)"
    ));
}
