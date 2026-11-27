use mockito::{Server, ServerOpts};
use nu_protocol::shell_error::network::NetworkError;
use nu_test_support::prelude::*;
use rstest::*;

mod delete;
mod dynamic;
mod get;
mod head;
mod options;
mod patch;
mod post;
mod put;

/// Start a mockito server that stays out of mockito's shared pool.
///
/// Use it when a mock's response outlives its client, like the stalled chunked bodies in the
/// `--max-time` tests. Mockito serves each server on one thread and joins the body thread there
/// when the client hangs up, so the server stops answering until the body closure returns. A
/// pooled server would go back to the pool in that state. A request that times out against it
/// stays queued, and after the thaw the server answers it against the mocks of whichever test
/// holds it by then. That test's `mock.assert()` then counts an extra hit.
fn unpooled_server() -> Server {
    Server::new_with_opts(ServerOpts::default())
}

#[rstest]
#[case::delete("delete")]
#[case::get("get")]
#[case::head("head")]
#[case::options("options")]
#[case::patch("patch")]
#[case::post("post")]
#[case::put("put")]
#[case::delete_uppercase("DELETE")]
#[case::get_uppercase("GET")]
#[case::head_uppercase("HEAD")]
#[case::options_uppercase("OPTIONS")]
#[case::patch_uppercase("PATCH")]
#[case::post_uppercase("POST")]
#[case::put_uppercase("PUT")]
fn disallow_dynamic_http_methods(#[case] method: &str) -> Result {
    let code = format!("let method = '{method}'; http $method example.com");
    let err = test().run(code).expect_error()?;
    match err {
        ShellError::Generic(err) => {
            let Some(help) = err.help else {
                return Err(ShellError::Generic(err).into());
            };
            assert_contains(
                format!("Prefer to use `http {}` directly", method.to_lowercase()),
                help,
            );
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn helpful_dns_error_for_unknown_domain() -> Result {
    let err = test().run("http get gibberish").expect_network_error()?;
    assert!(matches!(err, NetworkError::Dns { .. }));
    Ok(())
}
