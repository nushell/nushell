use mockito::Server;
use nu_test_support::{nu, pipeline};
use reqwest::StatusCode;

#[test]
fn http_patch_is_success() {
    let mut server = Server::new();

    let _mock = server.mock("PATCH", "/").match_body("foo").create();

    let actual = nu!(pipeline(
        format!(
            r#"
        http patch {url} "foo"
        "#,
            url = server.url()
        )
        .as_str()
    ));

    assert!(actual.out.is_empty())
}

#[test]
fn http_patch_failed_due_to_server_error() {
    let mut server = Server::new();

    let _mock = server
        .mock("PATCH", "/")
        .with_status(StatusCode::BAD_REQUEST.as_u16() as usize)
        .create();

    let actual = nu!(pipeline(
        format!(
            r#"
        http patch {url} "body"
        "#,
            url = server.url()
        )
        .as_str()
    ));

    assert!(actual.err.contains("Bad request (400)"))
}

#[test]
fn http_patch_failed_due_to_missing_body() {
    let mut server = Server::new();

    let _mock = server.mock("PATCH", "/").create();

    let actual = nu!(pipeline(
        format!(
            r#"
        http patch {url}
        "#,
            url = server.url()
        )
        .as_str()
    ));

    assert!(actual.err.contains("Usage: http patch"))
}

#[test]
fn http_patch_failed_due_to_unexpected_body() {
    let mut server = Server::new();

    let _mock = server.mock("PATCH", "/").match_body("foo").create();

    let actual = nu!(pipeline(
        format!(
            r#"
        http patch {url} "bar"
        "#,
            url = server.url()
        )
        .as_str()
    ));

    assert!(actual.err.contains("Cannot make request"))
}
