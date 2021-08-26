use crate::prelude::*;
use nu_engine::run_block;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    hir::CapturedBlock, hir::ExternalRedirection, Signature, SyntaxShape, UntaggedValue, Value,
};

pub struct Do;

struct DoArgs {
    block: CapturedBlock,
    ignore_errors: bool,
    rest: Vec<Value>,
}

impl WholeStreamCommand for Do {
    fn name(&self) -> &str {
        "do"
    }

    fn signature(&self) -> Signature {
        Signature::build("do")
            .required("block", SyntaxShape::Block, "the block to run ")
            .switch(
                "ignore-errors",
                "ignore errors as the block runs",
                Some('i'),
            )
            .rest("rest", SyntaxShape::Any, "the parameter(s) for the block")
    }

    fn usage(&self) -> &str {
        "Runs a block, optionally ignoring errors."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
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

fn do_(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let external_redirection = args.call_info.args.external_redirection;

    let context = args.context().clone();
    let do_args = DoArgs {
        block: args.req(0)?,
        ignore_errors: args.has_flag("ignore-errors"),
        rest: args.rest(1)?,
    };

    let block_redirection = match external_redirection {
        ExternalRedirection::None => {
            if do_args.ignore_errors {
                ExternalRedirection::Stderr
            } else {
                ExternalRedirection::None
            }
        }
        ExternalRedirection::Stdout => {
            if do_args.ignore_errors {
                ExternalRedirection::StdoutAndStderr
            } else {
                ExternalRedirection::Stdout
            }
        }
        x => x,
    };

    context.scope.enter_scope();

    context.scope.add_vars(&do_args.block.captured.entries);

    for (param, value) in do_args
        .block
        .block
        .params
        .positional
        .iter()
        .zip(do_args.rest)
    {
        context.scope.add_var(param.0.name(), value.clone());
    }

    let result = run_block(
        &do_args.block.block,
        &context,
        args.input,
        block_redirection,
    );
    context.scope.exit_scope();

    if do_args.ignore_errors {
        // To properly ignore errors we need to redirect stderr, consume it, and remove
        // any errors we see in the process.

        match result {
            Ok(mut stream) => {
                let output = stream.drain_vec();
                context.clear_errors();
                Ok(output.into_iter().into_output_stream())
            }
            Err(_) => Ok(OutputStream::empty()),
        }
    } else {
        result.map(|x| x.into_output_stream())
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
