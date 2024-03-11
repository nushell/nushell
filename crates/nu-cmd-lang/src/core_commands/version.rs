use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack, StateWorkingSet};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, Record, ShellError, Signature, Type, Value,
};
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
            .input_output_types(vec![(Type::Nothing, Type::Record(vec![]))])
            .allow_variants_without_examples(true)
            .category(Category::Core)
    }

    fn usage(&self) -> &str {
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
        version(engine_state, call)
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        version(working_set.permanent(), call)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Display Nu version",
            example: "version",
            result: None,
        }]
    }
}

pub fn version(engine_state: &EngineState, call: &Call) -> Result<PipelineData, ShellError> {
    // Pre-allocate the arrays in the worst case (12 items):
    // - version
    // - branch
    // - commit_hash
    // - build_os
    // - build_target
    // - rust_version
    // - cargo_version
    // - build_time
    // - build_rust_channel
    // - features
    // - installed_plugins
    let mut record = Record::with_capacity(12);

    record.push(
        "version",
        Value::string(env!("CARGO_PKG_VERSION"), call.head),
    );

    record.push("branch", Value::string(build::BRANCH, call.head));

    let commit_hash = option_env!("NU_COMMIT_HASH");
    if let Some(commit_hash) = commit_hash {
        record.push("commit_hash", Value::string(commit_hash, call.head));
    }

    let build_os = Some(build::BUILD_OS).filter(|x| !x.is_empty());
    if let Some(build_os) = build_os {
        record.push("build_os", Value::string(build_os, call.head));
    }

    let build_target = Some(build::BUILD_TARGET).filter(|x| !x.is_empty());
    if let Some(build_target) = build_target {
        record.push("build_target", Value::string(build_target, call.head));
    }

    let rust_version = Some(build::RUST_VERSION).filter(|x| !x.is_empty());
    if let Some(rust_version) = rust_version {
        record.push("rust_version", Value::string(rust_version, call.head));
    }

    let rust_channel = Some(build::RUST_CHANNEL).filter(|x| !x.is_empty());
    if let Some(rust_channel) = rust_channel {
        record.push("rust_channel", Value::string(rust_channel, call.head));
    }

    let cargo_version = Some(build::CARGO_VERSION).filter(|x| !x.is_empty());
    if let Some(cargo_version) = cargo_version {
        record.push("cargo_version", Value::string(cargo_version, call.head));
    }

    let build_time = Some(build::BUILD_TIME).filter(|x| !x.is_empty());
    if let Some(build_time) = build_time {
        record.push("build_time", Value::string(build_time, call.head));
    }

    let build_rust_channel = Some(build::BUILD_RUST_CHANNEL).filter(|x| !x.is_empty());
    if let Some(build_rust_channel) = build_rust_channel {
        record.push(
            "build_rust_channel",
            Value::string(build_rust_channel, call.head),
        );
    }

    record.push("allocator", Value::string(global_allocator(), call.head));

    record.push(
        "features",
        Value::string(features_enabled().join(", "), call.head),
    );

    // Get a list of plugin names
    let installed_plugins = engine_state
        .plugins()
        .iter()
        .map(|x| x.identity().name())
        .collect::<Vec<_>>();

    record.push(
        "installed_plugins",
        Value::string(installed_plugins.join(", "), call.head),
    );

    Ok(Value::record(record, call.head).into_pipeline_data())
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

    #[cfg(feature = "which-support")]
    {
        names.push("which".to_string());
    }

    #[cfg(feature = "trash-support")]
    {
        names.push("trash".to_string());
    }

    #[cfg(feature = "sqlite")]
    {
        names.push("sqlite".to_string());
    }

    #[cfg(feature = "dataframe")]
    {
        names.push("dataframe".to_string());
    }

    #[cfg(feature = "static-link-openssl")]
    {
        names.push("static-link-openssl".to_string());
    }

    #[cfg(feature = "wasi")]
    {
        names.push("wasi".to_string());
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
