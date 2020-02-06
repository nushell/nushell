use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use log::trace;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, UntaggedValue};

pub struct Lines;

impl WholeStreamCommand for Lines {
    fn name(&self) -> &str {
        "lines"
    }

    fn signature(&self) -> Signature {
        Signature::build("lines")
    }

    fn usage(&self) -> &str {
        "Split single string into rows, one per line."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        lines(args, registry)
    }
}

// TODO: "Amount remaining" wrapper

fn lines(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let tag = args.name_tag();
    let name_span = tag.span;
    let input = args.input;

    let stream = input
        .values
        .map(move |v| {
            if let Ok(s) = v.as_string() {
                let split_result: Vec<_> = s.lines().filter(|s| s.trim() != "").collect();

                trace!("split result = {:?}", split_result);

                let result = split_result
                    .into_iter()
                    .map(|s| {
                        ReturnSuccess::value(
                            UntaggedValue::Primitive(Primitive::Line(s.into()))
                                .into_untagged_value(),
                        )
                    })
                    .collect::<Vec<_>>();

                futures::stream::iter(result)
            } else {
                let value_span = v.tag.span;

                futures::stream::iter(vec![Err(ShellError::labeled_error_with_secondary(
                    "Expected a string from pipeline",
                    "requires string input",
                    name_span,
                    "value originates from here",
                    value_span,
                ))])
            }
        })
        .flatten();

    Ok(stream.to_output_stream())
}
