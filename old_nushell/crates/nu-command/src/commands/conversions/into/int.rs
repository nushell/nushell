use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Primitive, Signature, SyntaxShape, UntaggedValue, Value};
use num_bigint::ToBigInt;

pub struct SubCommand;

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "into int"
    }

    fn signature(&self) -> Signature {
        Signature::build("into int").rest(
            "rest",
            SyntaxShape::ColumnPath,
            "column paths to convert to int (for table input)",
        )
    }

    fn usage(&self) -> &str {
        "Convert value to integer"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        into_int(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Convert string to integer in table",
                example: "echo [[num]; ['-5'] [4] [1.5]] | into int num",
                result: Some(vec![
                    UntaggedValue::row(indexmap! {
                        "num".to_string() => UntaggedValue::int(-5).into(),
                    })
                    .into(),
                    UntaggedValue::row(indexmap! {
                        "num".to_string() => UntaggedValue::int(4).into(),
                    })
                    .into(),
                    UntaggedValue::row(indexmap! {
                        "num".to_string() => UntaggedValue::int(1).into(),
                    })
                    .into(),
                ]),
            },
            Example {
                description: "Convert string to integer",
                example: "echo '2' | into int",
                result: Some(vec![UntaggedValue::int(2).into()]),
            },
            Example {
                description: "Convert decimal to integer",
                example: "echo 5.9 | into int",
                result: Some(vec![UntaggedValue::int(5).into()]),
            },
            Example {
                description: "Convert decimal string to integer",
                example: "echo '5.9' | into int",
                result: Some(vec![UntaggedValue::int(5).into()]),
            },
            Example {
                description: "Convert file size to integer",
                example: "echo 4KB | into int",
                result: Some(vec![UntaggedValue::int(4000).into()]),
            },
            Example {
                description: "Convert bool to integer",
                example: "echo $false $true | into int",
                result: Some(vec![
                    UntaggedValue::int(0).into(),
                    UntaggedValue::int(1).into(),
                ]),
            },
        ]
    }
}

fn into_int(args: CommandArgs) -> Result<OutputStream, ShellError> {
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
        UntaggedValue::Primitive(prim) => Ok(UntaggedValue::int(match prim {
            Primitive::String(a_string) => match int_from_string(a_string, &tag) {
                Ok(n) => n,
                Err(e) => {
                    return Err(e);
                }
            },
            Primitive::Decimal(dec) => match dec.to_bigint() {
                Some(n) => match n.to_i64() {
                    Some(i) => i,
                    None => {
                        return Err(ShellError::unimplemented(
                            "failed to convert decimal to int",
                        ));
                    }
                },
                None => {
                    return Err(ShellError::unimplemented(
                        "failed to convert decimal to int",
                    ));
                }
            },
            Primitive::Int(n_ref) => *n_ref,
            Primitive::Boolean(a_bool) => match a_bool {
                false => 0,
                true => 1,
            },
            Primitive::Filesize(a_filesize) => match a_filesize.to_bigint() {
                Some(n) => match n.to_i64() {
                    Some(i) => i,
                    None => {
                        return Err(ShellError::unimplemented(
                            "failed to convert filesize to bigint",
                        ));
                    }
                },
                None => {
                    return Err(ShellError::unimplemented(
                        "failed to convert filesize to bigint",
                    ));
                }
            },
            _ => {
                return Err(ShellError::unimplemented(
                    "'into int' for non-numeric primitives",
                ))
            }
        })
        .into_value(&tag)),
        UntaggedValue::Row(_) => Err(ShellError::labeled_error(
            "specify column name to use, with 'into int COLUMN'",
            "found table",
            tag,
        )),
        _ => Err(ShellError::unimplemented("'into int' for unsupported type")),
    }
}

fn int_from_string(a_string: &str, tag: &Tag) -> Result<i64, ShellError> {
    match a_string.parse::<i64>() {
        Ok(n) => Ok(n),
        Err(_) => match a_string.parse::<f64>() {
            Ok(f) => match f.to_i64() {
                Some(i) => Ok(i),
                None => Err(ShellError::labeled_error(
                    "Could not convert string value to int",
                    "original value",
                    tag.clone(),
                )),
            },
            Err(_) => Err(ShellError::labeled_error(
                "Could not convert string value to int",
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
