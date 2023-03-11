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

    assert!(actual.err.contains("Bad request (400)"))
}
