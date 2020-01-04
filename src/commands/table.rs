use crate::commands::WholeStreamCommand;
use crate::format::TableView;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};

pub struct Table;

impl WholeStreamCommand for Table {
    fn name(&self) -> &str {
        "table"
    }

    fn signature(&self) -> Signature {
        Signature::build("table").named(
            "start_number",
            SyntaxShape::Number,
            "row number to start viewing from",
        )
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
            Some(Value { value: UntaggedValue::Primitive(Primitive::Int(i)), .. }) => {
                if let Some(num) = i.to_usize() {
                    num
                } else {
                    yield Err(ShellError::labeled_error("Expected a row number", "expected a row number", &args.args.call_info.name_tag));
                    0
                }
            }
            _ => {
                0
            }
        };

        let input: Vec<Value> = args.input.into_vec().await;
        if input.len() > 0 {
            let mut host = host.lock();
            let view = TableView::from_list(&input, start_number);

            if let Some(view) = view {
                handle_unexpected(&mut *host, |host| crate::format::print_view(&view, host));
            }
        }
        // Needed for async_stream to type check
        if false {
            yield ReturnSuccess::value(UntaggedValue::nothing().into_value(Tag::unknown()));
        }
    };

    Ok(OutputStream::new(stream))
}
