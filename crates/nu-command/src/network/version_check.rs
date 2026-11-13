use crate::network::http::client::{RedirectMode, http_client, send_request_no_body};
use nu_engine::command_prelude::*;
use nu_protocol::Signals;
use nu_protocol::shell_error::generic::GenericError;
use semver::Version;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::borrow::Cow;

#[derive(Clone)]
pub struct VersionCheck;

impl Command for VersionCheck {
    fn name(&self) -> &str {
        "version check"
    }

    fn description(&self) -> &str {
        "Checks to see if you have the latest version of nushell."
    }

    fn extra_description(&self) -> &str {
        "If you're running nushell nightly, `version check` will check to see if you are running
 the latest nightly version. If you are running the nushell release, `version check` will check
 to see if you're running the latest release version.

If `GITHUB_TOKEN` (or `GH_TOKEN`) is set in the environment, it will be used to authenticate the
 check against the GitHub API. When both are set, `GH_TOKEN` takes precedence, as with the `gh` CLI."
    }

    fn signature(&self) -> Signature {
        Signature::build("version check")
            .category(Category::Platform)
            .input_output_types(vec![(Type::Nothing, Type::record())])
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Check if you have the latest version of nushell",
            example: "version check",
            result: None,
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let agent = http_client(false, RedirectMode::Follow, None, engine_state, stack)?;
        let token = github_token(engine_state, stack);
        let signals = engine_state.signals();
        let version_check = check_for_latest_nushell_version(&agent, signals, token, call.head)?;
        Ok(version_check.into_pipeline_data())
    }
}

#[derive(Deserialize, Debug)]
struct GitHubRelease {
    tag_name: String,
}

/// Gets a JSON document from GitHub's API without treating non-2xx responses as success.
///
/// Errors from the HTTP stack are passed along as they are; a non-2xx status or an unparsable
/// body is reported with a hint based on what happened.
fn github_get_json<T: DeserializeOwned>(
    agent: &ureq::Agent,
    url: &str,
    token: Option<&str>,
    signals: &Signals,
    span: Span,
) -> Result<T, ShellError> {
    let mut response = send_github_request(agent, url, token, signals, span)?;

    // GitHub rejects an invalid token with a 401 status, so an authorized request that fails with
    // one is retried unauthenticated; without a token the check works with the unauthenticated
    // API rate limit.
    if response.status().as_u16() == 401 && token.is_some() {
        response = send_github_request(agent, url, None, signals, span)?;
    }

    let status = response.status();
    if !status.is_success() {
        return Err(github_check_error(
            format!("http status: {status}"),
            github_status_help(status.as_u16()),
            span,
        ));
    }

    response.body_mut().read_json::<T>().map_err(|err| {
        github_check_error(
            err.to_string(),
            "The response from GitHub is not the expected release data",
            span,
        )
    })
}

/// Sends a GET request to the GitHub API.
fn send_github_request(
    agent: &ureq::Agent,
    url: &str,
    token: Option<&str>,
    signals: &Signals,
    span: Span,
) -> Result<http::Response<ureq::Body>, ShellError> {
    let (response, _) =
        send_request_no_body(build_github_request(agent, url, token), span, span, signals);
    response
}

fn build_github_request(
    agent: &ureq::Agent,
    url: &str,
    token: Option<&str>,
) -> ureq::RequestBuilder<ureq::typestate::WithoutBody> {
    let mut request = agent
        .get(url)
        .header("Accept", "application/vnd.github.v3+json");
    if let Some(token) = token {
        request = request.header("Authorization", format!("Bearer {token}"));
    }
    request
}

/// Help for errors from the status the GitHub API reported.
fn github_status_help(status: u16) -> &'static str {
    match status {
        401 => {
            "GitHub rejected the request as unauthorized; if GITHUB_TOKEN (or GH_TOKEN) is set, it is likely invalid or expired"
        }
        403 | 429 => {
            "GitHub is rate limiting you; a valid GITHUB_TOKEN (or GH_TOKEN) raises the request limit"
        }
        _ => "GitHub may be having API problems",
    }
}

/// Creates the error `version check` reports when the check with GitHub failed.
fn github_check_error(
    msg: impl Into<Cow<'static, str>>,
    help: &'static str,
    span: Span,
) -> ShellError {
    ShellError::Generic(
        GenericError::new("failed to check for the latest version", msg, span).with_help(help),
    )
}

/// Looks up `GITHUB_TOKEN` (or `GH_TOKEN`), skipping variables that do not hold a non-empty token.
///
/// `GH_TOKEN` takes precedence, as with the `gh` CLI.
fn github_token(engine_state: &EngineState, stack: &Stack) -> Option<String> {
    ["GH_TOKEN", "GITHUB_TOKEN"].iter().find_map(|name| {
        stack
            .get_env_var(engine_state, name)
            .and_then(|value| value.coerce_str().ok())
            .map(|token| token.trim().to_owned())
            .filter(|token| !token.is_empty())
    })
}

fn check_for_latest_nushell_version(
    agent: &ureq::Agent,
    signals: &Signals,
    token: Option<String>,
    span: Span,
) -> Result<Value, ShellError> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();

    let mut rec = Record::new();

    let latest_tag = if current_version.contains("nightly") {
        rec.push("channel", Value::string("nightly", span));

        let url = "https://api.github.com/repos/nushell/nightly/releases";
        let releases: Vec<GitHubRelease> =
            github_get_json(agent, url, token.as_deref(), signals, span)?;

        releases.first().map(|release| {
            // Nightly repo tags look like "0.102.0-nightly.4+23dc1b6", but the running
            // nightly build reports its version as "0.102.0-nightly.4", so the build
            // metadata of the tag is stripped before the comparison.
            release
                .tag_name
                .split('+')
                .next()
                .unwrap_or(&release.tag_name)
                .to_owned()
        })
    } else {
        rec.push("channel", Value::string("release", span));

        let url = "https://api.github.com/repos/nushell/nushell/releases/latest";
        let release: GitHubRelease = github_get_json(agent, url, token.as_deref(), signals, span)?;
        Some(release.tag_name)
    };

    let latest = latest_tag
        .map(|tag| {
            Version::parse(tag.strip_prefix('v').unwrap_or(&tag)).map_err(|err| {
                github_check_error(
                    err.to_string(),
                    "The release reported by GitHub does not use a valid version",
                    span,
                )
            })
        })
        .transpose()?;
    let current = Version::parse(&current_version).map_err(|err| {
        github_check_error(
            err.to_string(),
            "The running version of nushell is not a valid semver version",
            span,
        )
    })?;

    match latest {
        Some(latest) if latest > current => {
            rec.push("current", Value::bool(false, span));
            rec.push("latest", Value::string(format!("v{latest}"), span));
        }
        _ => {
            rec.push("current", Value::bool(true, span));
            rec.push("latest", Value::string(current_version, span));
        }
    }

    Ok(Value::record(rec, span))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_token_takes_gh_token_first_like_gh_cli() {
        let engine_state = EngineState::new();
        let mut stack = Stack::new();
        stack.add_env_var("GITHUB_TOKEN".to_owned(), Value::test_string("first"));
        stack.add_env_var("GH_TOKEN".to_owned(), Value::test_string("second"));

        assert_eq!(
            github_token(&engine_state, &stack).as_deref(),
            Some("second")
        );
    }

    #[test]
    fn github_token_falls_through_empty_values_to_the_next_name() {
        let engine_state = EngineState::new();
        let mut stack = Stack::new();
        stack.add_env_var("GH_TOKEN".to_owned(), Value::test_string("  "));
        stack.add_env_var("GITHUB_TOKEN".to_owned(), Value::test_string("token"));

        assert_eq!(
            github_token(&engine_state, &stack).as_deref(),
            Some("token")
        );
    }
}
