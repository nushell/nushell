use crate::commands::each::process_row;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{hir::Block, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use serde::Deserialize;

pub struct EachGroup;

#[derive(Deserialize)]
pub struct EachGroupArgs {
    block: Block,
    group_size: Tagged<usize>,
    //numbered: Tagged<bool>,
}

#[async_trait]
impl WholeStreamCommand for EachGroup {
    fn name(&self) -> &str {
        "each group"
    }

    fn signature(&self) -> Signature {
        Signature::build("each group")
            .required(
                "block",
                SyntaxShape::Block,
                "the block to run on each group",
            )
            .required("group_size", SyntaxShape::Int, "the size of each group")
    }

    fn usage(&self) -> &str {
        "Runs a block on groups of `group_size` rows of a table at a time."
    }

    async fn run(
        &self,
        raw_args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let registry = registry.clone();
        let head = Arc::new(raw_args.call_info.args.head.clone());
        let scope = Arc::new(raw_args.call_info.scope.clone());
        let context = Arc::new(Context::from_raw(&raw_args, &registry));
        let (each_args, input): (EachGroupArgs, _) = raw_args.process(&registry).await?;
        let block = Arc::new(each_args.block);

        Ok(input
            .chunks(each_args.group_size.item)
            .then(move |input| {
                let block = block.clone();
                let scope = scope.clone();
                let head = head.clone();
                let context = context.clone();

                let value = Value {
                    value: UntaggedValue::Table(input),
                    tag: Tag::unknown(),
                };

                async {
                    match process_row(block, scope, head, context, value).await {
                        Ok(s) => s,
                        //Ok(s) => {
                        //let vec = s
                        //.filter_map(|x| async { x.unwrap().raw_value() })
                        //.collect::<Vec<Value>>()
                        //.await;
                        //let val = Value {
                        //value: UntaggedValue::Table(vec),
                        //tag: Tag::unknown(),
                        //};
                        //OutputStream::one(Ok(ReturnSuccess::Value(val)))
                        //}
                        Err(e) => OutputStream::one(Err(e)),
                    }
                }
            })
            .flatten()
            .to_output_stream())
    }
}
