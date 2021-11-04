use std::{
    collections::HashMap,
    convert::{TryFrom, TryInto},
};

use nu_engine::{eval_block, CallExt};
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Example, PipelineData, ShellError, Signature, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct WithEnv;

impl Command for WithEnv {
    fn name(&self) -> &str {
        "with-env"
    }

    fn signature(&self) -> Signature {
        Signature::build("with-env")
            .required(
                "variable",
                SyntaxShape::Any,
                "the environment variable to temporarily set",
            )
            .required(
                "block",
                SyntaxShape::Block(Some(vec![SyntaxShape::Any])),
                "the block to run once the variable is set",
            )
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
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        with_env(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Set the MYENV environment variable",
                example: r#"with-env [MYENV "my env value"] { $nu.env.MYENV }"#,
                result: Some(Value::test_string("my env value")),
            },
            Example {
                description: "Set by primitive value list",
                example: r#"with-env [X Y W Z] { $nu.env.X }"#,
                result: Some(Value::test_string("Y")),
            },
            Example {
                description: "Set by single row table",
                example: r#"with-env [[X W]; [Y Z]] { $nu.env.W }"#,
                result: Some(Value::test_string("Z")),
            },
            Example {
                description: "Set by row(e.g. `open x.json` or `from json`)",
                example: r#"echo '{"X":"Y","W":"Z"}'|from json|with-env $it { echo $nu.env.X $nu.env.W }"#,
                result: None,
            },
        ]
    }
}

#[derive(Debug, Clone)]
pub enum EnvVar {
    Proper(String),
    Nothing,
}

impl TryFrom<&Value> for EnvVar {
    type Error = ShellError;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        if matches!(value, Value::Nothing { .. }) {
            Ok(EnvVar::Nothing)
        } else if let Ok(s) = value.as_string() {
            if s.is_empty() {
                Ok(EnvVar::Nothing)
            } else {
                Ok(EnvVar::Proper(s))
            }
        } else {
            Err(ShellError::CantConvert("string".into(), value.span()?))
        }
    }
}

fn with_env(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    // let external_redirection = args.call_info.args.external_redirection;
    let variable: Value = call.req(engine_state, stack, 0)?;

    let block_id = call.positional[1]
        .as_block()
        .expect("internal error: expected block");
    let block = engine_state.get_block(block_id).clone();
    let mut stack = stack.collect_captures(&block.captures);

    let mut env: HashMap<String, EnvVar> = HashMap::new();

    match &variable {
        Value::List { vals: table, .. } => {
            if table.len() == 1 {
                // single row([[X W]; [Y Z]])
                match &table[0] {
                    Value::Record { cols, vals, .. } => {
                        for (k, v) in cols.iter().zip(vals.iter()) {
                            env.insert(k.to_string(), v.try_into()?);
                        }
                    }
                    _ => {
                        return Err(ShellError::CantConvert(
                            "string list or single row".into(),
                            call.positional[1].span,
                        ));
                    }
                }
            } else {
                // primitive values([X Y W Z])
                for row in table.chunks(2) {
                    if row.len() == 2 {
                        env.insert(row[0].as_string()?, (&row[1]).try_into()?);
                    }
                }
            }
        }
        // when get object by `open x.json` or `from json`
        Value::Record { cols, vals, .. } => {
            for (k, v) in cols.iter().zip(vals) {
                env.insert(k.clone(), v.try_into()?);
            }
        }
        _ => {
            return Err(ShellError::CantConvert(
                "string list or single row".into(),
                call.positional[1].span,
            ));
        }
    };

    for (k, v) in env {
        match v {
            EnvVar::Nothing => {
                stack.env_vars.remove(&k);
            }
            EnvVar::Proper(s) => {
                stack.env_vars.insert(k, s);
            }
        }
    }

    eval_block(engine_state, &mut stack, &block, input)
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
