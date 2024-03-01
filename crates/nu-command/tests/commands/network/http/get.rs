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

#[test]
fn http_get_with_accept_errors() {
    let mut server = Server::new();

    let _mock = server
        .mock("GET", "/")
        .with_status(400)
        .with_body("error body")
        .create();

    let actual = nu!(pipeline(
        format!(
            r#"
        http get -e {url}
        "#,
            url = server.url()
        )
        .as_str()
    ));

    assert!(actual.out.contains("error body"))
}

#[test]
fn http_get_with_accept_errors_and_full_raw_response() {
    let mut server = Server::new();

    let _mock = server
        .mock("GET", "/")
        .with_status(400)
        .with_body("error body")
        .create();

    let actual = nu!(pipeline(
        format!(
            r#"
        http get -e -f {url} | $"($in.status) => ($in.body)"
        "#,
            url = server.url()
        )
        .as_str()
    ));

    assert!(actual.out.contains("400 => error body"))
}

#[test]
fn http_get_with_accept_errors_and_full_json_response() {
    let mut server = Server::new();

    let _mock = server
        .mock("GET", "/")
        .with_status(400)
        .with_header("content-type", "application/json")
        .with_body(
            r#"
        {"msg": "error body"}
        "#,
        )
        .create();

    let actual = nu!(pipeline(
        format!(
            r#"
        http get -e -f {url} | $"($in.status) => ($in.body.msg)"
        "#,
            url = server.url()
        )
        .as_str()
    ));

    assert!(actual.out.contains("400 => error body"))
}

#[test]
fn http_get_with_custom_headers_as_records() {
    let mut server = Server::new();

    let mock1 = server
        .mock("GET", "/")
        .match_header("content-type", "application/json")
        .with_body(r#"{"hello": "world"}"#)
        .create();

    let mock2 = server
        .mock("GET", "/")
        .match_header("content-type", "text/plain")
        .with_body("world")
        .create();

    let _json_response = nu!(format!(
        "http get -H {{content-type: application/json}} {url}",
        url = server.url()
    ));

    let _text_response = nu!(format!(
        "http get -H {{content-type: text/plain}} {url}",
        url = server.url()
    ));

    mock1.assert();
    mock2.assert();
}

#[test]
fn http_get_full_response() {
    let mut server = Server::new();

    let _mock = server.mock("GET", "/").with_body("foo").create();

    let actual = nu!(pipeline(
        format!(
            "http get --full {url} --headers [foo bar] | to json",
            url = server.url()
        )
        .as_str()
    ));

    let output: serde_json::Value = serde_json::from_str(&actual.out).unwrap();

    assert_eq!(output["status"], 200);
    assert_eq!(output["body"], "foo");

    // There's only one request header, we can get it by index
    assert_eq!(output["headers"]["request"][0]["name"], "foo");
    assert_eq!(output["headers"]["request"][0]["value"], "bar");

    // ... and multiple response headers, so have to search by name
    let header = output["headers"]["response"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["name"] == "connection")
        .unwrap();
    assert_eq!(header["value"], "close");
}

#[test]
fn http_get_follows_redirect() {
    let mut server = Server::new();

    let _mock = server.mock("GET", "/bar").with_body("bar").create();
    let _mock = server
        .mock("GET", "/foo")
        .with_status(301)
        .with_header("Location", "/bar")
        .create();

    let actual = nu!(pipeline(
        format!("http get {url}/foo", url = server.url()).as_str()
    ));

    assert_eq!(&actual.out, "bar");
}

#[test]
fn http_get_redirect_mode_manual() {
    let mut server = Server::new();

    let _mock = server
        .mock("GET", "/foo")
        .with_status(301)
        .with_body("foo")
        .with_header("Location", "/bar")
        .create();

    let actual = nu!(pipeline(
        format!(
            "http get --redirect-mode manual {url}/foo",
            url = server.url()
        )
        .as_str()
    ));

    assert_eq!(&actual.out, "foo");
}

#[test]
fn http_get_redirect_mode_error() {
    let mut server = Server::new();

    let _mock = server
        .mock("GET", "/foo")
        .with_status(301)
        .with_body("foo")
        .with_header("Location", "/bar")
        .create();

    let actual = nu!(pipeline(
        format!(
            "http get --redirect-mode error {url}/foo",
            url = server.url()
        )
        .as_str()
    ));

    assert!(&actual.err.contains("nu::shell::network_failure"));
    assert!(&actual.err.contains(
        "Redirect encountered when redirect handling mode was 'error' (301 Moved Permanently)"
    ));
}

// These tests require network access; they use badssl.com which is a Google-affiliated site for testing various SSL errors.
// Revisit this if these tests prove to be flaky or unstable.
//
// These tests are flaky and cause CI to fail somewhat regularly. See PR #12010.

#[test]
#[ignore = "unreliable test"]
fn http_get_expired_cert_fails() {
    let actual = nu!("http get https://expired.badssl.com/");
    assert!(actual.err.contains("network_failure"));
}

#[test]
#[ignore = "unreliable test"]
fn http_get_expired_cert_override() {
    let actual = nu!("http get --insecure https://expired.badssl.com/");
    assert!(actual.out.contains("<html>"));
}

#[test]
#[ignore = "unreliable test"]
fn http_get_self_signed_fails() {
    let actual = nu!("http get https://self-signed.badssl.com/");
    assert!(actual.err.contains("network_failure"));
}

#[test]
#[ignore = "unreliable test"]
fn http_get_self_signed_override() {
    let actual = nu!("http get --insecure https://self-signed.badssl.com/");
    assert!(actual.out.contains("<html>"));
}
