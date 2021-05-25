use crate::prelude::*;
use nu_engine::WholeStreamCommand;

use nu_errors::ShellError;
use nu_protocol::{Signature, Value};

pub struct LoadEnv;

#[derive(Deserialize)]
struct LoadEnvArgs {}

impl WholeStreamCommand for LoadEnv {
    fn name(&self) -> &str {
        "load-env"
    }

    fn signature(&self) -> Signature {
        Signature::build("load-env")
    }

    fn usage(&self) -> &str {
        "Set environment variables using a table stream"
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        load_env(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Load a few variables",
            example: r#"echo [[name, value]; ["NAME", "JT"] ["AGE", "UNKNOWN"]] | load-env; echo $nu.env.NAME"#,
            result: Some(vec![Value::from("JT")]),
        }]
    }
}

pub fn load_env(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let ctx = EvaluationContext::from_args(&args);

    let (LoadEnvArgs {}, stream) = args.process()?;

    for value in stream {
        let mut var_name = None;
        let mut var_value = None;

        let tag = value.tag();

        for (key, value) in value.row_entries() {
            if key == "name" {
                var_name = Some(value.as_string()?);
            } else if key == "value" {
                var_value = Some(value.as_string()?);
            }
        }

        match (var_name, var_value) {
            (Some(name), Some(value)) => ctx.scope.add_env_var(name, value),
            _ => {
                return Err(ShellError::labeled_error(
                    r#"Expected each row in the table to have a "name" and "value" field."#,
                    r#"expected a "name" and "value" field"#,
                    tag,
                ))
            }
        }
    }

    /*
    ctx.scope.add_vars(&captured.entries);

    let value = evaluate_baseline_expr(&expr, &ctx);

    ctx.scope.exit_scope();

    let value = value?;
    let value = value.as_string()?;

    let name = name.item;

    */

    Ok(ActionStream::empty())
}
