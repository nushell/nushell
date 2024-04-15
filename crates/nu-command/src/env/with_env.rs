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
    let variable: Value = call.req(engine_state, stack, 0)?;

    let capture_block: Closure = call.req(engine_state, stack, 1)?;
    let block = engine_state.get_block(capture_block.block_id);
    let mut stack = stack.captures_to_stack_preserve_out_dest(capture_block.captures);

    let mut env: HashMap<String, Value> = HashMap::new();

    match &variable {
        Value::List { vals: table, .. } => {
            nu_protocol::report_error_new(
                engine_state,
                &ShellError::GenericError {
                    error: "Deprecated argument type".into(),
                    msg: "providing the variables to `with-env` as a list or single row table has been deprecated".into(),
                    span: Some(variable.span()),
                    help: Some("use the record form instead".into()),
                    inner: vec![],
                },
            );
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
                            to_type: "record".into(),
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
                    if row.len() == 1 {
                        return Err(ShellError::IncorrectValue {
                            msg: format!("Missing value for $env.{}", row[0].coerce_string()?),
                            val_span: row[0].span(),
                            call_span: call.head,
                        });
                    }
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
                to_type: "record".into(),
                from_type: x.get_type().to_string(),
                span: call
                    .positional_nth(1)
                    .expect("already checked through .req")
                    .span,
                help: None,
            });
        }
    };

    // TODO: factor list of prohibited env vars into common place
    for prohibited in ["PWD", "FILE_PWD", "CURRENT_FILE"] {
        if env.contains_key(prohibited) {
            return Err(ShellError::AutomaticEnvVarSetManually {
                envvar_name: prohibited.into(),
                span: call.head,
            });
        }
    }

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
