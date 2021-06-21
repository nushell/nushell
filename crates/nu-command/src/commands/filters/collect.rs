use crate::prelude::*;
use nu_engine::run_block;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{hir::CapturedBlock, Signature, SyntaxShape, UntaggedValue};

pub struct Command;

impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "collect"
    }

    fn signature(&self) -> Signature {
        Signature::build("collect").required(
            "block",
            SyntaxShape::Block,
            "the block to run once the stream is collected",
        )
    }

    fn usage(&self) -> &str {
        "Collect the stream and pass it to a block."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        collect(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Use the second value in the stream",
            example: "echo 1 2 3 | collect { |x| echo $x.1 }",
            result: Some(vec![UntaggedValue::int(2).into()]),
        }]
    }
}

fn collect(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let external_redirection = args.call_info.args.external_redirection;
    let context = &args.context;
    let tag = args.call_info.name_tag.clone();
    let block: CapturedBlock = args.req(0)?;
    let mut input = args.input;
    let param = if !block.block.params.positional.is_empty() {
        block.block.params.positional[0].0.name()
    } else {
        "$it"
    };

    context.scope.enter_scope();

    context.scope.add_vars(&block.captured.entries);
    let mut input = input.drain_vec();
    match input.len() {
        x if x > 1 => {
            context
                .scope
                .add_var(param, UntaggedValue::Table(input).into_value(tag));
        }
        1 => {
            let item = input.swap_remove(0);
            context.scope.add_var(param, item);
        }
        _ => {}
    }

    let result = run_block(
        &block.block,
        &context,
        InputStream::empty(),
        external_redirection,
    );
    context.scope.exit_scope();

    Ok(result?.into_output_stream())
}

#[cfg(test)]
mod tests {
    use super::Command;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Command {})
    }
}
