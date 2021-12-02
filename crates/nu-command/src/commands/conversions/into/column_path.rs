use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Primitive, Signature, SyntaxShape, UntaggedValue, Value};

pub struct SubCommand;

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "into column_path"
    }

    fn signature(&self) -> Signature {
        Signature::build("into column_path").rest(
            "rest",
            SyntaxShape::ColumnPath,
            "values to convert to column_path",
        )
    }

    fn usage(&self) -> &str {
        "Convert value to column path"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        into_filepath(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Convert string to column_path in table",
                example: "echo [[name]; ['/dev/null'] ['C:\\Program Files'] ['../../Cargo.toml']] | into column_path name",
                result: Some(vec![
                    UntaggedValue::row(indexmap! {
                        "name".to_string() => UntaggedValue::column_path("/dev/null", Span::unknown()).into(),
                    })
                    .into(),
                    UntaggedValue::row(indexmap! {
                        "name".to_string() => UntaggedValue::column_path("C:\\Program Files", Span::unknown()).into(),
                    })
                    .into(),
                    UntaggedValue::row(indexmap! {
                        "name".to_string() => UntaggedValue::column_path("../../Cargo.toml", Span::unknown()).into(),
                    })
                    .into(),
                ]),
            },
            Example {
                description: "Convert string to column_path",
                example: "echo 'Cargo.toml' | into column_path",
                result: Some(vec![UntaggedValue::column_path("Cargo.toml", Span::unknown()).into()]),
            },
        ]
    }
}

fn into_filepath(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let column_paths: Vec<ColumnPath> = args.rest(0)?;

    Ok(args
        .input
        .map(move |v| {
            if column_paths.is_empty() {
                action(&v, v.tag())
            } else {
                let mut ret = v;
                for path in &column_paths {
                    ret = ret.swap_data_by_column_path(
                        path,
                        Box::new(move |old| action(old, old.tag())),
                    )?;
                }

                Ok(ret)
            }
        })
        .into_input_stream())
}

pub fn action(input: &Value, tag: impl Into<Tag>) -> Result<Value, ShellError> {
    let tag = tag.into();
    match &input.value {
        UntaggedValue::Primitive(prim) => Ok(UntaggedValue::column_path(
            match prim {
                Primitive::String(a_string) => a_string,
                _ => {
                    return Err(ShellError::unimplemented(
                        "'into column_path' for non-string primitives",
                    ))
                }
            },
            Span::unknown(),
        )
        .into_value(&tag)),
        UntaggedValue::Row(_) => Err(ShellError::labeled_error(
            "specify column name to use, with 'into column_path COLUMN'",
            "found table",
            tag,
        )),
        _ => Err(ShellError::unimplemented(
            "'into column_path' for unsupported type",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::SubCommand;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }
}
