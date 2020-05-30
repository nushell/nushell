use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::ShellTypeName;
use nu_protocol::{
    ColumnPath, Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::{Tag, Tagged};
use nu_value_ext::ValueExt;

use std::cmp;

#[derive(Deserialize)]
struct Arguments {
    range: Tagged<String>,
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
                SyntaxShape::String,
                "the indexes to substring \"start, end\"",
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
        operate(args, registry)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get a substring from the text",
                example: "echo 'good nushell' | str substring '5,12'",
                result: Some(vec![Value::from("nushell")]),
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
struct Substring(usize, usize);

fn operate(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let name = args.call_info.name_tag.clone();
    let registry = registry.clone();

    let stream = async_stream! {
        let (Arguments { range, rest }, mut input) = args.process(&registry).await?;

        let v: Vec<&str> = range.item.split(',').collect();

        let start = match v[0] {
            "" => 0,
            _ => v[0]
                .trim()
                .parse()
                .map_err(|_| {
                    ShellError::labeled_error(
                        "could not perform substring",
                        "could not perform substring",
                        name.span,
                    )
                })?
        };

        let end = match v[1] {
            "" => usize::max_value(),
            _ => v[1]
                .trim()
                .parse()
                .map_err(|_| {
                    ShellError::labeled_error(
                        "could not perform substring",
                        "could not perform substring",
                        name.span,
                    )
                })?
        };

        if start > end {
            yield Err(ShellError::labeled_error(
                "End must be greater than or equal to Start",
                "End must be greater than or equal to Start",
                name.span,
            ));
            return;
        }

        let options = Substring(start, end);

        let column_paths: Vec<_> = rest.iter().map(|x| x.clone()).collect();

        while let Some(v) = input.next().await {
            if column_paths.is_empty() {
                match action(&v, &options, v.tag()) {
                    Ok(out) => yield ReturnSuccess::value(out),
                    Err(err) => {
                        yield Err(err);
                        return;
                    }
                }
            } else {

                let mut ret = v.clone();

                for path in &column_paths {
                    let options = options.clone();

                    let swapping = ret.swap_data_by_column_path(path, Box::new(move |old| action(old, &options, old.tag())));

                    match swapping {
                        Ok(new_value) => {
                            ret = new_value;
                        }
                        Err(err) => {
                            yield Err(err);
                            return;
                        }
                    }
                }

                yield ReturnSuccess::value(ret);
            }
        }
    };

    Ok(stream.to_output_stream())
}

fn action(input: &Value, options: &Substring, tag: impl Into<Tag>) -> Result<Value, ShellError> {
    match &input.value {
        UntaggedValue::Primitive(Primitive::Line(s))
        | UntaggedValue::Primitive(Primitive::String(s)) => {
            let start = options.0;
            let end: usize = cmp::min(options.1, s.len());

            let out = {
                if start > s.len() - 1 {
                    UntaggedValue::string("")
                } else {
                    UntaggedValue::string(
                        s.chars().skip(start).take(end - start).collect::<String>(),
                    )
                }
            };

            Ok(out.into_value(tag))
        }
        other => {
            let got = format!("got {}", other.type_name());
            Err(ShellError::labeled_error(
                "value is not string",
                got,
                tag.into().span,
            ))
        }
    }
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

    #[test]
    fn given_start_and_end_indexes() {
        let word = string("andresS");
        let expected = string("andres");

        let substring_options = Substring(0, 6);

        let actual = action(&word, &substring_options, Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }
}
