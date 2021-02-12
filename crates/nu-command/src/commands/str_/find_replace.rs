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

#[derive(Deserialize)]
struct Arguments {
    find: Tagged<String>,
    replace: Tagged<String>,
    rest: Vec<ColumnPath>,
    all: bool,
}

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "str find-replace"
    }

    fn signature(&self) -> Signature {
        Signature::build("str find-replace")
            .required("find", SyntaxShape::String, "the pattern to find")
            .required("replace", SyntaxShape::String, "the replacement pattern")
            .rest(
                SyntaxShape::ColumnPath,
                "optionally find and replace text by column paths",
            )
            .switch("all", "replace all occurrences of find string", Some('a'))
    }

    fn usage(&self) -> &str {
        "finds and replaces text"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        operate(args).await
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

#[derive(Clone)]
struct FindReplace(String, String);

async fn operate(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let (
        Arguments {
            find,
            replace,
            rest,
            all,
        },
        input,
    ) = args.process().await?;
    let options = FindReplace(find.item, replace.item);
    let column_paths: Vec<_> = rest;

    Ok(input
        .map(move |v| {
            if column_paths.is_empty() {
                ReturnSuccess::value(action(&v, &options, v.tag(), all)?)
            } else {
                let mut ret = v;

                for path in &column_paths {
                    let options = options.clone();

                    ret = ret.swap_data_by_column_path(
                        path,
                        Box::new(move |old| action(old, &options, old.tag(), all)),
                    )?;
                }

                ReturnSuccess::value(ret)
            }
        })
        .to_output_stream())
}

fn action(
    input: &Value,
    options: &FindReplace,
    tag: impl Into<Tag>,
    all: bool,
) -> Result<Value, ShellError> {
    match &input.value {
        UntaggedValue::Primitive(Primitive::String(s)) => {
            let find = &options.0;
            let replacement = &options.1;

            let regex = Regex::new(find.as_str());

            let out = match regex {
                Ok(re) => {
                    if all {
                        UntaggedValue::string(re.replace_all(s, replacement.as_str()).to_owned())
                    } else {
                        UntaggedValue::string(re.replace(s, replacement.as_str()).to_owned())
                    }
                }
                Err(_) => UntaggedValue::string(s),
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
    use super::ShellError;
    use super::{action, FindReplace, SubCommand};
    use nu_source::Tag;
    use nu_test_support::value::string;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    fn can_have_capture_groups() {
        let word = string("Cargo.toml");
        let expected = string("Carga.toml");
        let all = false;
        let find_replace_options = FindReplace("Cargo.(.+)".to_string(), "Carga.$1".to_string());

        let actual = action(&word, &find_replace_options, Tag::unknown(), all).unwrap();
        assert_eq!(actual, expected);
    }
}
