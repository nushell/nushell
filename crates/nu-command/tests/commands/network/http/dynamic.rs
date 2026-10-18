use mockito::Server;
use nu_test_support::prelude::*;
use rstest::rstest;

#[rstest]
#[case("get", "GET", "")]
#[case("head", "HEAD", "")]
#[case("options", "OPTIONS", "")]
#[case("delete", "DELETE", "")]
#[case("post", "POST", "'payload'")]
#[case("put", "PUT", "'payload'")]
#[case("patch", "PATCH", "'payload'")]
#[case("pUt", "PUT", "'payload'")]
fn explicit_method(#[case] method: &str, #[case] wire_method: &str, #[case] body: &str) -> Result {
    let mut server = Server::new();
    let mock = server.mock(wire_method, "/").with_status(204).create();
    let code = format!(
        "let method = '{method}'; http --method $method --full {} {body} | get status",
        server.url(),
    );
    test().run(code).expect_value_eq(204)?;
    mock.assert();
    Ok(())
}

#[rstest]
#[case("post")]
#[case("put")]
#[case("patch")]
#[case("delete")]
fn body_from_pipeline_or_position(
    #[case] method: &str,
    #[values(false, true)] pipeline: bool,
) -> Result {
    let mut server = Server::new();
    let mock = server
        .mock(method.to_ascii_uppercase().as_str(), "/")
        .match_body("payload")
        .with_body("accepted")
        .create();
    let code = if pipeline {
        format!("'payload' | http --method '{method}' {}", server.url())
    } else {
        format!("http --method '{method}' {} 'payload'", server.url())
    };
    test().run(code).expect_value_eq("accepted")?;
    mock.assert();
    Ok(())
}

#[test]
fn byte_stream_keeps_content_type() -> Result {
    let mut server = Server::new();
    let mock = server
        .mock("PUT", "/")
        .match_header("content-type", "text/plain")
        .match_body(if cfg!(windows) {
            "1\r\n2\r\n3\r\n"
        } else {
            "1\n2\n3\n"
        })
        .with_body("accepted")
        .create();
    test()
        .run(format!(
            "1..3 | each {{|x| $x }} | to text | http --method put {}",
            server.url()
        ))
        .expect_value_eq("accepted")?;
    mock.assert();
    Ok(())
}

#[rstest]
#[case("head")]
#[case("options")]
fn header_methods_keep_their_response_shape(#[case] method: &str) -> Result {
    let mut server = Server::new();
    let mock = server
        .mock(method.to_ascii_uppercase().as_str(), "/")
        .with_header("x-test", "present")
        .create();
    test()
        .run(format!(
            "http --method '{method}' {} | where name == x-test | get value.0",
            server.url()
        ))
        .expect_value_eq("present")?;
    mock.assert();
    Ok(())
}

#[rstest]
#[case("get")]
#[case("head")]
#[case("options")]
fn body_is_not_silently_discarded(
    #[case] method: &str,
    #[values(false, true)] pipeline: bool,
) -> Result {
    let code = if pipeline {
        format!("'payload' | http --method '{method}' http://localhost")
    } else {
        format!("http --method '{method}' http://localhost 'payload'")
    };
    let err = test().run(code).expect_shell_error()?.generic_error()?;
    assert_contains("does not accept a request body", err);
    Ok(())
}

#[rstest]
#[case("post")]
#[case("put")]
#[case("patch")]
fn body_required_for_write_methods(#[case] method: &str) -> Result {
    let err = test()
        .run(format!("http --method '{method}' http://localhost"))
        .expect_shell_error()?
        .generic_error()?;
    assert_contains("Data must be provided", err);
    Ok(())
}

#[rstest]
#[case("")]
#[case("invalid")]
#[case("GET / HTTP/1.1")]
fn invalid_method_is_rejected(#[case] method: &str) -> Result {
    let err = test()
        .run(format!("http --method '{method}' http://localhost"))
        .expect_shell_error()?
        .generic_error()?;
    assert_contains("Unsupported HTTP method", err);
    Ok(())
}

#[test]
fn explicit_method_needs_a_url() -> Result {
    let err = test().run("http --method get").expect_shell_error()?;
    assert!(matches!(err, ShellError::MissingParameter { .. }));
    Ok(())
}

#[test]
fn defaults_still_infer_get_and_post() -> Result {
    let mut server = Server::new();
    let get = server.mock("GET", "/").with_body("get").create();
    let post = server
        .mock("POST", "/")
        .match_body("payload")
        .with_body("post")
        .create();
    test()
        .run(format!("http {}", server.url()))
        .expect_value_eq("get")?;
    test()
        .run(format!("http {} payload", server.url()))
        .expect_value_eq("post")?;
    get.assert();
    post.assert();
    Ok(())
}

#[test]
fn piped_body_without_method_does_not_send_get() -> Result {
    let mut server = Server::new();
    let get = server.mock("GET", "/").expect(0).create();
    let err = test()
        .run(format!("'payload' | http {}", server.url()))
        .expect_shell_error()?
        .generic_error()?;
    assert_contains("--method", err);
    get.assert();
    Ok(())
}

#[test]
fn null_pipeline_without_method_keeps_default_get() -> Result {
    let mut server = Server::new();
    let get = server.mock("GET", "/").with_body("get").create();
    test()
        .run(format!("null | http {}", server.url()))
        .expect_value_eq("get")?;
    get.assert();
    Ok(())
}

#[rstest]
#[case::dedicated("http patch")]
#[case::dynamic("http --method patch")]
fn request_options_reach_the_selected_method(#[case] command: &str) -> Result {
    let mut server = Server::new();
    let target = server
        .mock("PATCH", "/")
        .match_header("authorization", "Basic dXNlcjpwYXNz")
        .match_header("x-custom", "present")
        .match_header("content-type", "application/json; charset=utf-8")
        .match_body(mockito::Matcher::Json(serde_json::json!({"a": 1})))
        .with_header("content-type", "application/json")
        .with_body(r#"{"accepted":true}"#)
        .create();
    let result = test()
        .run(format!(
            "{command} --user user --password pass --headers {{x-custom: present}} --content-type application/json {} {{a: 1}} | get accepted",
            server.url()
        ));
    target.assert();
    result.expect_value_eq(true)?;
    Ok(())
}

#[test]
fn response_errors_can_be_returned_in_full() -> Result {
    let mut server = Server::new();
    let mock = server
        .mock("DELETE", "/")
        .with_status(400)
        .with_body("rejected")
        .expect(2)
        .create();
    let err = test()
        .run(format!("http --method delete {}", server.url()))
        .expect_shell_error()?;
    assert!(matches!(err, ShellError::HttpError { .. }));
    test()
        .run(format!(
            "http --method delete --allow-errors --full --raw {} | select status body",
            server.url()
        ))
        .expect_value_eq(test_value!({status: 400, body: "rejected"}))?;
    mock.assert();
    Ok(())
}

#[test]
fn response_can_be_consumed_as_a_stream() -> Result {
    let mut server = Server::new();
    let mock = server
        .mock("GET", "/")
        .with_body("first\nsecond\n")
        .create();
    test()
        .run(format!("http --method get --raw {} | lines", server.url()))
        .expect_value_eq(["first", "second"])?;
    mock.assert();
    Ok(())
}

#[rstest]
#[case("get")]
#[case("head")]
#[case("options")]
fn null_pipeline_is_not_a_body(#[case] method: &str) -> Result {
    let mut server = Server::new();
    let mock = server
        .mock(method.to_ascii_uppercase().as_str(), "/")
        .with_status(204)
        .create();
    test()
        .run(format!(
            "null | http --method '{method}' --full {} | get status",
            server.url()
        ))
        .expect_value_eq(204)?;
    mock.assert();
    Ok(())
}

#[test]
fn positional_body_is_evaluated_once() -> Result {
    let mut server = Server::new();
    let mock = server.mock("POST", "/").match_body("payload").create();
    test()
        .run(format!(
            "mut calls = 0; http --method post {} ($calls += 1; 'payload'); $calls",
            server.url()
        ))
        .expect_value_eq(1)?;
    mock.assert();
    Ok(())
}

#[test]
fn redirect_mode_is_preserved() -> Result {
    let mut server = Server::new();
    let redirect = server
        .mock("GET", "/start")
        .with_status(301)
        .with_header("location", "/finish")
        .with_body("redirected")
        .expect(2)
        .create();
    let target = server.mock("GET", "/finish").with_body("followed").create();
    test()
        .run(format!("http --method get {}/start", server.url()))
        .expect_value_eq("followed")?;
    test()
        .run(format!(
            "http --method get --redirect-mode manual --raw {}/start",
            server.url()
        ))
        .expect_value_eq("redirected")?;
    redirect.assert();
    target.assert();
    Ok(())
}
