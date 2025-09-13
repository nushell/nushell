use nu_engine::{command_prelude::*, eval_block};
use nu_protocol::{debugger::WithoutDebug, engine::Closure};

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
        "Runs a block with an environment variable set."
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
        vec![Example {
            description: "Set by key-value record",
            example: r#"with-env {X: "Y", W: "Z"} { [$env.X $env.W] }"#,
            result: Some(Value::list(
                vec![Value::test_string("Y"), Value::test_string("Z")],
                Span::test_data(),
            )),
        }]
    }
}

fn with_env(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let env: Record = call.req(engine_state, stack, 0)?;
    let capture_block: Closure = call.req(engine_state, stack, 1)?;
    let block = engine_state.get_block(capture_block.block_id);
    let mut stack = stack.captures_to_stack_preserve_out_dest(capture_block.captures);

    // TODO: factor list of prohibited env vars into common place
    for prohibited in ["PWD", "FILE_PWD", "CURRENT_FILE"] {
        if env.contains(prohibited) {
            return Err(ShellError::AutomaticEnvVarSetManually {
                envvar_name: prohibited.into(),
                span: call.head,
            });
        }
    }

    for (k, v) in env {
        stack.add_env_var(k, v);
    }

    eval_block::<WithoutDebug>(engine_state, &mut stack, block, input).map(|p| p.body)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(WithEnv {})
    }
}
