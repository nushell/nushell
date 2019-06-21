use crate::commands::command::SinkCommandArgs;
use crate::errors::ShellError;
use crate::format::TableView;
use crate::prelude::*;

pub fn table(args: SinkCommandArgs) -> Result<(), ShellError> {
    if args.input.len() > 0 {
        let mut host = args.ctx.host.lock().unwrap();
        let view = TableView::from_list(&args.input);
        if let Some(view) = view {
            handle_unexpected(&mut *host, |host| crate::format::print_view(&view, host));
        }
    }

    Ok(())
}
