use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Primitive, Signature, UntaggedValue, Value};

pub struct SubCommand;

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "error make"
    }

    fn signature(&self) -> Signature {
        Signature::build("error make")
    }

    fn usage(&self) -> &str {
        "Create an error."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let input = args.input;

        Ok(input
            .map(|value| {
                make_error(&value)
                    .map(|err| UntaggedValue::Error(err).into_value(value.tag()))
                    .unwrap_or_else(|| {
                        UntaggedValue::Error(ShellError::untagged_runtime_error(
                            "Creating error value not supported.",
                        ))
                        .into_value(value.tag())
                    })
            })
            .into_output_stream())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Creates a labeled error",
            example: r#"[
  [          msg,                 labels,                      span];
  ["The message", "Helpful message here", ([[start, end]; [0, 141]])]
] | error make"#,
            result: None,
        }]
    }
}

fn make_error(value: &Value) -> Option<ShellError> {
    if let Value {
        value: UntaggedValue::Row(dict),
        ..
    } = value
    {
        let msg = dict.get_data_by_key("msg".spanned_unknown());

        let labels =
            dict.get_data_by_key("labels".spanned_unknown())
                .and_then(|table| match &table.value {
                    UntaggedValue::Table(_) => table
                        .table_entries()
                        .map(|value| value.as_string().ok())
                        .collect(),
                    UntaggedValue::Primitive(Primitive::String(label)) => {
                        Some(vec![label.to_string()])
                    }
                    _ => None,
                });

        let _anchor = dict.get_data_by_key("tag".spanned_unknown());
        let span = dict.get_data_by_key("span".spanned_unknown());

        if msg.is_none() || labels.is_none() || span.is_none() {
            return None;
        }

        let msg = msg.and_then(|msg| msg.as_string().ok());

        if let Some(labels) = labels {
            if labels.is_empty() {
                return None;
            }

            return Some(ShellError::labeled_error(
                msg.expect("Message will always be present."),
                &labels[0],
                span.map(|data| match data {
                    Value {
                        value: UntaggedValue::Row(vals),
                        ..
                    } => match (vals.entries.get("start"), vals.entries.get("end")) {
                        (Some(start), Some(end)) => {
                            let start = start.as_usize().ok().unwrap_or(0);
                            let end = end.as_usize().ok().unwrap_or(0);

                            Span::new(start, end)
                        }
                        (_, _) => Span::unknown(),
                    },
                    _ => Span::unknown(),
                })
                .unwrap_or_else(Span::unknown),
            ));
        }
    }

    None
}
