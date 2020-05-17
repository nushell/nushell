use nu_errors::ShellError;
use nu_plugin::Plugin;
use nu_protocol::{
    CallInfo, Primitive, ReturnSuccess, ReturnValue, Signature, SyntaxShape, TaggedDictBuilder,
    UntaggedValue, Value,
};

use crate::Parse;
use regex::Regex;

impl Plugin for Parse {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("parse")
            .required(
                "pattern",
                SyntaxShape::String,
                "the pattern to match. Eg) \"{foo}: {bar}\"",
            )
            .filter())
    }

    fn begin_filter(&mut self, call_info: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        if let Some(args) = call_info.args.positional {
            match &args[0] {
                Value {
                    value: UntaggedValue::Primitive(Primitive::String(s)),
                    tag,
                } => {
                    self.pattern_tag = tag.clone();
                    let parse_pattern = parse(&s);
                    let parse_regex = build_regex(&parse_pattern);
                    self.column_names = column_names(&parse_pattern);

                    self.regex = Regex::new(&parse_regex).map_err(|_| {
                        ShellError::labeled_error(
                            "Could not parse regex",
                            "could not parse regex",
                            tag.span,
                        )
                    })?;
                }
                Value { tag, .. } => {
                    return Err(ShellError::labeled_error(
                        "Unrecognized type in params",
                        "unexpected value",
                        tag,
                    ));
                }
            }
        }
        Ok(vec![])
    }

    fn filter(&mut self, input: Value) -> Result<Vec<ReturnValue>, ShellError> {
        match &input.as_string() {
            Ok(s) => {
                let mut output = vec![];
                for caps in self.regex.captures_iter(&s) {
                    let group_count = caps.len() - 1;

                    if self.column_names.len() != group_count {
                        return Err(ShellError::labeled_error(
                            format!(
                                "There are {} column(s) specified in the pattern, but could only match the first {}: [{}]",
                                self.column_names.len(),
                                group_count,
                                caps.iter()
                                    .skip(1)
                                    .map(|m| {
                                        if let Some(m) = m {
                                            let m = m.as_str();
                                            let mut m = m.replace(",","\\,");
                                            if m.len() > 20 {
                                                m.truncate(17);
                                                format!("{}...", m)
                                            } else {
                                                m
                                            }
                                        } else {
                                            "<none>".to_string()
                                        }
                                    })
                                    .collect::<Vec<String>>()
                                    .join(", ")
                            ),
                            "could not match all columns in pattern",
                            &self.pattern_tag,
                        ));
                    }

                    let mut dict = TaggedDictBuilder::new(&input.tag);
                    for (idx, column_name) in self.column_names.iter().enumerate() {
                        dict.insert_untagged(
                            column_name,
                            UntaggedValue::string(caps[idx + 1].to_string()),
                        );
                    }
                    output.push(Ok(ReturnSuccess::Value(dict.into_value())));
                }
                Ok(output)
            }
            _ => Err(ShellError::labeled_error_with_secondary(
                "Expected string input",
                "expected string input",
                &self.name,
                "value originated here",
                input.tag,
            )),
        }
    }
}

fn parse(input: &str) -> Vec<ParseCommand> {
    let mut output = vec![];

    //let mut loop_input = input;
    let mut loop_input = input.chars();
    loop {
        let mut before = String::new();

        while let Some(c) = loop_input.next() {
            if c == '{' {
                break;
            }
            before.push(c);
        }

        if !before.is_empty() {
            output.push(ParseCommand::Text(before.to_string()));
        }
        // Look for column as we're now at one
        let mut column = String::new();

        while let Some(c) = loop_input.next() {
            if c == '}' {
                break;
            }
            column.push(c);
        }

        if !column.is_empty() {
            output.push(ParseCommand::Column(column.to_string()));
        }

        if before.is_empty() && column.is_empty() {
            break;
        }
    }

    output
}

fn column_names(commands: &[ParseCommand]) -> Vec<String> {
    let mut output = vec![];

    for command in commands {
        if let ParseCommand::Column(c) = command {
            output.push(c.clone());
        }
    }

    output
}

fn build_regex(commands: &[ParseCommand]) -> String {
    let mut output = String::new();

    for command in commands {
        match command {
            ParseCommand::Text(s) => {
                output.push_str(&s.replace("(", "\\("));
            }
            ParseCommand::Column(_) => {
                output.push_str("(.*)");
            }
        }
    }

    output
}

#[derive(Debug)]
enum ParseCommand {
    Text(String),
    Column(String),
}
