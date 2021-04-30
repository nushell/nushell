use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    ColumnPath, Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};
use num_bigint::{BigInt, ToBigInt};

pub struct SubCommand;

#[derive(Deserialize)]
pub struct Arguments {
    pub rest: Vec<ColumnPath>,
}

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "into binary"
    }

    fn signature(&self) -> Signature {
        Signature::build("into binary").rest(
            SyntaxShape::ColumnPath,
            "column paths to convert to binary (for table input)",
        )
    }

    fn usage(&self) -> &str {
        "Convert value to a binary primitive"
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        into_binary(args)
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

fn into_binary(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let (Arguments { rest: column_paths }, input) = args.process()?;

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
        .to_action_stream())
}

pub fn action(input: &Value, tag: impl Into<Tag>) -> Result<Value, ShellError> {
    let tag = tag.into();
    match &input.value {
        UntaggedValue::Primitive(prim) => Ok(UntaggedValue::binary(match prim {
            Primitive::Binary(b) => b.to_vec(),
            // TODO: Several places here we use Little Endian. We should probably
            // query the host to determine if it's Big Endian or Little Endian
            Primitive::Int(n_ref) => n_ref.to_bytes_le().1,
            Primitive::Decimal(dec) => match dec.to_bigint() {
                Some(n) => n.to_bytes_le().1,
                None => {
                    return Err(ShellError::unimplemented(
                        "failed to convert decimal to int",
                    ));
                }
            },
            Primitive::Filesize(a_filesize) => match a_filesize.to_bigint() {
                Some(n) => n.to_bytes_le().1,
                None => {
                    return Err(ShellError::unimplemented(
                        "failed to convert filesize to bigint",
                    ));
                }
            },
            Primitive::String(a_string) => a_string.as_bytes().to_vec(),
            Primitive::Boolean(a_bool) => match a_bool {
                false => BigInt::from(0).to_bytes_le().1,
                true => BigInt::from(1).to_bytes_le().1,
            },
            Primitive::Date(a_date) => a_date.format("%c").to_string().as_bytes().to_vec(),
            Primitive::FilePath(a_filepath) => a_filepath
                .as_path()
                .display()
                .to_string()
                .as_bytes()
                .to_vec(),
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
