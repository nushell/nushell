use crate::commands::WholeStreamCommand;
use crate::errors::ShellError;
use crate::format::TableView;
use crate::prelude::*;
use futures_async_stream::async_stream_block;

pub struct Table;

#[derive(Deserialize)]
pub struct TableArgs {}

impl WholeStreamCommand for Table {
    fn name(&self) -> &str {
        "table"
    }

    fn signature(&self) -> Signature {
        Signature::build("table")
    }

    fn usage(&self) -> &str {
        "View the contents of the pipeline as a table."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, table)?.run()
    }
}

pub fn table(_args: TableArgs, context: RunnableContext) -> Result<OutputStream, ShellError> {
    let stream = async_stream_block! {
        let input: Vec<Tagged<Value>> = context.input.into_vec().await;
        if input.len() > 0 {
            let mut host = context.host.lock().unwrap();
            let view = TableView::from_list(&input);
            if let Some(view) = view {
                handle_unexpected(&mut *host, |host| crate::format::print_view(&view, host));
            }
        }
    };

    Ok(OutputStream::new(stream))
}
