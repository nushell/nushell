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

fn ends_with_line_ending(st: &str) -> bool {
    let mut temp = st.to_string();
    let last = temp.pop();
    if let Some(c) = last {
        c == '\n'
    } else {
        false
    }
}

fn lines(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let tag = args.name_tag();
    let name_span = tag.span;
    let mut input = args.input;

    let mut leftover = vec![];
    let mut leftover_string = String::new();
    let stream = async_stream! {
        loop {
            match input.values.next().await {
                Some(Value { value: UntaggedValue::Primitive(Primitive::String(st)), ..}) => {
                    let mut st = leftover_string.clone() + &st;
                    leftover.clear();

                    let mut lines: Vec<String> = st.lines().filter(|s| s.trim() != "").map(|x| x.to_string()).collect();
                    if !ends_with_line_ending(&st) {
                        if let Some(last) = lines.pop() {
                            leftover_string = last;
                        } else {
                            leftover_string.clear();
                        }
                    } else {
                        leftover_string.clear();
                    }

                    let success_lines: Vec<_> = lines.iter().map(|x| ReturnSuccess::value(UntaggedValue::line(x).into_untagged_value())).collect();
                    yield futures::stream::iter(success_lines)                        
                }
                Some(Value { value: UntaggedValue::Primitive(Primitive::Line(st)), ..}) => {
                    let mut st = leftover_string.clone() + &st;
                    leftover.clear();

                    let mut lines: Vec<String> = st.lines().filter(|s| s.trim() != "").map(|x| x.to_string()).collect();
                    if !ends_with_line_ending(&st) {
                        if let Some(last) = lines.pop() {
                            leftover_string = last;
                        } else {
                            leftover_string.clear();
                        }
                    } else {
                        leftover_string.clear();
                    }

                    let success_lines: Vec<_> = lines.iter().map(|x| ReturnSuccess::value(UntaggedValue::line(x).into_untagged_value())).collect();
                    yield futures::stream::iter(success_lines)                        
                }
                Some( Value { tag: value_span, ..}) => {
                    yield futures::stream::iter(vec![Err(ShellError::labeled_error_with_secondary(
                        "Expected a string from pipeline",
                        "requires string input",
                        name_span,
                        "value originates from here",
                        value_span,
                    ))]);
                }
                None => {
                    if !leftover.is_empty() {
                        let mut st = leftover_string.clone();
                        if let Ok(extra) = String::from_utf8(leftover) {
                            st.push_str(&extra);
                        }
                        yield futures::stream::iter(vec![ReturnSuccess::value(UntaggedValue::string(st).into_untagged_value())])
                    }
                    break;
                }
            }
        }
        yield futures::stream::iter(vec![ReturnSuccess::value(UntaggedValue::string(leftover_string).into_untagged_value())]);
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
