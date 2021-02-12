use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::ShellTypeName;
use nu_protocol::{
    ColumnPath, Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::{Tag, Tagged};
use nu_value_ext::ValueExt;

#[derive(Deserialize)]
struct Arguments {
    pattern: Tagged<String>,
    rest: Vec<ColumnPath>,
    insensitive: bool,
}

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "str contains"
    }

    fn signature(&self) -> Signature {
        Signature::build("str contains")
            .required("pattern", SyntaxShape::String, "the pattern to find")
            .rest(
                SyntaxShape::ColumnPath,
                "optionally check if string contains pattern by column paths",
            )
            .switch("insensitive", "search is case insensitive", Some('i'))
    }

    fn usage(&self) -> &str {
        "Checks if string contains pattern"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        operate(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Check if string contains pattern",
                example: "echo 'my_library.rb' | str contains '.rb'",
                result: Some(vec![UntaggedValue::boolean(true).into_untagged_value()]),
            },
            Example {
                description: "Check if string contains pattern case insensitive",
                example: "echo 'my_library.rb' | str contains -i '.RB'",
                result: Some(vec![UntaggedValue::boolean(true).into_untagged_value()]),
            },
        ]
    }
}

async fn operate(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let (
        Arguments {
            pattern,
            rest,
            insensitive,
        },
        input,
    ) = args.process().await?;
    let column_paths: Vec<_> = rest;

    Ok(input
        .map(move |v| {
            if column_paths.is_empty() {
                ReturnSuccess::value(action(&v, &pattern, insensitive, v.tag())?)
            } else {
                let mut ret = v;

                for path in &column_paths {
                    let pattern = pattern.clone();
                    ret = ret.swap_data_by_column_path(
                        path,
                        Box::new(move |old| action(old, &pattern, insensitive, old.tag())),
                    )?;
                }

                ReturnSuccess::value(ret)
            }
        })
        .to_output_stream())
}

fn action(
    input: &Value,
    pattern: &str,
    insensitive: bool,
    tag: impl Into<Tag>,
) -> Result<Value, ShellError> {
    match &input.value {
        UntaggedValue::Primitive(Primitive::String(s)) => {
            let contains = if insensitive {
                s.to_lowercase().contains(&pattern.to_lowercase())
            } else {
                s.contains(pattern)
            };

            Ok(UntaggedValue::boolean(contains).into_value(tag))
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
    use super::ShellError;
    use super::{action, SubCommand};
    use nu_protocol::UntaggedValue;
    use nu_source::Tag;
    use nu_test_support::value::string;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    fn string_contains_other_string_case_sensitive() {
        let word = string("Cargo.tomL");
        let pattern = ".tomL";
        let insensitive = false;
        let expected = UntaggedValue::boolean(true).into_untagged_value();

        let actual = action(&word, &pattern, insensitive, Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn string_does_not_contain_other_string_case_sensitive() {
        let word = string("Cargo.tomL");
        let pattern = "Lomt.";
        let insensitive = false;
        let expected = UntaggedValue::boolean(false).into_untagged_value();

        let actual = action(&word, &pattern, insensitive, Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn string_contains_other_string_case_insensitive() {
        let word = string("Cargo.ToMl");
        let pattern = ".TOML";
        let insensitive = true;
        let expected = UntaggedValue::boolean(true).into_untagged_value();

        let actual = action(&word, &pattern, insensitive, Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn string_does_not_contain_other_string_case_insensitive() {
        let word = string("Cargo.tOml");
        let pattern = "lomt.";
        let insensitive = true;
        let expected = UntaggedValue::boolean(false).into_untagged_value();

        let actual = action(&word, &pattern, insensitive, Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }
}
