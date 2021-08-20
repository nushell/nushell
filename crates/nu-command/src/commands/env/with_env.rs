use std::convert::TryInto;

use crate::prelude::*;
use nu_engine::run_block;
use nu_engine::EnvVar;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    hir::CapturedBlock, Signature, SpannedTypeName, SyntaxShape, UntaggedValue, Value,
};

pub struct WithEnv;

#[derive(Deserialize, Debug)]
struct WithEnvArgs {
    variable: Value,
    block: CapturedBlock,
}

impl WholeStreamCommand for WithEnv {
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
                SyntaxShape::Block,
                "the block to run once the variable is set",
            )
    }

    fn usage(&self) -> &str {
        "Runs a block with an environment variable set."
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        with_env(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Set the MYENV environment variable",
                example: r#"with-env [MYENV "my env value"] { echo $nu.env.MYENV }"#,
                result: Some(vec![Value::from("my env value")]),
            },
            Example {
                description: "Set by primitive value list",
                example: r#"with-env [X Y W Z] { echo $nu.env.X $nu.env.W }"#,
                result: Some(vec![Value::from("Y"), Value::from("Z")]),
            },
            Example {
                description: "Set by single row table",
                example: r#"with-env [[X W]; [Y Z]] { echo $nu.env.X $nu.env.W }"#,
                result: Some(vec![Value::from("Y"), Value::from("Z")]),
            },
            Example {
                description: "Set by row(e.g. `open x.json` or `from json`)",
                example: r#"echo '{"X":"Y","W":"Z"}'|from json|with-env $it { echo $nu.env.X $nu.env.W }"#,
                result: None,
            },
        ]
    }
}

fn with_env(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let external_redirection = args.call_info.args.external_redirection;
    let context = &args.context;
    let variable: Value = args.req(0)?;
    let block: CapturedBlock = args.req(1)?;

    let mut env: IndexMap<String, EnvVar> = IndexMap::new();

    match &variable.value {
        UntaggedValue::Table(table) => {
            if table.len() == 1 {
                // single row([[X W]; [Y Z]])
                for (k, v) in table[0].row_entries() {
                    env.insert(k.clone(), v.try_into()?);
                }
            } else {
                // primitive values([X Y W Z])
                for row in table.chunks(2) {
                    if row.len() == 2 && row[0].is_primitive() && row[1].is_primitive() {
                        env.insert(row[0].convert_to_string(), (&row[1]).try_into()?);
                    }
                }
            }
        }
        // when get object by `open x.json` or `from json`
        UntaggedValue::Row(row) => {
            for (k, v) in &row.entries {
                env.insert(k.clone(), v.try_into()?);
            }
        }
        _ => {
            return Err(ShellError::type_error(
                "string list or single row",
                variable.spanned_type_name(),
            ));
        }
    };

    context.scope.enter_scope();
    context.scope.add_env(env);
    context.scope.add_vars(&block.captured.entries);

    let result = run_block(&block.block, context, args.input, external_redirection);
    context.scope.exit_scope();

    result.map(|x| x.into_action_stream())
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::WithEnv;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(WithEnv {})
    }
}
