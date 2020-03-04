use crate::commands::WholeStreamCommand;
use crate::prelude::*;
//use log::trace;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, UntaggedValue, Value};

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
    //let name_span = tag.span;
    let mut input = args.input;

    let mut leftover = vec![];
    let stream = async_stream! {
        loop {
            match input.values.next().await {
                Some(Value { value: UntaggedValue::Primitive(Primitive::Binary(mut binary)), ..}) => {
                    // Let's try to convert the block of u8 to a string. If we're not the first buffer, there may be leftover bytes
                    // that weren't successfully converted to utf-8 before. Let's start with those and then add the new buffer to it.
                    leftover.append(&mut binary);
                    match String::from_utf8(leftover.clone()) {
                        Ok(st) => {
                            leftover.clear();
                            yield futures::stream::iter(vec![ReturnSuccess::value(UntaggedValue::string(st).into_untagged_value())])                        
                        }
                        Err(err) => {
                            let mut partial = Vec::new();
                            partial.clone_from_slice(&leftover[0..err.utf8_error().valid_up_to()]);
                            let st = String::from_utf8(partial).unwrap();
                            leftover.drain(0..err.utf8_error().valid_up_to());
                            yield futures::stream::iter(vec![ReturnSuccess::value(UntaggedValue::string(st).into_untagged_value())])
                        }
                    }
                }
                Some(v) => {
                    leftover.clear();
                    yield futures::stream::iter(vec![ReturnSuccess::value(v)])
                }
                None => {
                    if !leftover.is_empty() {
                        let st = String::from_utf8(leftover).unwrap();
                        yield futures::stream::iter(vec![ReturnSuccess::value(UntaggedValue::string(st).into_untagged_value())])
                    }
                    break;
                }
            }
        }
    }
    .flatten();

    // let stream = input
    //     .values
    //     .map(move |v| {
    //         if let Ok(s) = v.as_string() {
    //             let split_result: Vec<_> = s.lines().filter(|s| s.trim() != "").collect();

    //             trace!("split result = {:?}", split_result);

    //             let result = split_result
    //                 .into_iter()
    //                 .map(|s| {
    //                     ReturnSuccess::value(
    //                         UntaggedValue::Primitive(Primitive::Line(s.into()))
    //                             .into_untagged_value(),
    //                     )
    //                 })
    //                 .collect::<Vec<_>>();

    //             futures::stream::iter(result)
    //         } else if let Value {
    //             value: UntaggedValue::Primitive(Primitive::Binary(_)),
    //             ..
    //         } = v
    //         {
    //             futures::stream::iter(vec![ReturnSuccess::value(v)])
    //         } else {
    //             let value_span = v.tag.span;

    //             futures::stream::iter(vec![Err(ShellError::labeled_error_with_secondary(
    //                 "Expected a string from pipeline",
    //                 "requires string input",
    //                 name_span,
    //                 "value originates from here",
    //                 value_span,
    //             ))])
    //         }
    //     })
    //     .flatten();

    Ok(stream.to_output_stream())
}
