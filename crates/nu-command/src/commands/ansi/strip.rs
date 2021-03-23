use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::ShellTypeName;
use nu_protocol::{
    ColumnPath, Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::Tag;
use strip_ansi_escapes::strip;

pub struct SubCommand;

#[derive(Deserialize)]
struct Arguments {
    rest: Vec<ColumnPath>,
}

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "ansi strip"
    }

    fn signature(&self) -> Signature {
        Signature::build("ansi strip").rest(
            SyntaxShape::ColumnPath,
            "optionally, remove ansi sequences by column paths",
        )
    }

    fn usage(&self) -> &str {
        "strip ansi escape sequences from string"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        operate(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "strip ansi escape sequences from string",
            example: "echo [$(ansi gb) 'hello' $(ansi reset)] | str collect | ansi strip",
            result: None,
        }]
    }
}

async fn operate(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let (Arguments { rest }, input) = args.process().await?;
    let column_paths: Vec<_> = rest;

    Ok(input
        .map(move |v| {
            if column_paths.is_empty() {
                ReturnSuccess::value(action(&v, v.tag())?)
            } else {
                let mut ret = v;

                for path in &column_paths {
                    ret = ret.swap_data_by_column_path(
                        path,
                        Box::new(move |old| action(old, old.tag())),
                    )?;
                }

                ReturnSuccess::value(ret)
            }
        })
        .to_output_stream())
}

fn action(input: &Value, tag: impl Into<Tag>) -> Result<Value, ShellError> {
    match &input.value {
        UntaggedValue::Primitive(Primitive::String(astring)) => {
            let stripped_string = {
                if let Ok(bytes) = strip(&astring) {
                    String::from_utf8_lossy(&bytes).to_string()
                } else {
                    astring.to_string()
                }
            };

            Ok(UntaggedValue::string(stripped_string).into_value(tag))
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

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    fn test_stripping() {
        let input_string =
            UntaggedValue::string("\u{1b}[3;93;41mHello\u{1b}[0m \u{1b}[1;32mNu \u{1b}[1;35mWorld")
                .into_untagged_value();
        let expected = UntaggedValue::string("Hello Nu World").into_untagged_value();

        let actual = action(&input_string, Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }
}
