use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::ShellTypeName;
use nu_protocol::{
    ColumnPath, Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::Tag;
use nu_value_ext::{as_string, ValueExt};

use std::cmp;
use std::cmp::Ordering;
use std::convert::TryInto;

#[derive(Deserialize)]
struct Arguments {
    range: Value,
    rest: Vec<ColumnPath>,
}

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "str substring"
    }

    fn signature(&self) -> Signature {
        Signature::build("str substring")
            .required(
                "range",
                SyntaxShape::Any,
                "the indexes to substring [start end]",
            )
            .rest(
                SyntaxShape::ColumnPath,
                "optionally substring text by column paths",
            )
    }

    fn usage(&self) -> &str {
        "substrings text"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        operate(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get a substring from the text",
                example: "echo 'good nushell' | str substring [5 12]",
                result: Some(vec![Value::from("nushell")]),
            },
            Example {
                description: "Alternatively, you can use the form",
                example: "echo 'good nushell' | str substring '5,12'",
                result: Some(vec![Value::from("nushell")]),
            },
            Example {
                description: "Get the last characters from the string",
                example: "echo 'good nushell' | str substring ',-5'",
                result: Some(vec![Value::from("shell")]),
            },
            Example {
                description: "Get the remaining characters from a starting index",
                example: "echo 'good nushell' | str substring '5,'",
                result: Some(vec![Value::from("nushell")]),
            },
            Example {
                description: "Get the characters from the beginning until ending index",
                example: "echo 'good nushell' | str substring ',7'",
                result: Some(vec![Value::from("good nu")]),
            },
        ]
    }
}

#[derive(Clone)]
struct Substring(isize, isize);

impl From<(isize, isize)> for Substring {
    fn from(input: (isize, isize)) -> Substring {
        Substring(input.0, input.1)
    }
}

struct SubstringText(String, String);

async fn operate(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let name = args.call_info.name_tag.clone();
    let registry = registry.clone();

    let (Arguments { range, rest }, input) = args.process(&registry).await?;

    let column_paths: Vec<_> = rest;
    let options = process_arguments(range, name)?.into();

    Ok(input
        .map(move |v| {
            if column_paths.is_empty() {
                ReturnSuccess::value(action(&v, &options, v.tag())?)
            } else {
                let mut ret = v;

                for path in &column_paths {
                    let options = options.clone();

                    ret = ret.swap_data_by_column_path(
                        path,
                        Box::new(move |old| action(old, &options, old.tag())),
                    )?;
                }

                ReturnSuccess::value(ret)
            }
        })
        .to_output_stream())
}

fn action(input: &Value, options: &Substring, tag: impl Into<Tag>) -> Result<Value, ShellError> {
    let tag = tag.into();

    match &input.value {
        UntaggedValue::Primitive(Primitive::Line(s))
        | UntaggedValue::Primitive(Primitive::String(s)) => {
            let len: isize = s.len().try_into().map_err(|_| {
                ShellError::labeled_error(
                    "could not perform substring",
                    "could not perform substring",
                    tag.span,
                )
            })?;

            let start: isize = options.0;
            let end: isize = options.1;

            if start < len && end >= 0 {
                match start.cmp(&end) {
                    Ordering::Equal => Ok(UntaggedValue::string("").into_value(tag)),
                    Ordering::Greater => Err(ShellError::labeled_error(
                        "End must be greater than or equal to Start",
                        "End must be greater than or equal to Start",
                        tag.span,
                    )),
                    Ordering::Less => {
                        let end: isize = cmp::min(options.1, len);

                        Ok(UntaggedValue::string(
                            s.chars()
                                .skip(start as usize)
                                .take((end - start) as usize)
                                .collect::<String>(),
                        )
                        .into_value(tag))
                    }
                }
            } else if start >= 0 && end <= 0 {
                let end = options.1.abs();
                let reversed = s
                    .chars()
                    .skip(start as usize)
                    .take((len - start) as usize)
                    .collect::<String>();

                let reversed = if start == 0 {
                    reversed
                } else {
                    s.chars().take(start as usize).collect::<String>()
                };

                let reversed = reversed
                    .chars()
                    .rev()
                    .take(end as usize)
                    .collect::<String>();

                Ok(
                    UntaggedValue::string(reversed.chars().rev().collect::<String>())
                        .into_value(tag),
                )
            } else {
                Ok(UntaggedValue::string("").into_value(tag))
            }
        }
        other => {
            let got = format!("got {}", other.type_name());
            Err(ShellError::labeled_error(
                "value is not string",
                got,
                tag.span,
            ))
        }
    }
}

fn process_arguments(range: Value, name: impl Into<Tag>) -> Result<(isize, isize), ShellError> {
    let name = name.into();

    let search = match &range.value {
        UntaggedValue::Table(indexes) => {
            if indexes.len() > 2 {
                Err(ShellError::labeled_error(
                    "could not perform substring",
                    "could not perform substring",
                    name.span,
                ))
            } else {
                let idx: Vec<String> = indexes
                    .iter()
                    .map(|v| as_string(v).unwrap_or_else(|_| String::from("")))
                    .collect();

                let start = idx
                    .get(0)
                    .ok_or_else(|| {
                        ShellError::labeled_error(
                            "could not perform substring",
                            "could not perform substring",
                            name.span,
                        )
                    })?
                    .to_string();
                let end = idx
                    .get(1)
                    .ok_or_else(|| {
                        ShellError::labeled_error(
                            "could not perform substring",
                            "could not perform substring",
                            name.span,
                        )
                    })?
                    .to_string();

                Ok(SubstringText(start, end))
            }
        }
        UntaggedValue::Primitive(Primitive::String(indexes)) => {
            let idx: Vec<&str> = indexes.split(',').collect();

            let start = idx
                .get(0)
                .ok_or_else(|| {
                    ShellError::labeled_error(
                        "could not perform substring",
                        "could not perform substring",
                        name.span,
                    )
                })?
                .to_string();
            let end = idx
                .get(1)
                .ok_or_else(|| {
                    ShellError::labeled_error(
                        "could not perform substring",
                        "could not perform substring",
                        name.span,
                    )
                })?
                .to_string();

            Ok(SubstringText(start, end))
        }
        _ => Err(ShellError::labeled_error(
            "could not perform substring",
            "could not perform substring",
            name.span,
        )),
    }?;

    let start = match &search {
        SubstringText(start, _) if start == "" || start == "_" => 0,
        SubstringText(start, _) => start.trim().parse().map_err(|_| {
            ShellError::labeled_error(
                "could not perform substring",
                "could not perform substring",
                name.span,
            )
        })?,
    };

    let end = match &search {
        SubstringText(_, end) if end == "" || end == "_" => isize::max_value(),
        SubstringText(_, end) => end.trim().parse().map_err(|_| {
            ShellError::labeled_error(
                "could not perform substring",
                "could not perform substring",
                name.span,
            )
        })?,
    };

    Ok((start, end))
}

#[cfg(test)]
mod tests {
    use super::{action, SubCommand, Substring};
    use nu_plugin::test_helpers::value::string;
    use nu_source::Tag;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }

    struct Expectation<'a> {
        options: (isize, isize),
        expected: &'a str,
    }

    impl Expectation<'_> {
        fn options(&self) -> Substring {
            Substring(self.options.0, self.options.1)
        }
    }

    fn expectation(word: &str, indexes: (isize, isize)) -> Expectation {
        Expectation {
            options: indexes,
            expected: word,
        }
    }

    #[test]
    fn substrings_indexes() {
        let word = string("andres");

        let cases = vec![
            expectation("a", (0, 1)),
            expectation("an", (0, 2)),
            expectation("and", (0, 3)),
            expectation("andr", (0, 4)),
            expectation("andre", (0, 5)),
            expectation("andres", (0, 6)),
            expectation("andres", (0, -6)),
            expectation("ndres", (0, -5)),
            expectation("dres", (0, -4)),
            expectation("res", (0, -3)),
            expectation("es", (0, -2)),
            expectation("s", (0, -1)),
            expectation("", (6, 0)),
            expectation("s", (6, -1)),
            expectation("es", (6, -2)),
            expectation("res", (6, -3)),
            expectation("dres", (6, -4)),
            expectation("ndres", (6, -5)),
            expectation("andres", (6, -6)),
        ];

        for expectation in cases.iter() {
            let expected = expectation.expected;
            let actual = action(&word, &expectation.options(), Tag::unknown()).unwrap();

            assert_eq!(actual, string(expected));
        }
    }
}
