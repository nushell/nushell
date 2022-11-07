use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    did_you_mean, Category, Example, PipelineData, ShellError, Signature, Span, Spanned,
    SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct HideEnv;

impl Command for HideEnv {
    fn name(&self) -> &str {
        "hide-env"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("hide-env")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .rest(
                "name",
                SyntaxShape::String,
                "environment variable names to hide",
            )
            .switch(
                "ignore-errors",
                "do not throw an error if an environment variable was not found",
                Some('i'),
            )
            .category(Category::Core)
    }

    fn usage(&self) -> &str {
        "Hide environment variables in the current scope"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let env_var_names: Vec<Spanned<String>> = call.rest(engine_state, stack, 0)?;
        let ignore_errors = call.has_flag("ignore-errors");

        for name in env_var_names {
            if stack.remove_env_var(engine_state, &name.item).is_none() && !ignore_errors {
                let all_names: Vec<String> = stack
                    .get_env_var_names(engine_state)
                    .iter()
                    .cloned()
                    .collect();
                if let Some(closest_match) = did_you_mean(&all_names, &name.item) {
                    return Err(ShellError::DidYouMeanCustom(
                        format!("Environment variable '{}' not found", name.item),
                        closest_match,
                        name.span,
                    ));
                } else {
                    return Err(ShellError::EnvVarNotFoundAtRuntime(name.item, name.span));
                }
            }
        }

        Ok(PipelineData::new(call.head))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Hide an environment variable",
            example: r#"let-env HZ_ENV_ABC = 1; hide-env HZ_ENV_ABC; 'HZ_ENV_ABC' in (env).name"#,
            result: Some(Value::boolean(false, Span::test_data())),
        }]
    }
}
