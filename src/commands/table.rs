use crate::commands::StaticCommand;
use crate::errors::ShellError;
use crate::format::TableView;
use crate::prelude::*;
use futures_async_stream::async_stream_block;

pub struct Table;

#[derive(Deserialize)]
pub struct TableArgs {
    full: bool,
}

impl StaticCommand for Table {
    fn name(&self) -> &str {
        "table"
    }
    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, table)?.run()
    }
    fn signature(&self) -> Signature {
        Signature::build("table").switch("full")
    }
}

pub fn table(
    TableArgs { full }: TableArgs,
    context: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let stream = async_stream_block! {
        let input: Vec<Tagged<Value>> = context.input.into_vec().await;
        if input.len() > 0 {
            let mut host = context.host.lock().unwrap();
            let view = TableView::from_list(&input, full);
            if let Some(view) = view {
                handle_unexpected(&mut *host, |host| crate::format::print_view(&view, host));
            }
        }
    };

    Ok(OutputStream::new(stream))
}
