use std::{thread, time::Duration};

use mockito::{Matcher, Server, ServerOpts};
use nu_protocol::shell_error;
use nu_test_support::prelude::*;

#[test]
fn http_post_is_success() -> Result {
    let mut server = Server::new();
    let _mock = server.mock("POST", "/").match_body("foo").create();
    let code = format!(r#"http post {url} "foo""#, url = server.url());
    test().run(code).expect_value_eq("")
}
#[test]
fn http_post_is_success_pipeline() -> Result {
    let mut server = Server::new();
    let _mock = server.mock("POST", "/").match_body("foo").create();
    let code = format!(r#""foo" | http post {url}"#, url = server.url());
    test().run(code).expect_value_eq("")
}

#[test]
fn http_post_failed_due_to_server_error() -> Result {
    let mut server = Server::new();

    let _mock = server.mock("POST", "/").with_status(400).create();

    let code = format!(r#"http post {url} "body""#, url = server.url());
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
fn http_post_failed_due_to_missing_body() -> Result {
    let mut server = Server::new();

    let _mock = server.mock("POST", "/").create();

    let code = format!("http post {url}", url = server.url());
    let err = test().run(code).expect_shell_error()?.generic_error()?;
    assert_eq!(
        err,
        "Data must be provided either through pipeline or positional argument"
    );
    Ok(())
}

#[test]
fn http_post_failed_due_to_unexpected_body() -> Result {
    let mut server = Server::new();

    let _mock = server.mock("POST", "/").match_body("foo").create();

    let code = format!(r#"http post {url} "bar""#, url = server.url());
    let err = test().run(code).expect_shell_error()?;

    match err {
        ShellError::NetworkFailure { msg, .. } => {
            assert_contains("Cannot make request", msg);
            Ok(())
        }
        err => Err(err.into()),
    }
}

const JSON: &str = r#"{
  "foo": "bar"
}"#;

#[test]
fn http_post_json_is_success() -> Result {
    let mut server = Server::new();

    let mock = server.mock("POST", "/").match_body(JSON).create();

    let code = format!(
        "http post -t 'application/json' {url} {{foo: 'bar'}}",
        url = server.url()
    );

    test().run(code).expect_value_eq("")?;
    mock.assert();
    Ok(())
}

#[test]
fn http_post_json_string_is_success() -> Result {
    let mut server = Server::new();

    let mock = server.mock("POST", "/").match_body(JSON).create();

    let code = format!(
        r#"http post -t 'application/json' {url} '{{"foo":"bar"}}'"#,
        url = server.url()
    );

    test().run(code).expect_value_eq("")?;
    mock.assert();
    Ok(())
}

const JSON_LIST: &str = r#"[
  {
    "foo": "bar"
  }
]"#;

#[test]
fn http_post_json_list_is_success() -> Result {
    let mut server = Server::new();

    let mock = server.mock("POST", "/").match_body(JSON_LIST).create();

    let code = format!(
        r#"http post -t 'application/json' {url} [{{foo: "bar"}}]"#,
        url = server.url()
    );

    test().run(code).expect_value_eq("")?;
    mock.assert();
    Ok(())
}

#[test]
fn http_post_json_int_is_success() -> Result {
    let mut server = Server::new();

    let mock = server.mock("POST", "/").match_body("50").create();

    let code = format!(
        "http post -t 'application/json' {url} 50",
        url = server.url()
    );

    test().run(code).expect_value_eq("")?;
    mock.assert();
    Ok(())
}

#[test]
fn http_post_json_raw_string_is_success() -> Result {
    let mut server = Server::new();

    let mock = server.mock("POST", "/").match_body(r#""test""#).create();

    let code = format!(
        r#"http post -t 'application/json' {url} "test""#,
        url = server.url()
    );

    test().run(code).expect_value_eq("")?;
    mock.assert();
    Ok(())
}

#[test]
fn http_post_follows_redirect() -> Result {
    let mut server = Server::new();

    let _mock = server.mock("GET", "/bar").with_body("bar").create();
    let _mock = server
        .mock("POST", "/foo")
        .with_status(301)
        .with_header("Location", "/bar")
        .create();

    let code = format!("http post {url}/foo postbody", url = server.url());
    test().run(code).expect_value_eq("bar")
}

#[test]
fn http_post_redirect_mode_manual() -> Result {
    let mut server = Server::new();

    let _mock = server
        .mock("POST", "/foo")
        .with_status(301)
        .with_body("foo")
        .with_header("Location", "/bar")
        .create();

    let code = format!(
        "http post --redirect-mode manual {url}/foo postbody",
        url = server.url()
    );
    test().run(code).expect_value_eq("foo")
}

#[test]
fn http_post_redirect_mode_error() -> Result {
    let mut server = Server::new();

    let _mock = server
        .mock("POST", "/foo")
        .with_status(301)
        .with_body("foo")
        .with_header("Location", "/bar")
        .create();

    let code = format!(
        "http post --redirect-mode error {url}/foo postbody",
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
#[test]
fn http_post_multipart_is_success() -> Result {
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
            Matcher::Regex("(?m)^Content-Type: application/octet-stream".to_string()),
            Matcher::Regex("(?m)^Content-Length: 3".to_string()),
            Matcher::Regex("(?m)^bar".to_string()),
        ]))
        .with_status(200)
        .create();

    let code = format!(
        "http post --content-type multipart/form-data {url} {{foo: ('bar' | into binary) }}",
        url = server.url()
    );

    test().run(code).expect_value_eq("")
}

#[test]
fn http_post_timeout() -> Result {
    let mut server = Server::new();
    let _mock = server
        .mock("POST", "/")
        .with_chunked_body(|w| {
            thread::sleep(Duration::from_secs(10));
            w.write_all(b"Delayed response!")
        })
        .create();

    let code = format!(
        "http post --max-time 100ms {url} postbody",
        url = server.url()
    );
    let err = test().run(code).expect_io_error()?;
    assert!(matches!(
        err.kind,
        shell_error::io::ErrorKind::Std(std::io::ErrorKind::TimedOut, ..)
    ));
    Ok(())
}
