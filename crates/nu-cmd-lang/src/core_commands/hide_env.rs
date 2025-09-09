use nu_engine::command_prelude::*;
use nu_protocol::did_you_mean;

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
                "Environment variable names to hide.",
            )
            .switch(
                "ignore-errors",
                "do not throw an error if an environment variable was not found",
                Some('i'),
            )
            .category(Category::Core)
    }

    fn description(&self) -> &str {
        "Hide environment variables in the current scope."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["unset", "drop"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let env_var_names: Vec<Spanned<String>> = call.rest(engine_state, stack, 0)?;
        let ignore_errors = call.has_flag(engine_state, stack, "ignore-errors")?;

        for name in env_var_names {
            if !stack.remove_env_var(engine_state, &name.item) && !ignore_errors {
                let all_names = stack.get_env_var_names(engine_state);
                if let Some(closest_match) = did_you_mean(&all_names, &name.item) {
                    return Err(ShellError::DidYouMeanCustom {
                        msg: format!("Environment variable '{}' not found", name.item),
                        suggestion: closest_match,
                        span: name.span,
                    });
                } else {
                    return Err(ShellError::EnvVarNotFoundAtRuntime {
                        envvar_name: name.item,
                        span: name.span,
                    });
                }
            }
        }

        Ok(PipelineData::empty())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Hide an environment variable",
            example: r#"$env.HZ_ENV_ABC = 1; hide-env HZ_ENV_ABC; 'HZ_ENV_ABC' in $env"#,
            result: Some(Value::test_bool(false)),
        }]
    }
}
