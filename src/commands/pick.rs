use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::data::base::select_fields;
use crate::prelude::*;
use futures_util::pin_mut;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, ReturnValue, Signature, SyntaxShape, UntaggedValue};
use nu_source::Tagged;

#[derive(Deserialize)]
struct PickArgs {
    rest: Vec<Tagged<String>>,
}

pub struct Pick;

impl WholeStreamCommand for Pick {
    fn name(&self) -> &str {
        "pick"
    }

    fn signature(&self) -> Signature {
        Signature::build("pick").rest(SyntaxShape::Any, "the columns to select from the table")
    }

    fn usage(&self) -> &str {
        "Down-select table to only these columns."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, pick)?.run()
    }
}

fn pick(
    PickArgs { rest: fields }: PickArgs,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    if fields.is_empty() {
        return Err(ShellError::labeled_error(
            "Pick requires columns to pick",
            "needs parameter",
            name,
        ));
    }

    let fields: Vec<_> = fields.iter().map(|f| f.item.clone()).collect();

    let stream = async_stream! {
        let values = input.values;
        pin_mut!(values);

        let mut empty = true;

        while let Some(value) = values.next().await {
            let new_value = select_fields(&value, &fields, value.tag.clone());

            if let UntaggedValue::Row(dict) = &new_value.value {
                if dict
                    .entries
                    .values()
                    .any(|v| v.value != UntaggedValue::Primitive(Primitive::Nothing))
                {
                    empty = false;
                    yield ReturnSuccess::value(new_value);
                }
            }
        }

        if empty {
            yield Err(ShellError::labeled_error("None of the columns were found in the input", "could not find columns given", name));
        }
    };

    let stream: BoxStream<'static, ReturnValue> = stream.boxed();

    Ok(stream.to_output_stream())
}
