use nu_engine::{command_prelude::*, eval_block};
use nu_protocol::{debugger::WithoutDebug, engine::Closure};
use std::collections::HashMap;

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

    fn usage(&self) -> &str {
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

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Set the MYENV environment variable",
                example: r#"with-env [MYENV "my env value"] { $env.MYENV }"#,
                result: Some(Value::test_string("my env value")),
            },
            Example {
                description: "Set by primitive value list",
                example: r#"with-env [X Y W Z] { $env.X }"#,
                result: Some(Value::test_string("Y")),
            },
            Example {
                description: "Set by single row table",
                example: r#"with-env [[X W]; [Y Z]] { $env.W }"#,
                result: Some(Value::test_string("Z")),
            },
            Example {
                description: "Set by key-value record",
                example: r#"with-env {X: "Y", W: "Z"} { [$env.X $env.W] }"#,
                result: Some(Value::list(
                    vec![Value::test_string("Y"), Value::test_string("Z")],
                    Span::test_data(),
                )),
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
    let variable: Value = call.req(engine_state, stack, 0)?;

    let capture_block: Closure = call.req(engine_state, stack, 1)?;
    let block = engine_state.get_block(capture_block.block_id);
    let mut stack = stack.captures_to_stack_preserve_out_dest(capture_block.captures);

    let mut env: HashMap<String, Value> = HashMap::new();

    match &variable {
        Value::List { vals: table, .. } => {
            if table.len() == 1 {
                // single row([[X W]; [Y Z]])
                match &table[0] {
                    Value::Record { val, .. } => {
                        for (k, v) in &**val {
                            env.insert(k.to_string(), v.clone());
                        }
                    }
                    x => {
                        return Err(ShellError::CantConvert {
                            to_type: "string list or single row".into(),
                            from_type: x.get_type().to_string(),
                            span: call
                                .positional_nth(1)
                                .expect("already checked through .req")
                                .span,
                            help: None,
                        });
                    }
                }
            } else {
                // primitive values([X Y W Z])
                for row in table.chunks(2) {
                    if row.len() == 2 {
                        env.insert(row[0].coerce_string()?, row[1].clone());
                    }
                    // TODO: else error?
                }
            }
        }
        // when get object by `open x.json` or `from json`
        Value::Record { val, .. } => {
            for (k, v) in &**val {
                env.insert(k.clone(), v.clone());
            }
        }
        x => {
            return Err(ShellError::CantConvert {
                to_type: "string list or single row".into(),
                from_type: x.get_type().to_string(),
                span: call
                    .positional_nth(1)
                    .expect("already checked through .req")
                    .span,
                help: None,
            });
        }
    };

    for (k, v) in env {
        stack.add_env_var(k, v);
    }

    eval_block::<WithoutDebug>(engine_state, &mut stack, block, input)
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
