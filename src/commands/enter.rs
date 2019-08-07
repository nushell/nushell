use crate::commands::command::CommandAction;
use crate::errors::ShellError;
use crate::prelude::*;

pub fn enter(args: CommandArgs) -> Result<OutputStream, ShellError> {
    if args.len() == 0 {
        return Err(ShellError::labeled_error(
            "First requires an amount",
            "needs parameter",
            args.call_info.name_span,
        ));
    }

    let location = args.expect_nth(0)?.as_string()?;

    Ok(vec![Ok(ReturnSuccess::Action(CommandAction::Enter(location)))].into())
}
