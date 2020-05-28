use regex::{self, Regex};

use nu_errors::ShellError;
use nu_plugin::Plugin;
use nu_protocol::{
    CallInfo, Primitive, ReturnSuccess, ReturnValue, ShellTypeName, Signature, SyntaxShape,
    TaggedDictBuilder, UntaggedValue, Value,
};
use nu_source::Tag;

use crate::Parse;

impl Plugin for Parse {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("parse")
            .switch("regex", "use full regex syntax for patterns", Some('r'))
            .required(
                "pattern",
                SyntaxShape::String,
                "the pattern to match. Eg) \"{foo}: {bar}\"",
            )
            .filter())
    }

    fn begin_filter(&mut self, call_info: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        if let Some(ref args) = call_info.args.positional {
            let value = &args[0];
            match value {
                Value {
                    value: UntaggedValue::Primitive(Primitive::String(s)),
                    tag,
                } => {
                    self.pattern_tag = tag.clone();
                    self.regex = if call_info.args.has("regex") {
                        Regex::new(&s).map_err(|_| {
                            ShellError::labeled_error("Invalid regex", "invalid regex", tag.span)
                        })?
                    } else {
                        let parse_regex = build_regex(&s, tag.clone())?;
                        Regex::new(&parse_regex).map_err(|_| {
                            ShellError::labeled_error(
                                "Invalid pattern",
                                "invalid pattern",
                                tag.span,
                            )
                        })?
                    };

                    self.column_names = column_names(&self.regex);
                }
                Value { tag, .. } => {
                    return Err(ShellError::labeled_error(
                        format!(
                            "Unexpected type in params (found `{}`, expected `String`)",
                            value.type_name()
                        ),
                        "unexpected type",
                        tag,
                    ));
                }
            }
        }

        Ok(vec![])
    }

    fn filter(&mut self, input: Value) -> Result<Vec<ReturnValue>, ShellError> {
        if let Ok(s) = input.as_string() {
            Ok(self
                .regex
                .captures_iter(&s)
                .map(|caps| {
                    let mut dict = TaggedDictBuilder::new(&input.tag);
                    for (column_name, cap) in self.column_names.iter().zip(caps.iter().skip(1)) {
                        let cap_string = cap.map(|v| v.as_str()).unwrap_or("").to_string();
                        dict.insert_untagged(column_name, UntaggedValue::string(cap_string));
                    }

                    Ok(ReturnSuccess::Value(dict.into_value()))
                })
                .collect())
        } else {
            Err(ShellError::labeled_error_with_secondary(
                "Expected string input",
                "expected string input",
                &self.name,
                "value originated here",
                input.tag,
            ))
        }
    }
}

fn build_regex(input: &str, tag: Tag) -> Result<String, ShellError> {
    let mut output = "(?s)\\A".to_string();

    //let mut loop_input = input;
    let mut loop_input = input.chars().peekable();
    loop {
        let mut before = String::new();
        while let Some(c) = loop_input.next() {
            if c == '{' {
                // If '{{', still creating a plaintext parse command, but just for a single '{' char
                if loop_input.peek() == Some(&'{') {
                    let _ = loop_input.next();
                } else {
                    break;
                }
            }
            before.push(c);
        }

        if !before.is_empty() {
            output.push_str(&regex::escape(&before));
        }

        // Look for column as we're now at one
        let mut column = String::new();
        while let Some(c) = loop_input.next() {
            if c == '}' {
                break;
            }
            column.push(c);

            if loop_input.peek().is_none() {
                return Err(ShellError::labeled_error(
                    "Found opening `{` without an associated closing `}`",
                    "invalid parse pattern",
                    tag,
                ));
            }
        }

        if !column.is_empty() {
            output.push_str("(?P<");
            output.push_str(&column);
            output.push_str(">.*?)");
        }

        if before.is_empty() && column.is_empty() {
            break;
        }
    }

    output.push_str("\\z");
    Ok(output)
}

fn column_names(regex: &Regex) -> Vec<String> {
    regex
        .capture_names()
        .enumerate()
        .skip(1)
        .map(|(i, name)| {
            name.map(String::from)
                .unwrap_or_else(|| format!("Capture{}", i))
        })
        .collect()
}
