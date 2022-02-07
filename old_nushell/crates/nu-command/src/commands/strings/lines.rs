use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, UntaggedValue, Value};
use parking_lot::Mutex;

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

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        lines(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Split multi-line string into lines",
            example: r#"^echo "two\nlines" | lines"#,
            result: None,
        }]
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

fn lines(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let leftover_string = Arc::new(Mutex::new(String::new()));
    let tag = args.name_tag();
    let name_span = tag.span;

    let eos = vec![UntaggedValue::Primitive(Primitive::EndOfStream).into_untagged_value()];

    Ok(args
        .input
        .chain(eos)
        .filter_map(move |item| {
            let leftover_string = leftover_string.clone();
            match item {
                Value {
                    value: UntaggedValue::Primitive(Primitive::String(st)),
                    ..
                } => {
                    let mut leftover_string = leftover_string.lock();

                    let mut buffer = leftover_string.clone();
                    buffer.push_str(&st);

                    let mut lines: Vec<String> = buffer.lines().map(|x| x.to_string()).collect();

                    leftover_string.clear();

                    if !ends_with_line_ending(&st) {
                        if let Some(last) = lines.pop() {
                            leftover_string.push_str(&last);
                        }
                    }

                    if !lines.is_empty() {
                        let success_lines: Vec<_> = lines
                            .iter()
                            .map(|x| {
                                ReturnSuccess::value(UntaggedValue::string(x).into_untagged_value())
                            })
                            .collect();

                        Some(success_lines)
                    } else {
                        None
                    }
                }
                Value {
                    value: UntaggedValue::Primitive(Primitive::EndOfStream),
                    ..
                } => {
                    let st = leftover_string.lock().clone();
                    if !st.is_empty() {
                        Some(vec![ReturnSuccess::value(
                            UntaggedValue::string(st).into_untagged_value(),
                        )])
                    } else {
                        None
                    }
                }
                Value {
                    tag: value_span, ..
                } => Some(vec![Err(ShellError::labeled_error_with_secondary(
                    "Expected a string from pipeline",
                    "requires string input",
                    name_span,
                    "value originates from here",
                    value_span,
                ))]),
            }
        })
        .flatten()
        .into_action_stream())
}

#[cfg(test)]
mod tests {
    use super::Lines;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Lines {})
    }
}
