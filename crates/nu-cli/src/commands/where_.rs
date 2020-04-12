use crate::commands::classified::pipeline::run_pipeline;
use crate::commands::PerItemCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_parser::ClassifiedPipeline;
use nu_protocol::{
    CallInfo, Primitive, ReturnSuccess, Scope, Signature, SyntaxShape, UntaggedValue, Value,
};

pub struct Where;

impl PerItemCommand for Where {
    fn name(&self) -> &str {
        "where"
    }

    fn signature(&self) -> Signature {
        Signature::build("where").required(
            "condition",
            SyntaxShape::Condition,
            "the condition that must match",
        )
    }

    fn usage(&self) -> &str {
        "Filter table to match the condition."
    }

    fn run(
        &self,
        call_info: &CallInfo,
        _registry: &CommandRegistry,
        _raw_args: &RawCommandArgs,
        input: Value,
    ) -> Result<OutputStream, ShellError> {
        let condition = call_info.args.expect_nth(0)?;

        let stream = match condition.as_bool() {
            Ok(b) => {
                if b {
                    VecDeque::from(vec![Ok(ReturnSuccess::Value(input))])
                } else {
                    VecDeque::new()
                }
            }
            Err(e) => return Err(e),
        };

        Ok(stream.into())

        /*
        let stream = async_stream! {
            match condition {
                Value {
                    value: UntaggedValue::Block(block),
                    tag
                } => {
                    let mut context = Context::from_raw(&raw_args, &registry);
                    let result = run_pipeline(
                        ClassifiedPipeline::new(block.clone(), None),
                        &mut context,
                        None,
                    ).await;

                    match result {
                        Ok(Some(v)) => {
                            let results: Vec<Value> = v.collect().await;

                            if results.len() == 1 {
                                match results[0] {
                                    Value { value: UntaggedValue::Primitive(Primitive::Boolean(b)), ..} => {
                                        if b {
                                            yield Ok(ReturnSuccess::Value(input));
                                        }
                                    }
                                    _ => {
                                        yield Err(ShellError::labeled_error(
                                            "Expected a condition",
                                            "where needs a condition",
                                            tag,
                                        ));
                                    }
                                }
                            } else {
                                yield Err(ShellError::labeled_error(
                                    "Expected a condition",
                                    "where needs a condition",
                                    tag,
                                ));
                            }
                        }
                        Ok(None) => {
                            yield Err(ShellError::labeled_error(
                                "Expected a condition",
                                "where needs a condition",
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
                        "Expected a condition",
                        "where needs a condition",
                        tag,
                    ))
                }
            };
        };
        */

        //Ok(stream.to_output_stream())
    }
}
