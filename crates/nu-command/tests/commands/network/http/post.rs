use std::{thread, time::Duration};

use mockito::{Matcher, Server, ServerOpts};
use nu_test_support::nu;

#[test]
fn http_post_is_success() {
    let mut server = Server::new();

    let _mock = server.mock("POST", "/").match_body("foo").create();

    let actual = nu!(format!(r#"http post {url} "foo""#, url = server.url()));

    assert!(actual.out.is_empty())
}
#[test]
fn http_post_is_success_pipeline() {
    let mut server = Server::new();

    let _mock = server.mock("POST", "/").match_body("foo").create();

    let actual = nu!(format!(r#""foo" | http post {url}"#, url = server.url()));

    assert!(actual.out.is_empty())
}

#[test]
fn http_post_failed_due_to_server_error() {
    let mut server = Server::new();

    let _mock = server.mock("POST", "/").with_status(400).create();

    let actual = nu!(format!(r#"http post {url} "body""#, url = server.url()));

    assert!(actual.err.contains("Bad request (400)"))
}

#[test]
fn http_post_failed_due_to_missing_body() {
    let mut server = Server::new();

    let _mock = server.mock("POST", "/").create();

    let actual = nu!(format!(r#"http post {url}"#, url = server.url()));

    assert!(
        actual
            .err
            .contains("Data must be provided either through pipeline or positional argument")
    )
}

#[test]
fn http_post_failed_due_to_unexpected_body() {
    let mut server = Server::new();

    let _mock = server.mock("POST", "/").match_body("foo").create();

    let actual = nu!(format!(r#"http post {url} "bar""#, url = server.url()));

    assert!(actual.err.contains("Cannot make request"))
}

const JSON: &str = r#"{
  "foo": "bar"
}"#;

#[test]
fn http_post_json_is_success() {
    let mut server = Server::new();

    let mock = server.mock("POST", "/").match_body(JSON).create();

    let actual = nu!(format!(
        r#"http post -t 'application/json' {url} {{foo: 'bar'}}"#,
        url = server.url()
    ));

    mock.assert();
    assert!(actual.out.is_empty(), "Unexpected output {:?}", actual.out)
}

#[test]
fn http_post_json_string_is_success() {
    let mut server = Server::new();

    let mock = server.mock("POST", "/").match_body(JSON).create();

    let actual = nu!(format!(
        r#"http post -t 'application/json' {url} '{{"foo":"bar"}}'"#,
        url = server.url()
    ));

    mock.assert();
    assert!(actual.out.is_empty())
}

const JSON_LIST: &str = r#"[
  {
    "foo": "bar"
  }
]"#;

#[test]
fn http_post_json_list_is_success() {
    let mut server = Server::new();

    let mock = server.mock("POST", "/").match_body(JSON_LIST).create();

    let actual = nu!(format!(
        r#"http post -t 'application/json' {url} [{{foo: "bar"}}]"#,
        url = server.url()
    ));

    mock.assert();
    assert!(actual.out.is_empty())
}

#[test]
fn http_post_json_int_is_success() {
    let mut server = Server::new();

    let mock = server.mock("POST", "/").match_body(r#"50"#).create();

    let actual = nu!(format!(
        r#"http post -t 'application/json' {url} 50"#,
        url = server.url()
    ));

    mock.assert();
    assert!(actual.out.is_empty())
}

#[test]
fn http_post_json_raw_string_is_success() {
    let mut server = Server::new();

    let mock = server.mock("POST", "/").match_body(r#""test""#).create();

    let actual = nu!(format!(
        r#"http post -t 'application/json' {url} "test""#,
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

    let actual = nu!(format!("http post {url}/foo postbody", url = server.url()));

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

    let actual = nu!(format!(
        "http post --redirect-mode manual {url}/foo postbody",
        url = server.url()
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

    let actual = nu!(format!(
        "http post --redirect-mode error {url}/foo postbody",
        url = server.url()
    ));

    assert!(&actual.err.contains("nu::shell::network_failure"));
    assert!(&actual.err.contains(
        "Redirect encountered when redirect handling mode was 'error' (301 Moved Permanently)"
    ));
}
#[test]
fn http_post_multipart_is_success() {
    let mut server = Server::new_with_opts(ServerOpts {
        assert_on_drop: true,
        ..Default::default()
    });
    let _mock = server
        .mock("POST", "/")
        .match_header(
            "content-type",
            Matcher::Regex("multipart/form-data; boundary=.*".to_string()),
        )
        .match_body(Matcher::AllOf(vec![
            Matcher::Regex(r#"(?m)^Content-Disposition: form-data; name="foo""#.to_string()),
            Matcher::Regex(r#"(?m)^Content-Type: application/octet-stream"#.to_string()),
            Matcher::Regex(r#"(?m)^Content-Length: 3"#.to_string()),
            Matcher::Regex(r#"(?m)^bar"#.to_string()),
        ]))
        .with_status(200)
        .create();

    let actual = nu!(format!(
        "http post --content-type multipart/form-data {url} {{foo: ('bar' | into binary) }}",
        url = server.url()
    ));

    assert!(actual.out.is_empty())
}

#[test]
fn http_post_timeout() {
    let mut server = Server::new();
    let _mock = server
        .mock("POST", "/")
        .with_chunked_body(|w| {
            thread::sleep(Duration::from_secs(10));
            w.write_all(b"Delayed response!")
        })
        .create();

    let actual = nu!(format!(
        "http post --max-time 100ms {url} postbody",
        url = server.url()
    ));

    assert!(&actual.err.contains("nu::shell::io::timed_out"));
    assert!(&actual.err.contains("Timed out"));
}
