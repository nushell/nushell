use crate::commands::classified::pipeline::run_pipeline;

use crate::commands::PerItemCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{
    hir::ClassifiedPipeline, CallInfo, CommandAction, Primitive, ReturnSuccess, Scope, Signature,
    SyntaxShape, UntaggedValue, Value,
};

pub struct Alias;

impl PerItemCommand for Alias {
    fn name(&self) -> &str {
        "alias"
    }

    fn signature(&self) -> Signature {
        Signature::build("alias")
            .required("name", SyntaxShape::String, "the name of the alias")
            .required("args", SyntaxShape::Table, "the arguments to the alias")
            .required("block", SyntaxShape::Block, "the block to run on each row")
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
            match (call_info.args.expect_nth(0)?, call_info.args.expect_nth(1)?, call_info.args.expect_nth(2)?) {
                (Value {value: UntaggedValue::Primitive(Primitive::String(name)), .. },
                Value { value: UntaggedValue::Table(list), .. },
                Value {
                    value: UntaggedValue::Block(block),
                    tag
                }) => {
                    let args: Vec<String> = list.iter().map(|x| format!("${}", x.as_string().expect("Couldn't convert to string"))).collect();
                    yield ReturnSuccess::action(CommandAction::AddAlias(name.to_string(), args, block.clone()))
                }
                _ => {
                    yield Err(ShellError::labeled_error(
                        "Expected `name [args] {block}",
                        "needs a name, args, and a block",
                        call_info.name_tag,
                    ))
                }
            };
        };

        Ok(stream.to_output_stream())
    }
}
