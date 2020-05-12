use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use nu_value_ext::{as_column_path, get_data_by_column_path};
use std::borrow::Borrow;

pub struct Format;

#[derive(Deserialize)]
pub struct FormatArgs {
    pattern: Tagged<String>,
}

impl WholeStreamCommand for Format {
    fn name(&self) -> &str {
        "format"
    }

    fn signature(&self) -> Signature {
        Signature::build("format").required(
            "pattern",
            SyntaxShape::String,
            "the pattern to output. Eg) \"{foo}: {bar}\"",
        )
    }

    fn usage(&self) -> &str {
        "Format columns into a string using a simple pattern."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, format_command)?.run()
    }

    fn examples(&self) -> &[Example] {
        &[Example {
            description: "Print filenames with their sizes",
            example: "ls | format '{name}: {size}'",
        }]
    }
}

fn format_command(
    FormatArgs { pattern }: FormatArgs,
    RunnableContext { input, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let pattern_tag = pattern.tag.clone();

    let format_pattern = format(&pattern);
    let commands = format_pattern;
    let mut input = input;

    let stream = async_stream! {
        while let Some(value) = input.next().await {
            match value {
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
                                output.push_str(&s);
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

                    yield ReturnSuccess::value(
                        UntaggedValue::string(output).into_untagged_value())
                }
                _ => yield ReturnSuccess::value(
                    UntaggedValue::string(String::new()).into_untagged_value()),
            };
        }
    };

    Ok(stream.to_output_stream())
}

#[derive(Debug)]
enum FormatCommand {
    Text(String),
    Column(String),
}

fn format(input: &str) -> Vec<FormatCommand> {
    let mut output = vec![];

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
            output.push(FormatCommand::Text(before.to_string()));
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
            output.push(FormatCommand::Column(column.to_string()));
        }

        if before.is_empty() && column.is_empty() {
            break;
        }
    }

    output
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
