use nu_engine::command_prelude::*;
use serde::Deserialize;
use update_informer::{
    http_client::{GenericHttpClient, HttpClient},
    registry, Check, Package, Registry, Result as UpdateResult,
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
        "If you're running nushell nightly, `version check` will check to see if you are running the latest nightly version. If you are running the nushell release, `version check` will check to see if you're running the latest release version."
    }

    fn signature(&self) -> Signature {
        Signature::build("version check")
            .category(Category::Platform)
            .input_output_types(vec![(Type::Nothing, Type::String)])
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Check if you have the latest version of nushell",
            example: "version check",
            result: None,
        }]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        _call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let version_check = check_for_latest_nushell_version();
        Ok(version_check.into_pipeline_data())
    }
}

pub struct NuShellNightly;

impl Registry for NuShellNightly {
    const NAME: &'static str = "nushell/nightly";

    fn get_latest_version<T: HttpClient>(
        http_client: GenericHttpClient<T>,
        pkg: &Package,
    ) -> UpdateResult<Option<String>> {
        #[derive(Deserialize, Debug)]
        struct Response {
            tag_name: String,
        }

        let url = format!("https://api.github.com/repos/{}/releases", pkg);
        let versions = http_client
            .add_header("Accept", "application/vnd.github.v3+json")
            .add_header("User-Agent", "update-informer")
            .get::<Vec<Response>>(&url)?;

        if let Some(v) = versions.first() {
            // The nightly repo tags look like "0.101.1-nightly.4+23dc1b6"
            // We want to return the "0.101.1-nightly.4" part because hustcer
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

pub fn check_for_latest_nushell_version() -> Value {
    let current_version = env!("CARGO_PKG_VERSION").to_string();

    if current_version.contains("nightly") {
        let nightly_pkg_name = "nushell/nightly";
        // The .interval() determines how long the cached check lives. Setting it to std::time::Duration::ZERO
        // means that there is essentially no cache and it will check for a new version each time you run nushell.
        // Since this is run on demand, there isn't really a need to cache the check.
        let informer =
            update_informer::new(NuShellNightly, nightly_pkg_name, current_version.clone())
                .interval(std::time::Duration::ZERO);

        if let Ok(Some(new_version)) = informer.check_version() {
            Value::test_string(format!(
                "A new release of nushell nightly is available: {current_version} -> {new_version}"
            ))
        } else {
            Value::test_string(format!(
                "You're running the latest version of the nushell nightly v{current_version}."
            ))
        }
    } else {
        let normal_pkg_name = "nushell/nushell";
        // By default, this update request is cached for 24 hours so it won't check for a new version
        // each time you run nushell. Since this is run on demand, there isn't really a need to cache the check which
        // is why we set the interval to std::time::Duration::ZERO.
        let informer =
            update_informer::new(registry::GitHub, normal_pkg_name, current_version.clone())
                .interval(std::time::Duration::ZERO);

        if let Ok(Some(new_version)) = informer.check_version() {
            Value::test_string(format!(
                "A new release of nushell is available: v{current_version} -> {new_version}"
            ))
        } else {
            Value::test_string(format!(
                "You're running the latest version of the nushell release v{current_version}."
            ))
        }
    }
}
