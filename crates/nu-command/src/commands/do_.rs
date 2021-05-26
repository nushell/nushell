use crate::prelude::*;
use nu_engine::run_block;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    hir::CapturedBlock, hir::ExternalRedirection, Signature, SyntaxShape, UntaggedValue, Value,
};

pub struct Do;

#[derive(Deserialize, Debug)]
struct DoArgs {
    block: CapturedBlock,
    rest: Vec<Value>,
    ignore_errors: bool,
}

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
            .rest(SyntaxShape::Any, "the parameter(s) for the block")
    }

    fn usage(&self) -> &str {
        "Runs a block, optionally ignoring errors."
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        do_(args)
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
            Example {
                description: "Run the block with a parameter",
                example: r#"do { |x| $x + 100 } 55"#,
                result: Some(vec![UntaggedValue::int(155).into()]),
            },
        ]
    }
}

fn do_(raw_args: CommandArgs) -> Result<ActionStream, ShellError> {
    let external_redirection = raw_args.call_info.args.external_redirection;

    let context = EvaluationContext::from_args(&raw_args);
    let (
        DoArgs {
            ignore_errors,
            rest,
            block,
        },
        input,
    ) = raw_args.process()?;

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

    context.scope.enter_scope();

    context.scope.add_vars(&block.captured.entries);

    for (param, value) in block.block.params.positional.iter().zip(rest) {
        context.scope.add_var(param.0.name(), value.clone());
    }

    let result = run_block(&block.block, &context, input, block_redirection);
    context.scope.exit_scope();

    if ignore_errors {
        // To properly ignore errors we need to redirect stderr, consume it, and remove
        // any errors we see in the process.

        match result {
            Ok(mut stream) => {
                let output = stream.drain_vec();
                context.clear_errors();
                Ok(output.into_iter().to_action_stream())
            }
            Err(_) => Ok(ActionStream::empty()),
        }
    } else {
        result.map(|x| x.to_action_stream())
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
