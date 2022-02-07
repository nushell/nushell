use std::convert::TryInto;

use crate::prelude::*;
use nu_engine::{EnvVar, WholeStreamCommand};

use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, Value};

pub struct LoadEnv;

impl WholeStreamCommand for LoadEnv {
    fn name(&self) -> &str {
        "load-env"
    }

    fn signature(&self) -> Signature {
        Signature::build("load-env").optional(
            "environ",
            SyntaxShape::Any,
            "Optional environment table to load in. If not provided, will use the table provided on the input stream",
        )
    }

    fn usage(&self) -> &str {
        "Set environment variables using a table stream"
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        load_env(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Load variables from an input stream",
                example: r#"echo [[name, value]; ["NAME", "JT"] ["AGE", "UNKNOWN"]] | load-env; echo $nu.env.NAME"#,
                result: Some(vec![Value::from("JT")]),
            },
            Example {
                description: "Load variables from an argument",
                example: r#"load-env [[name, value]; ["NAME", "JT"] ["AGE", "UNKNOWN"]]; echo $nu.env.NAME"#,
                result: Some(vec![Value::from("JT")]),
            },
            Example {
                description: "Load variables from an argument and an input stream",
                example: r#"echo [[name, value]; ["NAME", "JT"]] | load-env [[name, value]; ["VALUE", "FOO"]]; echo $nu.env.NAME $nu.env.VALUE"#,
                result: Some(vec![Value::from("JT"), Value::from("UNKNOWN")]),
            },
        ]
    }
}

fn load_env_from_table(
    values: impl IntoIterator<Item = Value>,
    ctx: &EvaluationContext,
) -> Result<(), ShellError> {
    for value in values {
        let mut var_name = None;
        let mut var_value = None;

        let tag = value.tag();

        for (key, value) in value.row_entries() {
            if key == "name" {
                var_name = Some(value);
            } else if key == "value" {
                var_value = Some(value);
            }
        }

        match (var_name, var_value) {
            (Some(name), Some(value)) => {
                let env_var: EnvVar = value.try_into()?;
                ctx.scope.add_env_var(name.as_string()?, env_var);
            }
            _ => {
                return Err(ShellError::labeled_error(
                    r#"Expected each row in the table to have a "name" and "value" field."#,
                    r#"expected a "name" and "value" field"#,
                    tag,
                ))
            }
        }
    }

    Ok(())
}

pub fn load_env(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let ctx = &args.context;

    if let Some(values) = args.opt::<Vec<Value>>(0)? {
        load_env_from_table(values, ctx)?;
    }

    load_env_from_table(args.input, ctx)?;

    Ok(ActionStream::empty())
}
