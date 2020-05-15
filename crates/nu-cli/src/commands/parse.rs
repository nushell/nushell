use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue};
use nu_source::Tagged;
use regex::Regex;

#[derive(Debug)]
enum ParseCommand {
    Text(String),
    Column(String),
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
pub struct Parse;

#[derive(Deserialize)]
pub struct ParseArgs {
    pattern: Tagged<String>,
}

impl WholeStreamCommand for Parse {
    fn name(&self) -> &str {
        "parse"
    }

    fn signature(&self) -> Signature {
        Signature::build("parse").required(
            "pattern",
            SyntaxShape::String,
            "the pattern to match. Eg) \"{foo}: {bar}\"",
        )
    }

    fn usage(&self) -> &str {
        "Parse columns from string data using a simple pattern."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        parse_command(args, registry)
    }

    fn examples(&self) -> &[Example] {
        &[Example {
            description: "Parse values from a string into a table",
            example: r#"echo "data: 123" | parse "{key}: {value}""#,
        }]
    }
}

fn create_regex(parse_regex: String, tag: Tag) -> Result<Regex, ShellError> {
    let regex = Regex::new(&parse_regex).map_err(|_| {
        ShellError::labeled_error("Could not parse regex", "could not parse regex", tag.span)
    });
}

fn parse_command(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let name = args.call_info.name_tag.clone();
    let stream = async_stream! {
        let (ParseArgs { pattern }, mut input) = args.process(&registry).await?;
        let parse_pattern = parse(&pattern.item);
        let parse_regex = build_regex(&parse_pattern);
        let column_names = column_names(&parse_pattern);
        let name = name.span;

        let regex = create_regex(parse_regex.clone(), pattern.tag.clone())?;

        while let Some(value) = input.next().await {
            if let Ok(s) = value.as_string() {
                for cap in regex.captures_iter(&s) {
                    let mut dict = TaggedDictBuilder::new(value.tag());
                    for (idx, column_name) in column_names.iter().enumerate() {
                        dict.insert_untagged(
                            column_name,
                            UntaggedValue::string(cap[idx + 1].to_string()),
                        );
                    }
                    yield Ok(ReturnSuccess::Value(dict.into_value()));
                }
            } else {
                yield Err(ShellError::labeled_error_with_secondary(
                    "Expected string input",
                    "expected string input",
                    name,
                    "value originated here",
                    value.tag,
                ));
            }
        }
    };

    Ok(stream.to_output_stream())
}
