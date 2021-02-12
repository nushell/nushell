use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue, Value};
use nu_source::Tagged;

use regex::Regex;

#[derive(Deserialize)]
struct Arguments {
    pattern: Tagged<String>,
    regex: Tagged<bool>,
}

pub struct Command;

#[async_trait]
impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "parse"
    }

    fn signature(&self) -> Signature {
        Signature::build("parse")
            .required(
                "pattern",
                SyntaxShape::String,
                "the pattern to match. Eg) \"{foo}: {bar}\"",
            )
            .switch("regex", "use full regex syntax for patterns", Some('r'))
    }

    fn usage(&self) -> &str {
        "Parse columns from string data using a simple pattern."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        operate(args).await
    }

    fn examples(&self) -> Vec<Example> {
        let mut row = IndexMap::new();
        row.insert("foo".to_string(), Value::from("hi"));
        row.insert("bar".to_string(), Value::from("there"));
        vec![Example {
            description: "Parse a string into two named columns",
            example: "echo \"hi there\" | parse \"{foo} {bar}\"",
            result: Some(vec![UntaggedValue::row(row).into()]),
        }]
    }
}

pub async fn operate(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name_tag = args.call_info.name_tag.clone();
    let (Arguments { regex, pattern }, mut input) = args.process().await?;

    let regex_pattern = if let Tagged { item: true, tag } = regex {
        Regex::new(&pattern.item)
            .map_err(|_| ShellError::labeled_error("Invalid regex", "invalid regex", tag.span))?
    } else {
        let parse_regex = build_regex(&pattern.item, name_tag.clone())?;

        Regex::new(&parse_regex).map_err(|_| {
            ShellError::labeled_error("Invalid pattern", "invalid pattern", name_tag.span)
        })?
    };

    let columns = column_names(&regex_pattern);
    let mut parsed: VecDeque<Value> = VecDeque::new();

    while let Some(v) = input.next().await {
        match v.as_string() {
            Ok(s) => {
                let results = regex_pattern.captures_iter(&s);

                for c in results {
                    let mut dict = TaggedDictBuilder::new(&v.tag);

                    for (column_name, cap) in columns.iter().zip(c.iter().skip(1)) {
                        let cap_string = cap.map(|v| v.as_str()).unwrap_or("").to_string();
                        dict.insert_untagged(column_name, UntaggedValue::string(cap_string));
                    }

                    parsed.push_back(dict.into_value());
                }
            }
            Err(_) => {
                return Err(ShellError::labeled_error_with_secondary(
                    "Expected string input",
                    "expected string input",
                    &name_tag,
                    "value originated here",
                    v.tag,
                ))
            }
        }
    }

    Ok(futures::stream::iter(parsed).to_output_stream())
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

#[cfg(test)]
mod tests {
    use super::Command;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Command {})
    }
}
