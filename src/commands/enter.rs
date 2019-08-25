use crate::commands::command::CommandAction;
use crate::commands::PerItemCommand;
use crate::errors::ShellError;
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
        call_info: &CallInfo,
        _registry: &registry::CommandRegistry,
        _shell_manager: &ShellManager,
        _input: Tagged<Value>,
    ) -> Result<OutputStream, ShellError> {
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
