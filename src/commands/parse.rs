use crate::commands::PerItemCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{
    CallInfo, ReturnSuccess, Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue, Value,
};

use nom::{
    bytes::complete::{tag, take_while},
    IResult,
};
use regex::Regex;

#[derive(Debug)]
enum ParseCommand {
    Text(String),
    Column(String),
}

fn parse(input: &str) -> IResult<&str, Vec<ParseCommand>> {
    let mut output = vec![];

    let mut loop_input = input;
    loop {
        let (input, before) = take_while(|c| c != '{')(loop_input)?;
        if !before.is_empty() {
            output.push(ParseCommand::Text(before.to_string()));
        }
        if input != "" {
            // Look for column as we're now at one
            let (input, _) = tag("{")(input)?;
            let (input, column) = take_while(|c| c != '}')(input)?;
            let (input, _) = tag("}")(input)?;

            output.push(ParseCommand::Column(column.to_string()));
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
pub struct Parse;

impl PerItemCommand for Parse {
    fn name(&self) -> &str {
        "parse"
    }

    fn signature(&self) -> Signature {
        Signature::build("parse").required(
            "pattern",
            SyntaxShape::Any,
            "the pattern to match. Eg) \"{foo}: {bar}\"",
        )
    }

    fn usage(&self) -> &str {
        "Parse columns from string data using a simple pattern."
    }

    fn run(
        &self,
        call_info: &CallInfo,
        _registry: &CommandRegistry,
        _raw_args: &RawCommandArgs,
        value: Value,
    ) -> Result<OutputStream, ShellError> {
        //let value_tag = value.tag();
        let pattern = call_info.args.expect_nth(0)?.as_string()?;

        let parse_pattern = parse(&pattern).map_err(|_| {
            ShellError::labeled_error(
                "Could not create parse pattern",
                "could not create parse pattern",
                &value.tag,
            )
        })?;
        let parse_regex = build_regex(&parse_pattern.1);

        let column_names = column_names(&parse_pattern.1);
        let regex = Regex::new(&parse_regex).map_err(|_| {
            ShellError::labeled_error("Could not parse regex", "could not parse regex", &value.tag)
        })?;

        let output = if let Ok(s) = value.as_string() {
            let mut results = vec![];
            for cap in regex.captures_iter(&s) {
                let mut dict = TaggedDictBuilder::new(value.tag());

                for (idx, column_name) in column_names.iter().enumerate() {
                    dict.insert_untagged(
                        column_name,
                        UntaggedValue::string(&cap[idx + 1].to_string()),
                    );
                }

                results.push(ReturnSuccess::value(dict.into_value()));
            }

            VecDeque::from(results)
        } else {
            VecDeque::new()
        };
        Ok(output.to_output_stream())
    }
}
