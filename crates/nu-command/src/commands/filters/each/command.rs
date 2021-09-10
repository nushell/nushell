use crate::prelude::*;
use nu_engine::run_block;
use nu_engine::WholeStreamCommand;

use nu_errors::ShellError;
use nu_protocol::{
    hir::{CapturedBlock, ExternalRedirection},
    Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue, Value,
};

pub struct Each;

impl WholeStreamCommand for Each {
    fn name(&self) -> &str {
        "each"
    }

    fn signature(&self) -> Signature {
        Signature::build("each")
            .required("block", SyntaxShape::Block, "the block to run on each row")
            .switch(
                "numbered",
                "returned a numbered item ($it.index and $it.item)",
                Some('n'),
            )
    }

    fn usage(&self) -> &str {
        "Run a block on each row of the table."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        each(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Echo the sum of each row",
                example: "echo [[1 2] [3 4]] | each { echo $it | math sum }",
                result: None,
            },
            Example {
                description: "Echo the square of each integer",
                example: "echo [1 2 3] | each { echo ($it * $it) }",
                result: Some(vec![
                    UntaggedValue::int(1).into(),
                    UntaggedValue::int(4).into(),
                    UntaggedValue::int(9).into(),
                ]),
            },
            Example {
                description: "Number each item and echo a message",
                example:
                    "echo ['bob' 'fred'] | each --numbered { echo $\"($it.index) is ($it.item)\" }",
                result: Some(vec![Value::from("0 is bob"), Value::from("1 is fred")]),
            },
            Example {
                description: "Name the block variable that each uses",
                example: "[1, 2, 3] | each {|x| $x + 100}",
                result: Some(vec![
                    UntaggedValue::int(101).into(),
                    UntaggedValue::int(102).into(),
                    UntaggedValue::int(103).into(),
                ]),
            },
        ]
    }
}

pub fn process_row(
    captured_block: Arc<CapturedBlock>,
    context: Arc<EvaluationContext>,
    input: Value,
    external_redirection: ExternalRedirection,
) -> Result<OutputStream, ShellError> {
    let input_clone = input.clone();
    // When we process a row, we need to know whether the block wants to have the contents of the row as
    // a parameter to the block (so it gets assigned to a variable that can be used inside the block) or
    // if it wants the contents as as an input stream

    let input_stream = if !captured_block.block.params.positional.is_empty() {
        InputStream::empty()
    } else {
        vec![Ok(input_clone)].into_iter().into_input_stream()
    };

    context.scope.enter_scope();
    context.scope.add_vars(&captured_block.captured.entries);

    if let Some((arg, _)) = captured_block.block.params.positional.first() {
        context.scope.add_var(arg.name(), input);
    } else {
        context.scope.add_var("$it", input);
    }

    let result = run_block(
        &captured_block.block,
        &context,
        input_stream,
        external_redirection,
    );

    context.scope.exit_scope();

    result
}

pub(crate) fn make_indexed_item(index: usize, item: Value) -> Value {
    let mut dict = TaggedDictBuilder::new(item.tag());
    dict.insert_untagged("index", UntaggedValue::int(index as i64));
    dict.insert_value("item", item);

    dict.into_value()
}

fn each(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let context = Arc::new(args.context.clone());
    let external_redirection = args.call_info.args.external_redirection;

    let block: CapturedBlock = args.req(0)?;
    let numbered: bool = args.has_flag("numbered");

    let block = Arc::new(block);

    if numbered {
        Ok(args
            .input
            .enumerate()
            .flat_map(move |input| {
                let block = block.clone();
                let context = context.clone();
                let row = make_indexed_item(input.0, input.1);

                match process_row(block, context, row, external_redirection) {
                    Ok(s) => s,
                    Err(e) => OutputStream::one(Value::error(e)),
                }
            })
            .into_output_stream())
    } else {
        Ok(args
            .input
            .flat_map(move |input| {
                let block = block.clone();
                let context = context.clone();

                match process_row(block, context, input, external_redirection) {
                    Ok(s) => s,
                    Err(e) => OutputStream::one(Value::error(e)),
                }
            })
            .into_output_stream())
    }
}

#[cfg(test)]
mod tests {
    use super::Each;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Each {})
    }
}
