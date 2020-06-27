use crate::commands::classified::block::run_block;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{hir::Block, Signature, SyntaxShape, Value};
use nu_source::Tagged;

pub struct WithEnv;

#[derive(Deserialize, Debug)]
struct WithEnvArgs {
    variable: (Tagged<String>, Tagged<String>),
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
        vec![Example {
            description: "Set the MYENV environment variable",
            example: r#"with-env [MYENV "my env value"] { echo $nu.env.MYENV }"#,
            result: Some(vec![Value::from("my env value")]),
        }]
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

    scope.env.insert(variable.0.item, variable.1.item);

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
