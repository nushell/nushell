use crate::commands::classified::block::run_block;
use crate::commands::PerItemCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;

use futures::stream::once;

use nu_errors::ShellError;
use nu_protocol::{CallInfo, ReturnSuccess, Scope, Signature, SyntaxShape, UntaggedValue, Value};

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
                    let input_clone = input.clone();
                    let input_stream = once(async { Ok(input) }).to_input_stream();

                    let result = run_block(
                        block,
                        &mut context,
                        input_stream,
                        &Scope::new(input_clone),
                    ).await;

                    match result {
                        Ok(stream) if stream.is_empty() => {
                            yield Err(ShellError::labeled_error(
                                "Expected a block",
                                "each needs a block",
                                tag,
                            ));
                        }
                        Ok(mut stream) => {
                            let errors = context.get_errors();
                            if let Some(error) = errors.first() {
                                yield Err(error.clone());
                                return;
                            }

                            while let Some(result) = stream.next().await {
                                yield Ok(ReturnSuccess::Value(result));
                            }
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
