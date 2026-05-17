use mockito::Server;
use nu_test_support::prelude::*;

#[test]
fn http_head_is_success() -> Result {
    let mut server = Server::new();

    let _mock = server.mock("HEAD", "/").with_header("foo", "bar").create();

    let code = format!(
        "http head {url} | where name == foo | get value.0",
        url = server.url()
    );
    test().run(code).expect_value_eq("bar")
}

#[test]
fn http_head_failed_due_to_server_error() -> Result {
    let mut server = Server::new();

    let _mock = server.mock("HEAD", "/").with_status(400).create();

    let code = format!("http head {url}", url = server.url());
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
fn http_head_with_accept_errors() -> Result {
    let mut server = Server::new();

    let _mock = server
        .mock("HEAD", "/")
        .with_status(400)
        .with_header("x-error-header", "present")
        .create();

    let code = format!(
        "http head -e {url} | where name == x-error-header | get value.0",
        url = server.url()
    );

    // When allowing errors, the command should not fail, and headers should still be available.
    test().run(code).expect_value_eq("present")
}

#[test]
fn http_head_full_response_includes_status_and_headers() -> Result {
    let mut server = Server::new();

    let _mock = server
        .mock("HEAD", "/")
        .with_status(204)
        .with_header("x-custom-header", "value")
        .create();

    let code = format!(
        "
            http head --full {url}
            | to json
        ",
        url = server.url()
    );

    let outcome: String = test().run(code)?;
    let output: serde_json::Value =
        serde_json::from_str(&outcome).expect("full response should be valid JSON");

    assert_eq!(output["status"], 204);

    let headers = &output["headers"]["response"];
    assert!(
        headers
            .as_array()
            .expect("response headers should be an array")
            .iter()
            .any(|h| h["name"] == "x-custom-header" && h["value"] == "value")
    );

    Ok(())
}

#[test]
fn http_head_follows_redirect() -> Result {
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

    let code = format!(
        "http head {url}/foo | (where name == bar).0.value",
        url = server.url()
    );

    test().run(code).expect_value_eq("bar")
}

#[test]
fn http_head_redirect_mode_manual() -> Result {
    let mut server = Server::new();

    let _mock = server
        .mock("HEAD", "/foo")
        .with_status(301)
        .with_body("foo")
        .with_header("Location", "/bar")
        .create();

    let code = format!(
        "http head --redirect-mode manual {url}/foo | (where name == location).0.value",
        url = server.url()
    );

    test().run(code).expect_value_eq("/bar")
}

#[test]
fn http_head_redirect_mode_error() -> Result {
    let mut server = Server::new();

    let _mock = server
        .mock("HEAD", "/foo")
        .with_status(301)
        .with_body("foo")
        .with_header("Location", "/bar")
        .create();

    let code = format!(
        "http head --redirect-mode error {url}/foo",
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
