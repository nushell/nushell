use mockito::Server;
use nu_protocol::shell_error;
use nu_test_support::prelude::*;
use std::{thread, time::Duration};

#[test]
fn http_get_is_success() -> Result {
    let mut server = Server::new();
    let _mock = server.mock("GET", "/").with_body("foo").create();
    let code = format!("http get {url}", url = server.url());
    test().run(code).expect_value_eq("foo")
}

#[test]
fn http_get_failed_due_to_server_error() -> Result {
    let mut server = Server::new();
    let _mock = server.mock("GET", "/").with_status(400).create();
    let code = format!("http get {url}", url = server.url());
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
fn http_get_with_accept_errors() -> Result {
    let mut server = Server::new();

    let _mock = server
        .mock("GET", "/")
        .with_status(400)
        .with_body("error body")
        .create();

    let code = format!("http get -e {url}", url = server.url());
    test().run(code).expect_value_eq("error body")
}

#[test]
fn http_get_with_accept_errors_and_full_raw_response() -> Result {
    let mut server = Server::new();

    let _mock = server
        .mock("GET", "/")
        .with_status(400)
        .with_body("error body")
        .create();

    #[derive(Debug, FromValue)]
    struct Response {
        status: u16,
        body: String,
    }

    let code = format!("http get -e -f {url}", url = server.url());
    let response: Response = test().run(code)?;
    assert_eq!(response.status, 400);
    assert_eq!(response.body, "error body");
    Ok(())
}

#[test]
fn http_get_with_accept_errors_and_full_json_response() -> Result {
    let mut server = Server::new();

    let _mock = server
        .mock("GET", "/")
        .with_status(400)
        .with_header("content-type", "application/json")
        .with_body(r#"{"msg": "error body"}"#)
        .create();

    #[derive(Debug, FromValue)]
    struct Response {
        status: u16,
        body: ResponseBody,
    }

    #[derive(Debug, FromValue)]
    struct ResponseBody {
        msg: String,
    }

    let code = format!("http get -e -f {url}", url = server.url());
    let response: Response = test().run(code)?;
    assert_eq!(response.status, 400);
    assert_eq!(response.body.msg, "error body");
    Ok(())
}

#[test]
fn http_get_with_custom_headers_as_records() -> Result {
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

    let json_code = format!(
        "http get -H {{content-type: application/json}} {url}",
        url = server.url()
    );

    let text_code = format!(
        "http get -H {{content-type: text/plain}} {url}",
        url = server.url()
    );

    let _: String = test().run(json_code)?;
    let _: String = test().run(text_code)?;

    mock1.assert();
    mock2.assert();
    Ok(())
}

#[test]
fn http_get_full_response() -> Result {
    let mut server = Server::new();

    let _mock = server.mock("GET", "/").with_body("foo").create();

    let code = format!(
        "http get --full {url} --headers [foo bar] | to json",
        url = server.url()
    );

    let outcome: String = test().run(code)?;
    let output: serde_json::Value =
        serde_json::from_str(&outcome).expect("full response should be valid JSON");

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
    Ok(())
}

#[test]
fn http_get_follows_redirect() -> Result {
    let mut server = Server::new();

    let _mock = server.mock("GET", "/bar").with_body("bar").create();
    let _mock = server
        .mock("GET", "/foo")
        .with_status(301)
        .with_header("Location", "/bar")
        .create();

    let code = format!("http get {url}/foo", url = server.url());
    test().run(code).expect_value_eq("bar")
}

#[test]
fn http_get_redirect_mode_manual() -> Result {
    let mut server = Server::new();

    let _mock = server
        .mock("GET", "/foo")
        .with_status(301)
        .with_body("foo")
        .with_header("Location", "/bar")
        .create();

    let code = format!(
        "http get --redirect-mode manual {url}/foo",
        url = server.url()
    );

    test().run(code).expect_value_eq("foo")
}

#[test]
fn http_get_redirect_mode_error() -> Result {
    let mut server = Server::new();

    let _mock = server
        .mock("GET", "/foo")
        .with_status(301)
        .with_body("foo")
        .with_header("Location", "/bar")
        .create();

    let code = format!(
        "http get --redirect-mode error {url}/foo",
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

// These tests require network access; they use badssl.com which is a Google-affiliated site for testing various SSL errors.
// Revisit this if these tests prove to be flaky or unstable.
//
// These tests are flaky and cause CI to fail somewhat regularly. See PR #12010.

#[test]
#[ignore = "unreliable test"]
fn http_get_expired_cert_fails() -> Result {
    let err = test()
        .run("http get https://expired.badssl.com/")
        .expect_shell_error()?;
    assert!(matches!(err, ShellError::NetworkFailure { .. }));
    Ok(())
}

#[test]
#[ignore = "unreliable test"]
fn http_get_expired_cert_override() -> Result {
    let outcome: String = test().run("http get --insecure https://expired.badssl.com/")?;
    assert_contains("<html>", outcome);
    Ok(())
}

#[test]
#[ignore = "unreliable test"]
fn http_get_self_signed_fails() -> Result {
    let err = test()
        .run("http get https://self-signed.badssl.com/")
        .expect_shell_error()?;
    assert!(matches!(err, ShellError::NetworkFailure { .. }));
    Ok(())
}

#[test]
#[ignore = "unreliable test"]
fn http_get_self_signed_override() -> Result {
    let outcome: String = test().run("http get --insecure https://self-signed.badssl.com/")?;
    assert_contains("<html>", outcome);
    Ok(())
}

#[test]
fn http_get_with_invalid_mime_type() -> Result {
    let mut server = Server::new();

    let _mock = server
        .mock("GET", "/foo.nuon")
        .with_status(200)
        // `what&ever` is not a parseable MIME type
        .with_header("content-type", "what&ever")
        .with_body("[1 2 3]")
        .create();

    // but `from nuon` is a known command in nu, so we take `foo.{ext}` and pass it to `from {ext}`
    let code = format!("http get {url}/foo.nuon", url = server.url());

    test().run(code).expect_value_eq([1, 2, 3])
}

#[test]
fn http_get_with_unknown_mime_type() -> Result {
    let mut server = Server::new();
    let _mock = server
        .mock("GET", "/foo")
        .with_status(200)
        // `application/nuon` is not an IANA-registered MIME type
        .with_header("content-type", "application/nuon")
        .with_body("[1 2 3]")
        .create();

    // but `from nuon` is a known command in nu, so we take `{garbage}/{whatever}` and pass it to `from {whatever}`
    let code = format!("http get {url}/foo", url = server.url());

    test().run(code).expect_value_eq([1, 2, 3])
}

#[test]
fn http_get_timeout() -> Result {
    let mut server = Server::new();
    let _mock = server
        .mock("GET", "/")
        .with_chunked_body(|w| {
            thread::sleep(Duration::from_secs(10));
            w.write_all(b"Delayed response!")
        })
        .create();

    let code = format!("http get --max-time 100ms {url}", url = server.url());

    let err = test().run(code).expect_io_error()?;
    assert!(matches!(
        err.kind,
        shell_error::io::ErrorKind::Std(std::io::ErrorKind::TimedOut, ..)
    ));
    Ok(())
}

#[test]
fn http_get_response_metadata() -> Result {
    let mut server = Server::new();

    let _mock = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("x-custom-header", "test-value")
        .with_body("success")
        .create();

    let code = format!(
        "http get --raw {url} | metadata | get http_response | get status",
        url = server.url()
    );

    test().run(code).expect_value_eq(200)
}

#[cfg(unix)]
#[rstest::rstest]
#[case::all_proxy("ALL_PROXY")]
#[case::http_proxy("HTTP_PROXY")]
#[case::https_proxy("HTTPS_PROXY")]
#[timeout(std::time::Duration::from_secs(10))]
#[nu_test_support::test]
#[serial]
fn http_get_with_socks5_proxy(#[case] proxy_env: &str) -> Result {
    use nu_test_support::net::{Address, proxy::Socks5Proxy};
    use std::net::Ipv4Addr;

    let mut server = Server::new();
    let _mock = server.mock("GET", "/").with_body("🦆").create();

    let redirect_port = nu_utils::net::reserve_local_addr().unwrap().port();
    let redirect_addr = Address::IpAddr(Ipv4Addr::LOCALHOST.into(), redirect_port);

    let proxy = Socks5Proxy::builder()
        .unwrap()
        .add_redirect(redirect_addr.clone(), server.socket_address().into())
        .spawn()
        .unwrap();

    let code = format!("http get --raw {redirect_addr}");
    let outcome: String = test().env(proxy_env, proxy.uri()).run(code)?;

    assert_eq!(outcome, "🦆");
    Ok(())
}
