use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::ShellTypeName;
use nu_protocol::{
    ColumnPath, Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::Tag;
use nu_value_ext::ValueExt;

#[derive(Deserialize)]
struct Arguments {
    rest: Vec<ColumnPath>,
}

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "str downcase"
    }

    fn signature(&self) -> Signature {
        Signature::build("str downcase").rest(
            SyntaxShape::ColumnPath,
            "optionally downcase text by column paths",
        )
    }

    fn usage(&self) -> &str {
        "downcases text"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        operate(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Downcase contents",
            example: "echo 'NU' | str downcase",
            result: Some(vec![Value::from("nu")]),
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
        UntaggedValue::Primitive(Primitive::String(s)) => {
            Ok(UntaggedValue::string(s.to_ascii_lowercase()).into_value(tag))
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
    use nu_source::Tag;
    use nu_test_support::value::string;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    fn downcases() {
        let word = string("ANDRES");
        let expected = string("andres");

        let actual = action(&word, Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }
}
