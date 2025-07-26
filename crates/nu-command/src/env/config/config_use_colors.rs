use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct ConfigUseColors;

impl Command for ConfigUseColors {
    fn name(&self) -> &str {
        "config use-colors"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .category(Category::Env)
            .input_output_type(Type::Nothing, Type::Bool)
    }

    fn description(&self) -> &str {
        "Get the configuration for color output."
    }

    fn extra_description(&self) -> &str {
        r#"Use this command instead of checking `$env.config.use_ansi_coloring` to properly handle the "auto" setting, including environment variables that influence its behavior."#
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let use_ansi_coloring = engine_state
            .get_config()
            .use_ansi_coloring
            .get(engine_state);
        Ok(PipelineData::value(
            Value::bool(use_ansi_coloring, call.head),
            None,
        ))
    }
}
