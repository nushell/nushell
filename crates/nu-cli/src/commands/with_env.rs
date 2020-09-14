use crate::commands::classified::block::run_block;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{hir::Block, Signature, SpannedTypeName, SyntaxShape, UntaggedValue, Value};

pub struct WithEnv;

#[derive(Deserialize, Debug)]
struct WithEnvArgs {
    variable: Value,
    block: Block,
}

#[async_trait]
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
        "Runs a block with an environment set. Eg) with-env [NAME 'foo'] { echo $nu.env.NAME }"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        with_env(args, registry).await
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

async fn with_env(
    raw_args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();

    let mut context = Context::from_raw(&raw_args, &registry);
    let mut scope = raw_args.call_info.scope.clone();
    let (WithEnvArgs { variable, block }, input) = raw_args.process(&registry).await?;

    match &variable.value {
        UntaggedValue::Table(table) => {
            if table.len() == 1 {
                // single row([[X W]; [Y Z]])
                for (k, v) in table[0].row_entries() {
                    scope.env.insert(k.clone(), v.convert_to_string());
                }
            } else {
                // primitive values([X Y W Z])
                for row in table.chunks(2) {
                    if row.len() == 2 && row[0].is_primitive() && row[1].is_primitive() {
                        scope
                            .env
                            .insert(row[0].convert_to_string(), row[1].convert_to_string());
                    }
                }
            }
        }
        // when get object by `open x.json` or `from json`
        UntaggedValue::Row(row) => {
            for (k, v) in &row.entries {
                scope.env.insert(k.clone(), v.convert_to_string());
            }
        }
        _ => {
            return Err(ShellError::type_error(
                "string list or single row",
                variable.spanned_type_name(),
            ));
        }
    };

    let result = run_block(
        &block,
        &mut context,
        input,
        &scope.it,
        &scope.vars,
        &scope.env,
    )
    .await;

    result.map(|x| x.to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::WithEnv;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(WithEnv {})
    }
}
