use std::{borrow::Cow, sync::OnceLock};

use itertools::Itertools;
use nu_engine::command_prelude::*;
use nu_protocol::engine::StateWorkingSet;
use shadow_rs::shadow;

shadow!(build);

/// Static container for the version shown by the `version` command.
///
/// By default, this falls back to `nu_cmd_lang`'s own crate version
/// (`env!("CARGO_PKG_VERSION")`).
///
/// If you're embedding Nushell (or wiring `nu_cmd_lang` into another binary),
/// set this static before calling `version` so it reports your host binary's version.
///
/// ```no_run
/// let version = env!("CARGO_PKG_VERSION")
///     .parse()
///     .expect("cargo sets valid version");
///
/// nu_cmd_lang::VERSION
///     .set(version)
///     .expect("VERSION is unset");
/// ```
pub static VERSION: OnceLock<semver::Version> = OnceLock::new();

/// Static container for the cargo features used by the `version` command.
///
/// This `OnceLock` holds the features from `nu`.
/// When you build `nu_cmd_lang`, Cargo doesn't pass along the same features that `nu` itself uses.
/// By setting this static before calling `version`, you make it show `nu`'s features instead
/// of `nu_cmd_lang`'s.
///
/// Embedders can set this to any feature list they need, but in most cases you'll probably want to
/// pass the cargo features of your host binary.
///
/// # How to get cargo features in your build script
///
/// In your binary's build script:
/// ```no_run
/// // Re-export CARGO_CFG_FEATURE to the main binary.
/// // It holds all the features that cargo sets for your binary as a comma-separated list.
/// println!(
///     "cargo:rustc-env=NU_FEATURES={}",
///     std::env::var("CARGO_CFG_FEATURE").expect("set by cargo")
/// );
/// ```
///
/// Then, before you call `version`:
/// ```ignore
/// // This uses static strings, but since we're using `Cow`, you can also pass owned strings.
/// let features = env!("NU_FEATURES")
///     .split(',')
///     .map(Cow::Borrowed)
///     .collect();
///
/// nu_cmd_lang::VERSION_NU_FEATURES
///     .set(features)
///     .expect("couldn't set VERSION_NU_FEATURES");
/// ```
pub static VERSION_NU_FEATURES: OnceLock<Vec<Cow<'static, str>>> = OnceLock::new();

#[derive(Clone)]
pub struct Version;

impl Command for Version {
    fn name(&self) -> &str {
        "version"
    }

    fn signature(&self) -> Signature {
        Signature::build("version")
            .input_output_types(vec![(Type::Nothing, Type::record())])
            .allow_variants_without_examples(true)
            .category(Category::Core)
    }

    fn description(&self) -> &str {
        "Display Nu version, and its build configuration."
    }

    fn is_const(&self) -> bool {
        true
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        version(engine_state, call.head)
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        version(working_set.permanent(), call.head)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Display Nu version.",
            example: "version",
            result: None,
        }]
    }
}

fn push_non_empty(record: &mut Record, name: &str, value: &str, span: Span) {
    if !value.is_empty() {
        record.push(name, Value::string(value, span))
    }
}

fn load_version() -> Cow<'static, semver::Version> {
    match VERSION.get() {
        Some(version) => Cow::Borrowed(version),
        None => Cow::Owned(
            semver::Version::parse(env!("CARGO_PKG_VERSION")).expect("cargo sets valid version"),
        ),
    }
}

pub fn version(engine_state: &EngineState, span: Span) -> Result<PipelineData, ShellError> {
    // Pre-allocate the arrays in the worst case:
    const VERSION_MAX_ROWS: usize = [
        "version",
        "major",
        "minor",
        "patch",
        "pre",
        "build",
        "branch",
        "commit_hash",
        "build_os",
        "build_target",
        "rust_version",
        "rust_channel",
        "cargo_version",
        "build_time",
        "build_rust_channel",
        "allocator",
        "features",
        "installed_plugins",
    ]
    .len();

    let mut record = Record::with_capacity(VERSION_MAX_ROWS);

    let version = load_version();
    record.push("version", Value::string(version.to_string(), span));
    record.push("major", Value::int(version.major as i64, span));
    record.push("minor", Value::int(version.minor as i64, span));
    record.push("patch", Value::int(version.patch as i64, span));
    push_non_empty(&mut record, "pre", version.pre.as_str(), span);
    push_non_empty(&mut record, "build", version.build.as_str(), span);

    record.push("branch", Value::string(build::BRANCH, span));

    if let Some(commit_hash) = option_env!("NU_COMMIT_HASH") {
        record.push("commit_hash", Value::string(commit_hash, span));
    }

    push_non_empty(&mut record, "build_os", build::BUILD_OS, span);
    push_non_empty(&mut record, "build_target", build::BUILD_TARGET, span);
    push_non_empty(&mut record, "rust_version", build::RUST_VERSION, span);
    push_non_empty(&mut record, "rust_channel", build::RUST_CHANNEL, span);
    push_non_empty(&mut record, "cargo_version", build::CARGO_VERSION, span);
    push_non_empty(&mut record, "build_time", build::BUILD_TIME, span);
    push_non_empty(
        &mut record,
        "build_rust_channel",
        build::BUILD_RUST_CHANNEL,
        span,
    );

    record.push("allocator", Value::string(global_allocator(), span));

    record.push(
        "features",
        Value::string(
            VERSION_NU_FEATURES
                .get()
                .as_ref()
                .map(|v| v.as_slice())
                .unwrap_or_default()
                .iter()
                .filter(|f| !f.starts_with("dep:"))
                .join(", "),
            span,
        ),
    );

    #[cfg(not(feature = "plugin"))]
    let _ = engine_state;

    #[cfg(feature = "plugin")]
    {
        // Get a list of plugin names and versions if present
        let installed_plugins = engine_state
            .plugins()
            .iter()
            .map(|x| {
                let name = x.identity().name();
                if let Some(version) = x.metadata().and_then(|m| m.version) {
                    format!("{name} {version}")
                } else {
                    name.into()
                }
            })
            .collect::<Vec<_>>();

        record.push(
            "installed_plugins",
            Value::string(installed_plugins.join(", "), span),
        );
    }

    record.push(
        "experimental_options",
        Value::string(
            nu_experimental::ALL
                .iter()
                .map(|option| format!("{}={}", option.identifier(), option.get()))
                .join(", "),
            span,
        ),
    );

    Ok(Value::record(record, span).into_pipeline_data())
}

fn global_allocator() -> &'static str {
    "standard"
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() -> nu_test_support::Result {
        use super::Version;
        nu_test_support::test().examples(Version)
    }
}
