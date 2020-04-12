use crate::commands::PerItemCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{CallInfo, ReturnSuccess, Signature, SyntaxShape, Value};

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
    }
}
