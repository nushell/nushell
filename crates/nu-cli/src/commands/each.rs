use crate::commands::classified::pipeline::run_pipeline;

use crate::commands::PerItemCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{
    hir::ClassifiedPipeline, CallInfo, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};

pub struct Each;

impl PerItemCommand for Each {
    fn name(&self) -> &str {
        "each"
    }

    fn signature(&self) -> Signature {
        Signature::build("each").required(
            "block",
            SyntaxShape::Block,
            "the block to run on each row",
        )
    }

    fn usage(&self) -> &str {
        "Run a block on each row of the table."
    }

    fn run(
        &self,
        call_info: &CallInfo,
        registry: &CommandRegistry,
        raw_args: &RawCommandArgs,
        input: Value,
    ) -> Result<OutputStream, ShellError> {
        let call_info = call_info.clone();
        let registry = registry.clone();
        let raw_args = raw_args.clone();
        let stream = async_stream! {
            match call_info.args.expect_nth(0)? {
                Value {
                    value: UntaggedValue::Block(block),
                    tag
                } => {
                    let mut context = Context::from_raw(&raw_args, &registry);
                    let input_stream = async_stream! {
                        yield Ok(input.clone())
                    }.to_input_stream();

                    let result = run_pipeline(
                        ClassifiedPipeline::new(block.clone(), None),
                        &mut context,
                        Some(input_stream),
                    ).await;

                    match result {
                        Ok(Some(v)) => {
                            let results: Vec<Value> = v.collect().await;

                            for result in results {
                                yield Ok(ReturnSuccess::Value(result));
                            }
                        }
                        Ok(None) => {
                            yield Err(ShellError::labeled_error(
                                "Expected a block",
                                "each needs a block",
                                tag,
                            ));
                        }
                        Err(e) => {
                            yield Err(e);
                        }
                    }
                }
                Value { tag, .. } => {
                    yield Err(ShellError::labeled_error(
                        "Expected a block",
                        "each needs a block",
                        tag,
                    ))
                }
            };
        };

        Ok(stream.to_output_stream())
    }
}
