use crate::commands::WholeStreamCommand;
use crate::errors::ShellError;
use crate::format::TableView;
use crate::prelude::*;

pub struct Table;

impl WholeStreamCommand for Table {
    fn name(&self) -> &str {
        "table"
    }

    fn signature(&self) -> Signature {
        Signature::build("table").named("start_number", SyntaxShape::Number)
    }

    fn usage(&self) -> &str {
        "View the contents of the pipeline as a table."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        table(args, registry)
    }
}

fn table(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;

    let stream = async_stream! {
        let host = args.host.clone();
        let start_number = match args.get("start_number") {
            Some(Tagged { item: Value::Primitive(Primitive::Int(i)), .. }) => {
                i.to_usize().unwrap()
            }
            _ => {
                0
            }
        };

        let input: Vec<Tagged<Value>> = args.input.into_vec().await;
        if input.len() > 0 {
            let mut host = host.lock().unwrap();
            let view = TableView::from_list(&input, start_number);

            if let Some(view) = view {
                handle_unexpected(&mut *host, |host| crate::format::print_view(&view, host));
            }
        }
        // Needed for async_stream to type check
        if false {
            yield ReturnSuccess::value(Value::nothing().tagged_unknown());
        }
    };

    Ok(OutputStream::new(stream))
}
