use crate::commands::classified::block::run_block;
use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;

use futures::stream::once;
use nu_errors::ShellError;
use nu_protocol::{
    hir::Block, hir::Expression, hir::SpannedExpression, hir::Synthetic, Scope, Signature,
    SyntaxShape, TaggedDictBuilder, UntaggedValue, Value,
};
use nu_source::Tagged;

pub struct Each;

#[derive(Deserialize)]
pub struct EachArgs {
    block: Block,
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

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        each(args, registry).await
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

fn is_expanded_it_usage(head: &SpannedExpression) -> bool {
    matches!(&*head, SpannedExpression {
        expr: Expression::Synthetic(Synthetic::String(s)),
        ..
    } if s == "expanded-each")
}

pub async fn process_row(
    block: Arc<Block>,
    scope: Arc<Scope>,
    head: Arc<Box<SpannedExpression>>,
    mut context: Arc<Context>,
    input: Value,
) -> Result<OutputStream, ShellError> {
    let input_clone = input.clone();
    let input_stream = if is_expanded_it_usage(&head) {
        InputStream::empty()
    } else {
        once(async { Ok(input_clone) }).to_input_stream()
    };
    Ok(run_block(
        &block,
        Arc::make_mut(&mut context),
        input_stream,
        &input,
        &scope.vars,
        &scope.env,
    )
    .await?
    .to_output_stream())
}

pub(crate) fn make_indexed_item(index: usize, item: Value) -> Value {
    let mut dict = TaggedDictBuilder::new(item.tag());
    dict.insert_untagged("index", UntaggedValue::int(index));
    dict.insert_value("item", item);

    dict.into_value()
}

async fn each(
    raw_args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let head = Arc::new(raw_args.call_info.args.head.clone());
    let scope = Arc::new(raw_args.call_info.scope.clone());
    let context = Arc::new(Context::from_raw(&raw_args, &registry));
    let (each_args, input): (EachArgs, _) = raw_args.process(&registry).await?;
    let block = Arc::new(each_args.block);

    if each_args.numbered.item {
        Ok(input
            .enumerate()
            .then(move |input| {
                let block = block.clone();
                let scope = scope.clone();
                let head = head.clone();
                let context = context.clone();
                let row = make_indexed_item(input.0, input.1);

                async {
                    match process_row(block, scope, head, context, row).await {
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
                let scope = scope.clone();
                let head = head.clone();
                let context = context.clone();

                async {
                    match process_row(block, scope, head, context, input).await {
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

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Each {})
    }
}
