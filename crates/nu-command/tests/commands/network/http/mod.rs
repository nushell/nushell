use nu_test_support::nu;
use rstest::*;

mod delete;
mod get;
mod head;
mod options;
mod patch;
mod post;
mod put;

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
fn disallow_dynamic_http_methods(#[case] method: &str) {
    assert!(
        nu!(format!("let method = '{method}'; http $method example.com"))
            .err
            .contains(&format!(
                "Prefer to use `http {}` directly",
                method.to_lowercase()
            ))
    );
}
