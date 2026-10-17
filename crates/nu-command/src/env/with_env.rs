use nu_engine::env::{is_automatic_env_var, var};
use nu_engine::{command_prelude::*, eval_block};
use nu_protocol::debugger::WithoutDebug;
use nu_protocol::engine::{Closure, env_var_eq};

#[derive(Clone)]
pub struct WithEnv;

impl Command for WithEnv {
    fn name(&self) -> &str {
        "with-env"
    }

    fn signature(&self) -> Signature {
        Signature::build("with-env")
            .input_output_types(vec![(Type::Any, Type::Any)])
            .required(
                "variable",
                SyntaxShape::Any,
                "The environment variable to temporarily set.",
            )
            .required(
                "block",
                SyntaxShape::Closure(None),
                "The block to run once the variable is set.",
            )
            .category(Category::Env)
    }

    fn description(&self) -> &str {
        "Runs a block with temporary environment variables. A config record updates the configuration for the duration of the block."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        with_env(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Set by key-value record.",
                example: r#"with-env {X: "Y", W: "Z"} { [$env.X $env.W] }"#,
                result: Some(Value::list(
                    vec![Value::test_string("Y"), Value::test_string("Z")],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Run a block with changed configuration.",
                example: "with-env {config: {float_precision: 4}} { $env.config.float_precision }",
                result: Some(Value::test_int(4)),
            },
        ]
    }
}

fn with_env(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let env: Spanned<Record> = call.req(engine_state, stack, 0)?;
    let capture_block: Closure = call.req(engine_state, stack, 1)?;

    for (env_var, _) in &env.item {
        if is_automatic_env_var(env_var) {
            return Err(ShellError::AutomaticEnvVarSetManually {
                envvar_name: env_var.to_owned(),
                span: env.span,
            });
        }
    }

    let block = engine_state.get_closure_block(&capture_block, call.head)?;
    let mut stack = stack.captures_to_stack_preserve_out_dest(capture_block.captures);

    let mut last_config_var = None;

    for (k, v) in env.item {
        if env_var_eq(&k, var::CONFIG) {
            last_config_var = Some((k, v));
        } else {
            stack.add_env_var(k, v);
        }
    }

    if let Some((k, v)) = last_config_var {
        if matches!(&v, Value::Record { .. }) {
            stack.add_env_var(var::CONFIG.to_owned(), v); // preserve canonical casing
            stack.update_config(engine_state)?;
        } else {
            stack.add_env_var(k, v);
        }
    }

    eval_block::<WithoutDebug>(engine_state, &mut stack, block, input).map(|p| p.body)
}
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(WithEnv)
    }
}
