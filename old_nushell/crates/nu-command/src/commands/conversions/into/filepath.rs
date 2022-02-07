use std::path::PathBuf;

use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Primitive, Signature, SyntaxShape, UntaggedValue, Value};

pub struct SubCommand;

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "into path"
    }

    fn signature(&self) -> Signature {
        Signature::build("into path").rest(
            "rest",
            SyntaxShape::ColumnPath,
            "column paths to convert to filepath (for table input)",
        )
    }

    fn usage(&self) -> &str {
        "Convert value to filepath"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        into_filepath(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Convert string to filepath in table",
                example: "echo [[name]; ['/dev/null'] ['C:\\Program Files'] ['../../Cargo.toml']] | into path name",
                result: Some(vec![
                    UntaggedValue::row(indexmap! {
                        "name".to_string() => UntaggedValue::filepath("/dev/null").into(),
                    })
                    .into(),
                    UntaggedValue::row(indexmap! {
                        "name".to_string() => UntaggedValue::filepath("C:\\Program Files").into(),
                    })
                    .into(),
                    UntaggedValue::row(indexmap! {
                        "name".to_string() => UntaggedValue::filepath("../../Cargo.toml").into(),
                    })
                    .into(),
                ]),
            },
            Example {
                description: "Convert string to filepath",
                example: "echo 'Cargo.toml' | into path",
                result: Some(vec![UntaggedValue::filepath("Cargo.toml").into()]),
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
        UntaggedValue::Primitive(prim) => Ok(UntaggedValue::filepath(match prim {
            Primitive::String(a_string) => match filepath_from_string(a_string, &tag) {
                Ok(n) => n,
                Err(e) => {
                    return Err(e);
                }
            },
            Primitive::FilePath(a_filepath) => a_filepath.clone(),
            _ => {
                return Err(ShellError::unimplemented(
                    "'into path' for non-string primitives",
                ))
            }
        })
        .into_value(&tag)),
        UntaggedValue::Row(_) => Err(ShellError::labeled_error(
            "specify column name to use, with 'into path COLUMN'",
            "found table",
            tag,
        )),
        _ => Err(ShellError::unimplemented(
            "'into path' for unsupported type",
        )),
    }
}

fn filepath_from_string(a_string: &str, _tag: &Tag) -> Result<PathBuf, ShellError> {
    Ok(PathBuf::from(a_string))
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
