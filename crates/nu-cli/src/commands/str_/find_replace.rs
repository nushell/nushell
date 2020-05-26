use crate::commands::WholeStreamCommand;
use crate::prelude::*;
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
                SyntaxShape::ColumnPath,
                "optionally find and replace text by column paths",
            )
    }

    fn usage(&self) -> &str {
        "finds and replaces text"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        operate(args, registry)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Find and replace contents with capture group",
            example: "echo 'my_library.rb' | str find-replace '(.+).rb' '$1.nu'",
            result: Some(vec![Value::from("my_library.nu")]),
        }]
    }
}

#[derive(Clone)]
struct FindReplace(String, String);

fn operate(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();

    let stream = async_stream! {
        let (Arguments { find, replace, rest }, mut input) = args.process(&registry).await?;
        let options = FindReplace(find.item, replace.item);

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

                    let swapping = ret.swap_data_by_column_path(path, Box::new(move |old| {
                        action(old, &options, old.tag())
                    }));

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

fn action(input: &Value, options: &FindReplace, tag: impl Into<Tag>) -> Result<Value, ShellError> {
    match &input.value {
        UntaggedValue::Primitive(Primitive::Line(s))
        | UntaggedValue::Primitive(Primitive::String(s)) => {
            let find = &options.0;
            let replacement = &options.1;

            let regex = Regex::new(find.as_str());

            let out = match regex {
                Ok(re) => UntaggedValue::string(re.replace(s, replacement.as_str()).to_owned()),
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
    use super::{action, FindReplace, SubCommand};
    use nu_plugin::test_helpers::value::string;
    use nu_source::Tag;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    fn can_have_capture_groups() {
        let word = string("Cargo.toml");
        let expected = string("Carga.toml");

        let find_replace_options = FindReplace("Cargo.(.+)".to_string(), "Carga.$1".to_string());

        let actual = action(&word, &find_replace_options, Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }
}
