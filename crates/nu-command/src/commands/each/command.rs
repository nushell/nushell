use crate::prelude::*;
use nu_engine::run_block;
use nu_engine::WholeStreamCommand;

use futures::stream::once;
use nu_errors::ShellError;
use nu_protocol::{
    hir::CapturedBlock, Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue, Value,
};
use nu_source::Tagged;

pub struct Each;

#[derive(Deserialize)]
pub struct EachArgs {
    block: CapturedBlock,
    numbered: Tagged<bool>,
}

#[async_trait]
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

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        each(args).await
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
                example: "echo [1 2 3] | each { echo $(= $it * $it) }",
                result: Some(vec![
                    UntaggedValue::int(1).into(),
                    UntaggedValue::int(4).into(),
                    UntaggedValue::int(9).into(),
                ]),
            },
            Example {
                description: "Number each item and echo a message",
                example:
                    "echo ['bob' 'fred'] | each --numbered { echo `{{$it.index}} is {{$it.item}}` }",
                result: Some(vec![Value::from("0 is bob"), Value::from("1 is fred")]),
            },
        ]
    }
}

pub async fn process_row(
    captured_block: Arc<Box<CapturedBlock>>,
    context: Arc<EvaluationContext>,
    input: Value,
) -> Result<OutputStream, ShellError> {
    let input_clone = input.clone();
    // When we process a row, we need to know whether the block wants to have the contents of the row as
    // a parameter to the block (so it gets assigned to a variable that can be used inside the block) or
    // if it wants the contents as as an input stream

    let input_stream = if !captured_block.block.params.positional.is_empty() {
        InputStream::empty()
    } else {
        once(async { Ok(input_clone) }).to_input_stream()
    };

    context.scope.enter_scope();
    context.scope.add_vars(&captured_block.captured.entries);

    if !captured_block.block.params.positional.is_empty() {
        // FIXME: add check for more than parameter, once that's supported
        context
            .scope
            .add_var(captured_block.block.params.positional[0].0.name(), input);
    } else {
        context.scope.add_var("$it", input);
    }

    let result = run_block(&captured_block.block, &*context, input_stream).await;

    context.scope.exit_scope();

    Ok(result?.to_output_stream())
}

pub(crate) fn make_indexed_item(index: usize, item: Value) -> Value {
    let mut dict = TaggedDictBuilder::new(item.tag());
    dict.insert_untagged("index", UntaggedValue::int(index));
    dict.insert_value("item", item);

    dict.into_value()
}

async fn each(raw_args: CommandArgs) -> Result<OutputStream, ShellError> {
    let context = Arc::new(EvaluationContext::from_args(&raw_args));

    let (each_args, input): (EachArgs, _) = raw_args.process().await?;
    let block = Arc::new(Box::new(each_args.block));

    if each_args.numbered.item {
        Ok(input
            .enumerate()
            .then(move |input| {
                let block = block.clone();
                let context = context.clone();
                let row = make_indexed_item(input.0, input.1);

                async {
                    match process_row(block, context, row).await {
                        Ok(s) => s,
                        Err(e) => OutputStream::one(Err(e)),
                    }
                }
            })
            .flatten()
            .to_output_stream())
    } else {
        Ok(input
            .then(move |input| {
                let block = block.clone();
                let context = context.clone();

                async {
                    match process_row(block, context, input).await {
                        Ok(s) => s,
                        Err(e) => OutputStream::one(Err(e)),
                    }
                }
            })
            .flatten()
            .to_output_stream())
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
