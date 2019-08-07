use crate::commands::command::CommandAction;
use crate::errors::ShellError;
use crate::prelude::*;

pub fn enter(args: CommandArgs) -> Result<OutputStream, ShellError> {
    //TODO: We could also enter a value in the stream
    if args.len() == 0 {
        return Err(ShellError::labeled_error(
            "Enter requires a path",
            "needs parameter",
            args.call_info.name_span,
        ));
    }

    let location = args.expect_nth(0)?.as_string()?;

    Ok(vec![Ok(ReturnSuccess::Action(CommandAction::EnterShell(
        location,
    )))]
    .into())
}
