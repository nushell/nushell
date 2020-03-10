use crate::commands::PerItemCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{
    CallInfo, ColumnPath, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::Tagged;
use nu_value_ext::{as_column_path, get_data_by_column_path};
use std::borrow::Borrow;

use nom::{
    bytes::complete::{tag, take_while},
    IResult,
};

pub struct Format;

impl PerItemCommand for Format {
    fn name(&self) -> &str {
        "format"
    }

    fn signature(&self) -> Signature {
        Signature::build("format").required(
            "pattern",
            SyntaxShape::Any,
            "the pattern to output. Eg) \"{foo}: {bar}\"",
        )
    }

    fn usage(&self) -> &str {
        "Format columns into a string using a simple pattern."
    }

    fn run(
        &self,
        call_info: &CallInfo,
        _registry: &CommandRegistry,
        _raw_args: &RawCommandArgs,
        value: Value,
    ) -> Result<OutputStream, ShellError> {
        //let value_tag = value.tag();
        let pattern = call_info.args.expect_nth(0)?;
        let pattern_tag = pattern.tag.clone();
        let pattern = pattern.as_string()?;

        let format_pattern = format(&pattern).map_err(|_| {
            ShellError::labeled_error(
                "Could not create format pattern",
                "could not create format pattern",
                &pattern_tag,
            )
        })?;
        let commands = format_pattern.1;

        let output = match value {
            value
            @
            Value {
                value: UntaggedValue::Row(_),
                ..
            } => {
                let mut output = String::new();

                for command in &commands {
                    match command {
                        FormatCommand::Text(s) => {
                            output.push_str(s);
                        }
                        FormatCommand::Column(c) => {
                            let key = to_column_path(&c, &pattern_tag)?;

                            let fetcher = get_data_by_column_path(
                                &value,
                                &key,
                                Box::new(move |(_, _, error)| error),
                            );

                            if let Ok(c) = fetcher {
                                output
                                    .push_str(&value::format_leaf(c.borrow()).plain_string(100_000))
                            }
                            // That column doesn't match, so don't emit anything
                        }
                    }
                }

                output
            }
            _ => String::new(),
        };

        Ok(futures::stream::iter(vec![ReturnSuccess::value(
            UntaggedValue::string(output).into_untagged_value(),
        )])
        .to_output_stream())
    }
}

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
        if !before.is_empty() {
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

fn to_column_path(
    path_members: &str,
    tag: impl Into<Tag>,
) -> Result<Tagged<ColumnPath>, ShellError> {
    let tag = tag.into();

    as_column_path(
        &UntaggedValue::Table(
            path_members
                .split('.')
                .map(|x| {
                    let member = match x.parse::<u64>() {
                        Ok(v) => UntaggedValue::int(v),
                        Err(_) => UntaggedValue::string(x),
                    };

                    member.into_value(&tag)
                })
                .collect(),
        )
        .into_value(&tag),
    )
}
