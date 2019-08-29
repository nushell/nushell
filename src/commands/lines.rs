use crate::commands::WholeStreamCommand;
use crate::errors::ShellError;
use crate::object::{Primitive, Value};
use crate::prelude::*;
use log::trace;

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
    let span = args.name_span();
    let input = args.input;

    let input: InputStream = trace_stream!(target: "nu::trace_stream::lines", "input" = input);

    let stream = input
        .values
        .map(move |v| match v.item {
            Value::Primitive(Primitive::String(s)) => {
                let split_result: Vec<_> = s.lines().filter(|s| s.trim() != "").collect();

                trace!("split result = {:?}", split_result);

                let mut result = VecDeque::new();
                for s in split_result {
                    result.push_back(ReturnSuccess::value(
                        Value::Primitive(Primitive::String(s.into())).tagged_unknown(),
                    ));
                }
                result
            }
            _ => {
                let mut result = VecDeque::new();
                result.push_back(Err(ShellError::labeled_error_with_secondary(
                    "Expected a string from pipeline",
                    "requires string input",
                    span,
                    "value originates from here",
                    v.span(),
                )));
                result
            }
        })
        .flatten();

    Ok(stream.to_output_stream())
}
