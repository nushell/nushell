use nu::{serve_plugin, value, Plugin};
use nu_errors::ShellError;
use nu_protocol::{
    CallInfo, Primitive, ReturnSuccess, ReturnValue, Signature, SyntaxShape, UntaggedValue, Value,
};

use nom::{
    bytes::complete::{tag, take_while},
    IResult,
};

#[derive(Debug)]
enum FormatCommand {
    Text(String),
    Column(String),
}

fn format(input: &str) -> IResult<&str, Vec<FormatCommand>> {
    let mut output = vec![];

    let mut loop_input = input;
    loop {
        let (input, before) = take_while(|c| c != '{')(loop_input)?;
        if before.len() > 0 {
            output.push(FormatCommand::Text(before.to_string()));
        }
        if input != "" {
            // Look for column as we're now at one
            let (input, _) = tag("{")(input)?;
            let (input, column) = take_while(|c| c != '}')(input)?;
            let (input, _) = tag("}")(input)?;

            output.push(FormatCommand::Column(column.to_string()));
            loop_input = input;
        } else {
            loop_input = input;
        }
        if loop_input == "" {
            break;
        }
    }

    Ok((loop_input, output))
}

struct Format {
    commands: Vec<FormatCommand>,
}

impl Format {
    fn new() -> Self {
        Format { commands: vec![] }
    }
}

impl Plugin for Format {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("format")
            .desc("Format columns into a string using a simple pattern")
            .required(
                "pattern",
                SyntaxShape::Any,
                "the pattern to match. Eg) \"{foo}: {bar}\"",
            )
            .filter())
    }
    fn begin_filter(&mut self, call_info: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        if let Some(args) = call_info.args.positional {
            match &args[0] {
                Value {
                    value: UntaggedValue::Primitive(Primitive::String(pattern)),
                    ..
                } => {
                    let format_pattern = format(&pattern).unwrap();
                    self.commands = format_pattern.1
                }
                Value { tag, .. } => {
                    return Err(ShellError::labeled_error(
                        "Unrecognized type in params",
                        "expected a string",
                        tag,
                    ));
                }
            }
        }
        Ok(vec![])
    }

    fn filter(&mut self, input: Value) -> Result<Vec<ReturnValue>, ShellError> {
        match &input {
            Value {
                value: UntaggedValue::Row(dict),
                ..
            } => {
                let mut output = String::new();

                for command in &self.commands {
                    match command {
                        FormatCommand::Text(s) => {
                            output.push_str(s);
                        }
                        FormatCommand::Column(c) => {
                            match dict.entries.get(c) {
                                Some(c) => match c.as_string() {
                                    Ok(v) => output.push_str(&v),
                                    _ => return Ok(vec![]),
                                },
                                None => {
                                    // This row doesn't match, so don't emit anything
                                    return Ok(vec![]);
                                }
                            }
                        }
                    }
                }

                return Ok(vec![ReturnSuccess::value(
                    value::string(output).into_untagged_value(),
                )]);
            }
            _ => {}
        }
        Ok(vec![])
    }
}

fn main() {
    serve_plugin(&mut Format::new());
}
