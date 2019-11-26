use crate::commands::WholeStreamCommand;
use crate::data::value;
use crate::format::TableView;
use crate::prelude::*;
use nu_protocol::{
    Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_errors::ShellError;

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
                i.to_usize().unwrap()
            }
            _ => {
                0
            }
        };

        let input: Vec<Value> = args.input.into_vec().await;
        if input.len() > 0 {
            let mut host = host.lock().unwrap();
            let view = TableView::from_list(&input, start_number);

            if let Some(view) = view {
                handle_unexpected(&mut *host, |host| crate::format::print_view(&view, host));
            }
        }
        // Needed for async_stream to type check
        if false {
            yield ReturnSuccess::value(value::nothing().into_value(Tag::unknown()));
        }
    };

    Ok(OutputStream::new(stream))
}
