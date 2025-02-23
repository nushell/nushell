use std::sync::OnceLock;

use nu_engine::command_prelude::*;
use nu_protocol::engine::StateWorkingSet;
use shadow_rs::shadow;

shadow!(build);

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

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Display Nu version",
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

pub fn version(engine_state: &EngineState, span: Span) -> Result<PipelineData, ShellError> {
    // Pre-allocate the arrays in the worst case (17 items):
    // - version
    // - major
    // - minor
    // - patch
    // - pre
    // - branch
    // - commit_hash
    // - build_os
    // - build_target
    // - rust_version
    // - rust_channel
    // - cargo_version
    // - build_time
    // - build_rust_channel
    // - allocator
    // - features
    // - installed_plugins
    let mut record = Record::with_capacity(17);

    record.push("version", Value::string(env!("CARGO_PKG_VERSION"), span));

    push_version_numbers(&mut record, span);

    push_non_empty(&mut record, "pre", build::PKG_VERSION_PRE, span);

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
        Value::string(features_enabled().join(", "), span),
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

    Ok(Value::record(record, span).into_pipeline_data())
}

/// Add version numbers as integers to the given record
fn push_version_numbers(record: &mut Record, head: Span) {
    static VERSION_NUMBERS: OnceLock<(u8, u8, u8)> = OnceLock::new();

    let &(major, minor, patch) = VERSION_NUMBERS.get_or_init(|| {
        (
            build::PKG_VERSION_MAJOR.parse().expect("Always set"),
            build::PKG_VERSION_MINOR.parse().expect("Always set"),
            build::PKG_VERSION_PATCH.parse().expect("Always set"),
        )
    });
    record.push("major", Value::int(major.into(), head));
    record.push("minor", Value::int(minor.into(), head));
    record.push("patch", Value::int(patch.into(), head));
}

fn global_allocator() -> &'static str {
    if cfg!(feature = "mimalloc") {
        "mimalloc"
    } else {
        "standard"
    }
}

fn features_enabled() -> Vec<String> {
    let mut names = vec!["default".to_string()];

    // NOTE: There should be another way to know features on.

    #[cfg(feature = "trash-support")]
    {
        names.push("trash".to_string());
    }

    #[cfg(feature = "sqlite")]
    {
        names.push("sqlite".to_string());
    }

    #[cfg(feature = "static-link-openssl")]
    {
        names.push("static-link-openssl".to_string());
    }

    #[cfg(feature = "system-clipboard")]
    {
        names.push("system-clipboard".to_string());
    }

    names.sort();

    names
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() {
        use super::Version;
        use crate::test_examples;
        test_examples(Version {})
    }
}
