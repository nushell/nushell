use nu_errors::ShellError;
use nu_plugin::Plugin;
use nu_protocol::{
    CallInfo, Primitive, ReturnSuccess, ReturnValue, Signature, SyntaxShape, TaggedDictBuilder,
    UntaggedValue, Value,
};

use crate::{ColumnNames, Parse};
use regex::{self, Regex};

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
        if let Some(ref args) = &call_info.args.positional {
            match &args[0] {
                Value {
                    value: UntaggedValue::Primitive(Primitive::String(s)),
                    tag,
                } => {
                    self.pattern_tag = tag.clone();
                    if call_info.args.has("regex") {
                        self.regex = Regex::new(&s).map_err(|_| {
                            ShellError::labeled_error(
                                "Could not parse regex",
                                "could not parse regex",
                                tag.span,
                            )
                        })?;
                        self.column_names = ColumnNames::from(&self.regex);
                    } else {
                        let parse_pattern = parse(&s);
                        let parse_regex = build_regex(&parse_pattern);
                        self.column_names = ColumnNames::from(parse_pattern.as_slice());
                        self.regex = Regex::new(&parse_regex).map_err(|_| {
                            ShellError::labeled_error(
                                "Could not parse regex",
                                "could not parse regex",
                                tag.span,
                            )
                        })?;
                    };
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

                    if self.column_names.0.len() != group_count {
                        return Err(ShellError::labeled_error(
                            format!(
                                "There are {} column(s) specified in the pattern, but could only match the first {}: [{}]",
                                self.column_names.0.len(),
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
                    for (idx, column_name) in self.column_names.0.iter().enumerate() {
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

impl From<&[ParseCommand]> for ColumnNames {
    fn from(commands: &[ParseCommand]) -> ColumnNames {
        let mut output = vec![];

        for command in commands {
            if let ParseCommand::Column(c) = command {
                output.push(c.clone());
            }
        }

        ColumnNames(output)
    }
}

impl From<&Regex> for ColumnNames {
    fn from(regex: &Regex) -> ColumnNames {
        let output = regex
            .capture_names()
            .enumerate()
            .skip(1)
            .map(|(i, name)| name.map(String::from).unwrap_or(format!("Capture{}", i)))
            .collect::<Vec<_>>();
        ColumnNames(output)
    }
}

fn build_regex(commands: &[ParseCommand]) -> String {
    let mut output = "(?s)\\A".to_string();

    for command in commands {
        match command {
            ParseCommand::Text(s) => {
                output.push_str(&regex::escape(&s));
            }
            ParseCommand::Column(_) => {
                output.push_str("(.*?)");
            }
        }
    }

    output.push_str("\\z");

    output
}

#[derive(Debug)]
enum ParseCommand {
    Text(String),
    Column(String),
}
