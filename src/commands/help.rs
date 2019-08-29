use crate::commands::command::CommandAction;
use crate::commands::PerItemCommand;
use crate::errors::ShellError;
use crate::parser::registry;
use crate::prelude::*;

pub struct Help;

impl PerItemCommand for Help {
    fn name(&self) -> &str {
        "help"
    }

    fn signature(&self) -> registry::Signature {
        Signature::build("help").rest(SyntaxType::Any)
    }

    fn usage(&self) -> &str {
        "Display help information about commands."
    }

    fn run(
        &self,
        call_info: &CallInfo,
        _registry: &CommandRegistry,
        _raw_args: &RawCommandArgs,
        _input: Tagged<Value>,
    ) -> Result<OutputStream, ShellError> {
        let span = call_info.name_span;

        if call_info.args.len() == 0 {
            return Ok(
                vec![
                Ok(ReturnSuccess::Action(
                    CommandAction::EnterHelpShell(
                        Tagged::from_simple_spanned_item(Value::nothing(), span)
                    )))].into()
            )
        }

        match call_info.args.expect_nth(0)? {
            Tagged {
                item: Value::Primitive(Primitive::String(document)),
                ..
            } => Ok(vec![Ok(ReturnSuccess::Action(CommandAction::EnterHelpShell(
                Tagged::from_simple_spanned_item(Value::string(document), span)
            )))]
            .into()),
            x => Ok(
                vec![Ok(ReturnSuccess::Action(CommandAction::EnterHelpShell(x.clone())))]
                .into(),
            ),
        }
    }
}
