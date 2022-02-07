use crate::prelude::*;
use nu_engine::{evaluate_baseline_expr, run_block};
use nu_engine::{FromValue, WholeStreamCommand};

use nu_errors::ShellError;
use nu_protocol::{
    hir::{CapturedBlock, ExternalRedirection},
    Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue, Value,
};

pub struct ForIn;

impl WholeStreamCommand for ForIn {
    fn name(&self) -> &str {
        "for"
    }

    fn signature(&self) -> Signature {
        Signature::build("for")
            .required("var", SyntaxShape::String, "the name of the variable")
            .required("in", SyntaxShape::String, "the word 'in'")
            .required("value", SyntaxShape::Any, "the value we want to iterate")
            .required("block", SyntaxShape::Block, "the block to run on each item")
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
        for_in(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Echo the square of each integer",
                example: "for x in [1 2 3] { $x * $x }",
                result: Some(vec![
                    UntaggedValue::int(1).into(),
                    UntaggedValue::int(4).into(),
                    UntaggedValue::int(9).into(),
                ]),
            },
            Example {
                description: "Work with elements of a range",
                example: "for $x in 1..3 { $x }",
                result: Some(vec![
                    UntaggedValue::int(1).into(),
                    UntaggedValue::int(2).into(),
                    UntaggedValue::int(3).into(),
                ]),
            },
            Example {
                description: "Number each item and echo a message",
                example: "for $it in ['bob' 'fred'] --numbered { $\"($it.index) is ($it.item)\" }",
                result: Some(vec![Value::from("0 is bob"), Value::from("1 is fred")]),
            },
        ]
    }
}

pub fn process_row(
    captured_block: &CapturedBlock,
    context: &EvaluationContext,
    input: Value,
    var_name: &str,
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

    context.scope.add_var(var_name, input);

    let result = run_block(
        &captured_block.block,
        context,
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

fn for_in(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let context = args.context.clone();
    let external_redirection = args.call_info.args.external_redirection;
    //
    let numbered: bool = args.call_info.switch_present("numbered");
    let positional = args
        .call_info
        .args
        .positional
        .expect("Internal error: type checker should require args");

    let var_name = positional[0].var_name()?;
    let rhs = evaluate_baseline_expr(&positional[2], &context)?;

    let block: CapturedBlock =
        FromValue::from_value(&evaluate_baseline_expr(&positional[3], &context)?)?;

    let input = crate::commands::core_commands::echo::expand_value_to_stream(rhs);

    if numbered {
        Ok(input
            .enumerate()
            .flat_map(move |input| {
                let row = make_indexed_item(input.0, input.1);

                match process_row(&block, &context, row, &var_name, external_redirection) {
                    Ok(s) => s,
                    Err(e) => OutputStream::one(Value::error(e)),
                }
            })
            .into_output_stream())
    } else {
        Ok(input
            .flat_map(move |input| {
                let block = block.clone();

                match process_row(&block, &context, input, &var_name, external_redirection) {
                    Ok(s) => s,
                    Err(e) => OutputStream::one(Value::error(e)),
                }
            })
            .into_output_stream())
    }
}

#[cfg(test)]
mod tests {
    use super::ForIn;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(ForIn {})
    }
}
