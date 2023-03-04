use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Example, IntoPipelineData, PipelineData, ShellError, Signature, Type, Value};
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
    }

    fn usage(&self) -> &str {
        "Display Nu version, and its build configuration."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        version(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Display Nu version",
            example: "version",
            result: None,
        }]
    }
}

pub fn version(
    engine_state: &EngineState,
    _stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
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
    let mut cols = Vec::with_capacity(12);
    let mut vals = Vec::with_capacity(12);

    cols.push("version".to_string());
    vals.push(Value::string(env!("CARGO_PKG_VERSION"), call.head));

    cols.push("branch".to_string());
    vals.push(Value::string(build::BRANCH, call.head));

    let commit_hash = option_env!("NU_COMMIT_HASH");
    if let Some(commit_hash) = commit_hash {
        cols.push("commit_hash".to_string());
        vals.push(Value::string(commit_hash, call.head));
    }

    let build_os = Some(build::BUILD_OS).filter(|x| !x.is_empty());
    if let Some(build_os) = build_os {
        cols.push("build_os".to_string());
        vals.push(Value::string(build_os, call.head));
    }

    let build_target = Some(build::BUILD_TARGET).filter(|x| !x.is_empty());
    if let Some(build_target) = build_target {
        cols.push("build_target".to_string());
        vals.push(Value::string(build_target, call.head));
    }

    let rust_version = Some(build::RUST_VERSION).filter(|x| !x.is_empty());
    if let Some(rust_version) = rust_version {
        cols.push("rust_version".to_string());
        vals.push(Value::string(rust_version, call.head));
    }

    let rust_channel = Some(build::RUST_CHANNEL).filter(|x| !x.is_empty());
    if let Some(rust_channel) = rust_channel {
        cols.push("rust_channel".to_string());
        vals.push(Value::string(rust_channel, call.head));
    }

    let cargo_version = Some(build::CARGO_VERSION).filter(|x| !x.is_empty());
    if let Some(cargo_version) = cargo_version {
        cols.push("cargo_version".to_string());
        vals.push(Value::string(cargo_version, call.head));
    }

    let build_time = Some(build::BUILD_TIME).filter(|x| !x.is_empty());
    if let Some(build_time) = build_time {
        cols.push("build_time".to_string());
        vals.push(Value::string(build_time, call.head));
    }

    let build_rust_channel = Some(build::BUILD_RUST_CHANNEL).filter(|x| !x.is_empty());
    if let Some(build_rust_channel) = build_rust_channel {
        cols.push("build_rust_channel".to_string());
        vals.push(Value::string(build_rust_channel, call.head));
    }

    cols.push("features".to_string());
    vals.push(Value::String {
        val: features_enabled().join(", "),
        span: call.head,
    });

    // Get a list of command names and check for plugins
    let installed_plugins = engine_state
        .plugin_decls()
        .filter(|x| x.is_plugin().is_some())
        .map(|x| x.name())
        .collect::<Vec<_>>();

    cols.push("installed_plugins".to_string());
    vals.push(Value::String {
        val: installed_plugins.join(", "),
        span: call.head,
    });

    Ok(Value::Record {
        cols,
        vals,
        span: call.head,
    }
    .into_pipeline_data())
}

fn features_enabled() -> Vec<String> {
    let mut names = vec!["default".to_string()];

    // NOTE: There should be another way to know features on.

    #[cfg(feature = "which-support")]
    {
        names.push("which".to_string());
    }

    // always include it?
    names.push("zip".to_string());

    #[cfg(feature = "trash-support")]
    {
        names.push("trash".to_string());
    }

    #[cfg(feature = "sqlite")]
    {
        names.push("database".to_string());
    }

    #[cfg(feature = "dataframe")]
    {
        names.push("dataframe".to_string());
    }

    #[cfg(feature = "static-link-openssl")]
    {
        names.push("static-link-openssl".to_string());
    }

    #[cfg(feature = "extra")]
    {
        names.push("extra".to_string());
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
