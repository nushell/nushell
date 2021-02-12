use crate::prelude::*;
use nu_engine::run_block;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{hir::CapturedBlock, hir::ExternalRedirection, Signature, SyntaxShape, Value};

pub struct Do;

#[derive(Deserialize, Debug)]
struct DoArgs {
    block: CapturedBlock,
    ignore_errors: bool,
}

#[async_trait]
impl WholeStreamCommand for Do {
    fn name(&self) -> &str {
        "do"
    }

    fn signature(&self) -> Signature {
        Signature::build("do")
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

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        do_(args).await
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
                result: Some(vec![]),
            },
        ]
    }
}

async fn do_(raw_args: CommandArgs) -> Result<OutputStream, ShellError> {
    let external_redirection = raw_args.call_info.args.external_redirection;

    let context = EvaluationContext::from_args(&raw_args);
    let (
        DoArgs {
            ignore_errors,
            mut block,
        },
        input,
    ) = raw_args.process().await?;

    let block_redirection = match external_redirection {
        ExternalRedirection::None => {
            if ignore_errors {
                ExternalRedirection::Stderr
            } else {
                ExternalRedirection::None
            }
        }
        ExternalRedirection::Stdout => {
            if ignore_errors {
                ExternalRedirection::StdoutAndStderr
            } else {
                ExternalRedirection::Stdout
            }
        }
        x => x,
    };

    block.block.set_redirect(block_redirection);
    context.scope.enter_scope();
    let result = run_block(&block.block, &context, input).await;
    context.scope.exit_scope();

    if ignore_errors {
        // To properly ignore errors we need to redirect stderr, consume it, and remove
        // any errors we see in the process.

        match result {
            Ok(mut stream) => {
                let output = stream.drain_vec().await;
                context.clear_errors();
                Ok(futures::stream::iter(output).to_output_stream())
            }
            Err(_) => Ok(OutputStream::empty()),
        }
    } else {
        result.map(|x| x.to_output_stream())
    }
}

#[cfg(test)]
mod tests {
    use super::Do;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Do {})
    }
}
