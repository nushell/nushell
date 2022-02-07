use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::ShellTypeName;
use nu_protocol::{
    ColumnPath, Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::{Tag, Tagged};
use nu_value_ext::ValueExt;

struct Arguments {
    pattern: Tagged<String>,
    insensitive: bool,
    column_paths: Vec<ColumnPath>,
}

pub struct SubCommand;

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "str contains"
    }

    fn signature(&self) -> Signature {
        Signature::build("str contains")
            .required("pattern", SyntaxShape::String, "the pattern to find")
            .rest(
                "rest",
                SyntaxShape::ColumnPath,
                "optionally check if string contains pattern by column paths",
            )
            .switch("insensitive", "search is case insensitive", Some('i'))
    }

    fn usage(&self) -> &str {
        "Checks if string contains pattern"
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        operate(args)
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

fn operate(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let (options, input) = (
        Arc::new(Arguments {
            pattern: args.req(0)?,
            insensitive: args.has_flag("insensitive"),
            column_paths: args.rest(1)?,
        }),
        args.input,
    );

    Ok(input
        .map(move |v| {
            if options.column_paths.is_empty() {
                ReturnSuccess::value(action(&v, &options, v.tag())?)
            } else {
                let mut ret = v;

                for path in &options.column_paths {
                    let options = options.clone();

                    ret = ret.swap_data_by_column_path(
                        path,
                        Box::new(move |old| action(old, &options, old.tag())),
                    )?;
                }

                ReturnSuccess::value(ret)
            }
        })
        .into_action_stream())
}

fn action(
    input: &Value,
    Arguments {
        ref pattern,
        insensitive,
        ..
    }: &Arguments,
    tag: impl Into<Tag>,
) -> Result<Value, ShellError> {
    match &input.value {
        UntaggedValue::Primitive(Primitive::String(s)) => {
            let contains = if *insensitive {
                s.to_lowercase().contains(&pattern.to_lowercase())
            } else {
                s.contains(&pattern.item)
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
    use super::{action, Arguments, SubCommand};
    use nu_protocol::UntaggedValue;
    use nu_source::{Tag, TaggedItem};
    use nu_test_support::value::string;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    fn string_contains_other_string_case_sensitive() {
        let word = string("Cargo.tomL");

        let options = Arguments {
            pattern: String::from(".tomL").tagged_unknown(),
            insensitive: false,
            column_paths: vec![],
        };

        let expected = UntaggedValue::boolean(true).into_untagged_value();
        let actual = action(&word, &options, Tag::unknown()).unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn string_does_not_contain_other_string_case_sensitive() {
        let word = string("Cargo.tomL");

        let options = Arguments {
            pattern: String::from("Lomt.").tagged_unknown(),
            insensitive: false,
            column_paths: vec![],
        };

        let expected = UntaggedValue::boolean(false).into_untagged_value();
        let actual = action(&word, &options, Tag::unknown()).unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn string_contains_other_string_case_insensitive() {
        let word = string("Cargo.ToMl");

        let options = Arguments {
            pattern: String::from(".TOML").tagged_unknown(),
            insensitive: true,
            column_paths: vec![],
        };

        let expected = UntaggedValue::boolean(true).into_untagged_value();
        let actual = action(&word, &options, Tag::unknown()).unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn string_does_not_contain_other_string_case_insensitive() {
        let word = string("Cargo.tOml");

        let options = Arguments {
            pattern: String::from("lomt.").tagged_unknown(),
            insensitive: true,
            column_paths: vec![],
        };

        let expected = UntaggedValue::boolean(false).into_untagged_value();
        let actual = action(&word, &options, Tag::unknown()).unwrap();

        assert_eq!(actual, expected);
    }
}
