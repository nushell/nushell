use mockito::Server;
use nu_test_support::{nu, pipeline};

#[test]
fn http_get_is_success() {
    let mut server = Server::new();

    let _mock = server.mock("GET", "/").with_body("foo").create();

    let actual = nu!(pipeline(
        format!(
            r#"
        http get {url}
        "#,
            url = server.url()
        )
        .as_str()
    ));

    assert_eq!(actual.out, "foo")
}

#[test]
fn http_get_failed_due_to_server_error() {
    let mut server = Server::new();

    let _mock = server.mock("GET", "/").with_status(400).create();

    let actual = nu!(pipeline(
        format!(
            r#"
        http get {url}
        "#,
            url = server.url()
        )
        .as_str()
    ));

    assert!(actual.err.contains("Bad request (400)"))
}
