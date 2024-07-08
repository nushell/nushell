use mockito::Server;
use nu_test_support::{nu, pipeline};

#[test]
fn http_post_is_success() {
    let mut server = Server::new();

    let _mock = server.mock("POST", "/").match_body("foo").create();

    let actual = nu!(pipeline(
        format!(
            r#"
        http post {url} "foo"
        "#,
            url = server.url()
        )
        .as_str()
    ));

    assert!(actual.out.is_empty())
}
#[test]
fn http_post_is_success_pipeline() {
    let mut server = Server::new();

    let _mock = server.mock("POST", "/").match_body("foo").create();

    let actual = nu!(pipeline(
        format!(
            r#"
        "foo" | http post {url}
        "#,
            url = server.url()
        )
        .as_str()
    ));

    assert!(actual.out.is_empty())
}

#[test]
fn http_post_failed_due_to_server_error() {
    let mut server = Server::new();

    let _mock = server.mock("POST", "/").with_status(400).create();

    let actual = nu!(pipeline(
        format!(
            r#"
        http post {url} "body"
        "#,
            url = server.url()
        )
        .as_str()
    ));

    assert!(actual.err.contains("Bad request (400)"))
}

#[test]
fn http_post_failed_due_to_missing_body() {
    let mut server = Server::new();

    let _mock = server.mock("POST", "/").create();

    let actual = nu!(pipeline(
        format!(
            r#"
        http post {url}
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
fn http_post_failed_due_to_unexpected_body() {
    let mut server = Server::new();

    let _mock = server.mock("POST", "/").match_body("foo").create();

    let actual = nu!(pipeline(
        format!(
            r#"
        http post {url} "bar"
        "#,
            url = server.url()
        )
        .as_str()
    ));

    assert!(actual.err.contains("Cannot make request"))
}

#[test]
fn http_post_json_is_success() {
    let mut server = Server::new();

    let mock = server
        .mock("POST", "/")
        .match_body(r#"{"foo":"bar"}"#)
        .create();

    let actual = nu!(format!(
        r#"http post -t 'application/json' {url} {{foo: 'bar'}}"#,
        url = server.url()
    ));

    mock.assert();
    assert!(actual.out.is_empty())
}

#[test]
fn http_post_json_list_is_success() {
    let mut server = Server::new();

    let mock = server
        .mock("POST", "/")
        .match_body(r#"[{"foo":"bar"}]"#)
        .create();

    let actual = nu!(format!(
        r#"http post -t 'application/json' {url} [{{foo: "bar"}}]"#,
        url = server.url()
    ));

    mock.assert();
    assert!(actual.out.is_empty())
}

#[test]
fn http_post_follows_redirect() {
    let mut server = Server::new();

    let _mock = server.mock("GET", "/bar").with_body("bar").create();
    let _mock = server
        .mock("POST", "/foo")
        .with_status(301)
        .with_header("Location", "/bar")
        .create();

    let actual = nu!(pipeline(
        format!("http post {url}/foo postbody", url = server.url()).as_str()
    ));

    assert_eq!(&actual.out, "bar");
}

#[test]
fn http_post_redirect_mode_manual() {
    let mut server = Server::new();

    let _mock = server
        .mock("POST", "/foo")
        .with_status(301)
        .with_body("foo")
        .with_header("Location", "/bar")
        .create();

    let actual = nu!(pipeline(
        format!(
            "http post --redirect-mode manual {url}/foo postbody",
            url = server.url()
        )
        .as_str()
    ));

    assert_eq!(&actual.out, "foo");
}

#[test]
fn http_post_redirect_mode_error() {
    let mut server = Server::new();

    let _mock = server
        .mock("POST", "/foo")
        .with_status(301)
        .with_body("foo")
        .with_header("Location", "/bar")
        .create();

    let actual = nu!(pipeline(
        format!(
            "http post --redirect-mode error {url}/foo postbody",
            url = server.url()
        )
        .as_str()
    ));

    assert!(&actual.err.contains("nu::shell::network_failure"));
    assert!(&actual.err.contains(
        "Redirect encountered when redirect handling mode was 'error' (301 Moved Permanently)"
    ));
}
