use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct Title;

impl Command for Title {
    fn name(&self) -> &str {
        "title"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .optional(
                "title_override",
                SyntaxShape::String,
                "Terminal window title to use instead of current path.",
            )
            .category(Category::Shells)
    }

    fn description(&self) -> &str {
        "Use the given value to override the default terminal window title."
    }

    fn extra_description(&self) -> &str {
        "This sets or unsets the NU_REPL_TITLE_OVERRIDE environment variable to override the osc2 default title."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["terminal", "window"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let title_override: Option<String> = call.opt(engine_state, stack, 0)?;
        if let Some(value) = title_override {
            stack.add_env_var(
                "NU_REPL_TITLE_OVERRIDE".to_string(),
                Value::string(value, Span::unknown()),
            );
        } else {
            stack.remove_env_var(engine_state, "NU_REPL_TITLE_OVERRIDE");
        }
        Ok(PipelineData::empty())
    }
}
