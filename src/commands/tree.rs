use crate::commands::command::SinkCommandArgs;
use crate::errors::ShellError;
use crate::format::TreeView;
use crate::prelude::*;

pub fn tree(args: SinkCommandArgs) -> Result<(), ShellError> {
    if args.input.len() > 0 {
        let mut host = args.ctx.host.lock().unwrap();
        for i in args.input.iter() {
            let view = TreeView::from_value(&i);
            handle_unexpected(&mut *host, |host| crate::format::print_view(&view, host));
            host.stdout("");
        }
    }

    Ok(())
}
