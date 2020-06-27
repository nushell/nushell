use crate::commands::classified::block::run_block;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{hir::Block, ReturnSuccess, Signature, SyntaxShape, Value};

pub struct Do;

#[derive(Deserialize, Debug)]
struct DoArgs {
    block: Block,
    ignore_errors: bool,
}

#[async_trait]
impl WholeStreamCommand for Do {
    fn name(&self) -> &str {
        "do"
    }

    fn signature(&self) -> Signature {
        Signature::build("with-env")
            .required("block", SyntaxShape::Block, "the block to run ")
            .switch(
                "ignore_errors",
                "ignore errors as the block runs",
                Some('i'),
            )
    }

    fn usage(&self) -> &str {
        "Runs a block, optionally ignoring errors"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        do_(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Run the block",
                example: r#"do { echo hello }"#,
                result: Some(vec![Value::from("hello")]),
            },
            Example {
                description: "Run the block and ignore errors",
                example: r#"do -i { thisisnotarealcommand }"#,
                result: Some(vec![Value::nothing()]),
            },
        ]
    }
}

async fn do_(
    raw_args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let is_last = raw_args.call_info.args.is_last;

    let mut context = Context::from_raw(&raw_args, &registry);
    let scope = raw_args.call_info.scope.clone();
    let (
        DoArgs {
            ignore_errors,
            mut block,
        },
        input,
    ) = raw_args.process(&registry).await?;
    block.set_is_last(!is_last);

    let result = run_block(
        &block,
        &mut context,
        input,
        &scope.it,
        &scope.vars,
        &scope.env,
    )
    .await;

    if ignore_errors {
        match result {
            Ok(mut stream) => {
                let output = stream.drain_vec().await;
                context.clear_errors();
                Ok(futures::stream::iter(output).to_output_stream())
            }
            Err(_) => Ok(OutputStream::one(ReturnSuccess::value(Value::nothing()))),
        }
    } else {
        result.map(|x| x.to_output_stream())
    }
}

#[cfg(test)]
mod tests {
    use super::Do;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Do {})
    }
}
