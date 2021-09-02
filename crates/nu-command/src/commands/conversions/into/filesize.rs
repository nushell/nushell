use std::convert::TryInto;

use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Primitive, Signature, SyntaxShape, UntaggedValue, Value};
use num_bigint::ToBigInt;

pub struct SubCommand;

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "into filesize"
    }

    fn signature(&self) -> Signature {
        Signature::build("into filesize").rest(
            "rest",
            SyntaxShape::ColumnPath,
            "column paths to convert to filesize (for table input)",
        )
    }

    fn usage(&self) -> &str {
        "Convert value to filesize"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        into_filesize(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Convert string to filesize in table",
                example: "echo [[bytes]; ['5'] [3.2] [4] [2kb]] | into filesize bytes",
                result: Some(vec![
                    UntaggedValue::row(indexmap! {
                        "bytes".to_string() => UntaggedValue::filesize(5).into(),
                    })
                    .into(),
                    UntaggedValue::row(indexmap! {
                        "bytes".to_string() => UntaggedValue::filesize(3).into(),
                    })
                    .into(),
                    UntaggedValue::row(indexmap! {
                        "bytes".to_string() => UntaggedValue::filesize(4).into(),
                    })
                    .into(),
                    UntaggedValue::row(indexmap! {
                        "bytes".to_string() => UntaggedValue::filesize(2000).into(),
                    })
                    .into(),
                ]),
            },
            Example {
                description: "Convert string to filesize",
                example: "echo '2' | into filesize",
                result: Some(vec![UntaggedValue::filesize(2).into()]),
            },
            Example {
                description: "Convert decimal to filesize",
                example: "echo 8.3 | into filesize",
                result: Some(vec![UntaggedValue::filesize(8).into()]),
            },
            Example {
                description: "Convert int to filesize",
                example: "echo 5 | into filesize",
                result: Some(vec![UntaggedValue::filesize(5).into()]),
            },
            Example {
                description: "Convert file size to filesize",
                example: "echo 4KB | into filesize",
                result: Some(vec![UntaggedValue::filesize(4000).into()]),
            },
        ]
    }
}

fn into_filesize(args: CommandArgs) -> Result<OutputStream, ShellError> {
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
        UntaggedValue::Primitive(prim) => Ok(UntaggedValue::filesize(match prim {
            Primitive::String(a_string) => match int_from_string(a_string.trim(), &tag) {
                Ok(n) => n,
                Err(e) => {
                    return Err(e);
                }
            },
            Primitive::Decimal(dec) => match dec.to_bigint() {
                Some(n) => match n.to_u64() {
                    Some(i) => i,
                    None => {
                        return Err(ShellError::unimplemented(
                            "failed to convert decimal to filesize",
                        ));
                    }
                },
                None => {
                    return Err(ShellError::unimplemented(
                        "failed to convert decimal to filesize",
                    ));
                }
            },
            Primitive::Int(n_ref) => (*n_ref).try_into().map_err(|_| {
                ShellError::unimplemented("cannot convert negative integer to filesize")
            })?,
            Primitive::Filesize(a_filesize) => *a_filesize,
            _ => {
                return Err(ShellError::unimplemented(
                    "'into filesize' for non-numeric primitives",
                ))
            }
        })
        .into_value(&tag)),
        UntaggedValue::Row(_) => Err(ShellError::labeled_error(
            "specify column name to use, with 'into filesize COLUMN'",
            "found table",
            tag,
        )),
        _ => Err(ShellError::unimplemented(
            "'into filesize' for unsupported type",
        )),
    }
}

fn int_from_string(a_string: &str, tag: &Tag) -> Result<u64, ShellError> {
    match a_string.parse::<u64>() {
        Ok(n) => Ok(n),
        Err(_) => match a_string.parse::<f64>() {
            Ok(f) => match f.to_u64() {
                Some(i) => Ok(i),
                None => Err(ShellError::labeled_error(
                    "Could not convert string value to filesize",
                    "original value",
                    tag.clone(),
                )),
            },
            Err(_) => Err(ShellError::labeled_error(
                "Could not convert string value to filesize",
                "original value",
                tag.clone(),
            )),
        },
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
