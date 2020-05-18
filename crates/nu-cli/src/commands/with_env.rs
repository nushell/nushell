use crate::commands::classified::block::run_block;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{hir::Block, ReturnSuccess, Signature, SyntaxShape, Value};
use nu_source::Tagged;

pub struct WithEnv;

#[derive(Deserialize, Debug)]
struct WithEnvArgs {
    variable: (Tagged<String>, Tagged<String>),
    block: Block,
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
        "Runs a block with an environment set. Eg) with-env [NAME 'foo'] { echo $nu.env.NAME }"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        with_env(args, registry)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Set the MYENV environment variable",
            example: r#"with-env [MYENV "my env value"] { echo $nu.env.MYENV }"#,
            result: Some(vec![Value::from("my env value")]),
        }]
    }
}

fn with_env(raw_args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();

    let stream = async_stream! {
        let mut context = Context::from_raw(&raw_args, &registry);
        let scope = raw_args
            .call_info
            .scope
            .clone();
        let (WithEnvArgs { variable, block }, mut input) = raw_args.process(&registry).await?;
        let scope = scope.set_env_var(variable.0.item, variable.1.item);

        let result = run_block(
            &block,
            &mut context,
            input,
            &scope.clone(),
        ).await;

        match result {
            Ok(mut stream) => {
                while let Some(result) = stream.next().await {
                    yield Ok(ReturnSuccess::Value(result));
                }

                let errors = context.get_errors();
                if let Some(error) = errors.first() {
                    yield Err(error.clone());
                }
            }
            Err(e) => {
                yield Err(e);
            }
        }
    };

    Ok(stream.to_output_stream())
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
