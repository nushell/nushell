use crate::commands::command::CommandAction;
use crate::commands::{PerItemCommand, RawCommandArgs};
use crate::errors::ShellError;
use crate::evaluate::Scope;
use crate::parser::registry;
use crate::prelude::*;

pub struct Enter;

impl PerItemCommand for Enter {
    fn name(&self) -> &str {
        "enter"
    }

    fn signature(&self) -> registry::Signature {
        Signature::build("enter").required("location", SyntaxType::Block)
    }

    fn run(
        &self,
        args: RawCommandArgs,
        registry: &registry::CommandRegistry,
        input: Tagged<Value>,
    ) -> Result<VecDeque<ReturnValue>, ShellError> {
        let call_info = args
            .call_info
            .evaluate(registry, &Scope::it_value(input))
            .unwrap();

        match call_info.args.expect_nth(0)? {
            Tagged {
                item: Value::Primitive(Primitive::String(location)),
                ..
            } => Ok(vec![Ok(ReturnSuccess::Action(CommandAction::EnterShell(
                location.to_string(),
            )))]
            .into()),
            x => Ok(
                vec![Ok(ReturnSuccess::Action(CommandAction::EnterValueShell(
                    x.clone(),
                )))]
                .into(),
            ),
        }
    }
}
