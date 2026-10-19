use nu_engine::command_prelude::*;
use nu_protocol::shell_error::generic::GenericError;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::sync::OnceLock;
use update_informer::{
    Check, Package, Registry, Result as UpdateResult,
    http_client::{GenericHttpClient, HttpClient},
};

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
        "If you're running nushell nightly, `version check` will check to see if you are running the latest nightly version. If you are running the nushell release, `version check` will check to see if you're running the latest release version. If `GITHUB_TOKEN` (or `GH_TOKEN`) is set in the environment, it will be used to authenticate the check against the GitHub API."
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
        init_github_auth_header(engine_state, stack);
        let version_check = check_for_latest_nushell_version(call.head)?;
        Ok(version_check.into_pipeline_data())
    }
}

/// The `Authorization` header value used for GitHub API requests.
static GITHUB_AUTH_HEADER: OnceLock<String> = OnceLock::new();

/// Looks up `GITHUB_TOKEN` (or `GH_TOKEN`) and caches the `Authorization` header value to send.
fn init_github_auth_header(engine_state: &EngineState, stack: &Stack) {
    if GITHUB_AUTH_HEADER.get().is_some() {
        return;
    }

    let Some(token) = ["GITHUB_TOKEN", "GH_TOKEN"]
        .iter()
        .find_map(|name| stack.get_env_var(engine_state, name))
        .and_then(|value| value.coerce_str().ok())
        .map(|token| token.trim().to_owned())
        .filter(|token| !token.is_empty())
    else {
        return;
    };

    let _ = GITHUB_AUTH_HEADER.set(format!("Bearer {token}"));
}

fn github_auth_header() -> Option<&'static str> {
    GITHUB_AUTH_HEADER.get().map(String::as_str)
}

fn github_get<'a, T: HttpClient, D: DeserializeOwned>(
    client: GenericHttpClient<'a, T>,
    url: &str,
) -> UpdateResult<D> {
    let client = client
        .add_header("Accept", "application/vnd.github.v3+json")
        .add_header("User-Agent", "update-informer");
    match github_auth_header() {
        Some(token) => client.add_header("Authorization", token).get::<D>(url),
        None => client.get::<D>(url),
    }
}

/// A GitHub registry that authenticates with the `Authorization` header when a token is available.
///
/// `update-informer`'s built-in `GitHub` registry cannot add headers.
pub struct NuShell;

impl Registry for NuShell {
    const NAME: &'static str = "github";

    fn get_latest_version<T: HttpClient>(
        http_client: GenericHttpClient<T>,
        pkg: &Package,
    ) -> UpdateResult<Option<String>> {
        #[derive(Deserialize, Debug)]
        struct Response {
            tag_name: String,
        }

        let url = format!("https://api.github.com/repos/{pkg}/releases/latest");
        let resp: Response = github_get(http_client, &url)?;

        if let Some(v) = resp.tag_name.strip_prefix('v') {
            return Ok(Some(v.to_string()));
        }

        Ok(Some(resp.tag_name))
    }
}

pub struct NuShellNightly;

impl Registry for NuShellNightly {
    const NAME: &'static str = "github";

    fn get_latest_version<T: HttpClient>(
        http_client: GenericHttpClient<T>,
        pkg: &Package,
    ) -> UpdateResult<Option<String>> {
        #[derive(Deserialize, Debug)]
        struct Response {
            tag_name: String,
        }

        let url = format!("https://api.github.com/repos/{pkg}/releases");
        let versions: Vec<Response> = github_get(http_client, &url)?;

        if let Some(v) = versions.first() {
            // The nightly repo tags look like "0.102.0-nightly.4+23dc1b6"
            // We want to return the "0.102.0-nightly.4" part because hustcer
            // is changing the cargo.toml package.version to be that syntax
            let up_through_plus = match v.tag_name.split('+').next() {
                Some(v) => v,
                None => &v.tag_name,
            };
            return Ok(Some(up_through_plus.to_string()));
        }

        Ok(None)
    }
}

pub fn check_for_latest_nushell_version(span: Span) -> Result<Value, ShellError> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();

    let mut rec = Record::new();

    if current_version.contains("nightly") {
        rec.push("channel", Value::string("nightly", span));

        let nightly_pkg_name = "nushell/nightly";
        // The .interval() determines how long the cached check lives. Setting it to std::time::Duration::ZERO
        // means that there is essentially no cache and it will check for a new version each time you run nushell.
        // Since this is run on demand, there isn't really a need to cache the check.
        let informer =
            update_informer::new(NuShellNightly, nightly_pkg_name, current_version.clone())
                .interval(std::time::Duration::ZERO);

        check_informer(informer, current_version, &mut rec, span)?;
    } else {
        rec.push("channel", Value::string("release", span));

        let normal_pkg_name = "nushell/nushell";
        // By default, this update request is cached for 24 hours so it won't check for a new version
        // each time you run nushell. Since this is run on demand, there isn't really a need to cache the check which
        // is why we set the interval to std::time::Duration::ZERO.
        let informer = update_informer::new(NuShell, normal_pkg_name, current_version.clone())
            .interval(std::time::Duration::ZERO);

        check_informer(informer, current_version, &mut rec, span)?;
    }

    Ok(Value::record(rec, span))
}

fn check_informer(
    informer: impl Check,
    current_version: String,
    rec: &mut Record,
    span: Span,
) -> Result<(), ShellError> {
    match informer.check_version() {
        Ok(Some(new_version)) => {
            rec.push("current", Value::bool(false, span));
            rec.push("latest", Value::string(new_version.to_string(), span));
        }
        Err(err) => {
            return Err(ShellError::Generic(
                GenericError::new(
                    "failed to check for the latest version",
                    err.to_string(),
                    span,
                )
                .with_help("GitHub may be rate limiting you or the network may be unavailable"),
            ));
        }
        Ok(None) => {
            rec.push("current", Value::bool(true, span));
            rec.push("latest", Value::string(current_version, span));
        }
    }
    Ok(())
}
