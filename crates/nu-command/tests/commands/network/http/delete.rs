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

    assert!(actual.err.contains("Bad request (400)"))
}
