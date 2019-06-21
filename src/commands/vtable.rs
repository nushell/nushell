use crate::commands::command::SinkCommandArgs;
use crate::errors::ShellError;
use crate::format::VTableView;
use crate::prelude::*;

pub fn vtable(args: SinkCommandArgs) -> Result<(), ShellError> {
    if args.input.len() > 0 {
        let mut host = args.ctx.host.lock().unwrap();
        let view = VTableView::from_list(&args.input);
        if let Some(view) = view {
            handle_unexpected(&mut *host, |host| crate::format::print_view(&view, host));
        }
    }

    Ok(())
}
