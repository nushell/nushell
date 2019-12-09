use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use nu_value_ext::ValueExt;

#[derive(Deserialize)]
struct DefaultArgs {
    column: Tagged<String>,
    value: Value,
}

pub struct Default;

impl WholeStreamCommand for Default {
    fn name(&self) -> &str {
        "default"
    }

    fn signature(&self) -> Signature {
        Signature::build("default")
            .required("column name", SyntaxShape::String, "the name of the column")
            .required(
                "column value",
                SyntaxShape::Any,
                "the value of the column to default",
            )
    }

    fn usage(&self) -> &str {
        "Sets a default row's column if missing."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, default)?.run()
    }
}

fn default(
    DefaultArgs { column, value }: DefaultArgs,
    RunnableContext { input, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let stream = input
        .values
        .map(move |item| {
            let mut result = VecDeque::new();

            let should_add = match item {
                Value {
                    value: UntaggedValue::Row(ref r),
                    ..
                } => r.get_data(&column.item).borrow().is_none(),
                _ => false,
            };

            if should_add {
                match item.insert_data_at_path(&column.item, value.clone()) {
                    Some(new_value) => result.push_back(ReturnSuccess::value(new_value)),
                    None => result.push_back(ReturnSuccess::value(item)),
                }
            } else {
                result.push_back(ReturnSuccess::value(item));
            }
            result
        })
        .flatten();

    Ok(stream.to_output_stream())
}
