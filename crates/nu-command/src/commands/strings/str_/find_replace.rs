use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::ShellTypeName;
use nu_protocol::{
    ColumnPath, Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::{Tag, Tagged};
use nu_value_ext::ValueExt;
use regex::Regex;

struct Arguments {
    all: bool,
    find: Tagged<String>,
    replace: Tagged<String>,
    column_paths: Vec<ColumnPath>,
}

pub struct SubCommand;

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "str find-replace"
    }

    fn signature(&self) -> Signature {
        Signature::build("str find-replace")
            .required("find", SyntaxShape::String, "the pattern to find")
            .required("replace", SyntaxShape::String, "the replacement pattern")
            .rest(
                "rest",
                SyntaxShape::ColumnPath,
                "optionally find and replace text by column paths",
            )
            .switch("all", "replace all occurrences of find string", Some('a'))
    }

    fn usage(&self) -> &str {
        "finds and replaces text"
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        operate(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Find and replace contents with capture group",
                example: "echo 'my_library.rb' | str find-replace '(.+).rb' '$1.nu'",
                result: Some(vec![Value::from("my_library.nu")]),
            },
            Example {
                description: "Find and replace all occurrences of find string",
                example: "echo 'abc abc abc' | str find-replace -a 'b' 'z'",
                result: Some(vec![Value::from("azc azc azc")]),
            },
        ]
    }
}

struct FindReplace<'a>(&'a str, &'a str);

fn operate(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let (options, input) = (
        Arc::new(Arguments {
            all: args.has_flag("all"),
            find: args.req(0)?,
            replace: args.req(1)?,
            column_paths: args.rest(2)?,
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
        find, replace, all, ..
    }: &Arguments,
    tag: impl Into<Tag>,
) -> Result<Value, ShellError> {
    match &input.value {
        UntaggedValue::Primitive(Primitive::String(s)) => {
            let FindReplace(find, replacement) = FindReplace(find, replace);
            let regex = Regex::new(find);

            Ok(match regex {
                Ok(re) => {
                    if *all {
                        UntaggedValue::string(re.replace_all(s, replacement).to_owned())
                    } else {
                        UntaggedValue::string(re.replace(s, replacement).to_owned())
                    }
                }
                Err(_) => UntaggedValue::string(s),
            }
            .into_value(tag))
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
    use nu_source::{Tag, TaggedItem};
    use nu_test_support::value::string;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    fn can_have_capture_groups() {
        let word = string("Cargo.toml");

        let options = Arguments {
            find: String::from("Cargo.(.+)").tagged_unknown(),
            replace: String::from("Carga.$1").tagged_unknown(),
            column_paths: vec![],
            all: false,
        };

        let actual = action(&word, &options, Tag::unknown()).unwrap();
        assert_eq!(actual, string("Carga.toml"));
    }
}
