use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Primitive, Signature, SyntaxShape, UntaggedValue, Value};
use num_bigint::{BigInt, ToBigInt};

pub struct SubCommand;

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "into binary"
    }

    fn signature(&self) -> Signature {
        Signature::build("into binary").rest(
            "rest",
            SyntaxShape::ColumnPath,
            "column paths to convert to binary (for table input)",
        )
    }

    fn usage(&self) -> &str {
        "Convert value to a binary primitive"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        into_binary(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "convert string to a nushell binary primitive",
                example:
                    "echo 'This is a string that is exactly 52 characters long.' | into binary",
                result: Some(vec![UntaggedValue::binary(
                    "This is a string that is exactly 52 characters long."
                        .to_string()
                        .as_bytes()
                        .to_vec(),
                )
                .into()]),
            },
            Example {
                description: "convert a number to a nushell binary primitive",
                example: "echo 1 | into binary",
                result: Some(vec![UntaggedValue::binary(
                    i64::from(1).to_le_bytes().to_vec(),
                )
                .into()]),
            },
            Example {
                description: "convert a boolean to a nushell binary primitive",
                example: "echo $true | into binary",
                result: Some(vec![UntaggedValue::binary(
                    i64::from(1).to_le_bytes().to_vec(),
                )
                .into()]),
            },
            Example {
                description: "convert a filesize to a nushell binary primitive",
                example: "ls | where name == LICENSE | get size | into binary",
                result: None,
            },
            Example {
                description: "convert a filepath to a nushell binary primitive",
                example: "ls | where name == LICENSE | get name | path expand | into binary",
                result: None,
            },
            Example {
                description: "convert a decimal to a nushell binary primitive",
                example: "echo 1.234 | into binary",
                result: Some(vec![
                    UntaggedValue::binary(BigInt::from(1).to_bytes_le().1).into()
                ]),
            },
        ]
    }
}

fn into_binary(args: CommandArgs) -> Result<OutputStream, ShellError> {
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

fn int_to_endian(n: i64) -> Vec<u8> {
    if cfg!(target_endian = "little") {
        n.to_le_bytes().to_vec()
    } else {
        n.to_be_bytes().to_vec()
    }
}

fn bigint_to_endian(n: &BigInt) -> Vec<u8> {
    if cfg!(target_endian = "little") {
        n.to_bytes_le().1
    } else {
        n.to_bytes_be().1
    }
}

pub fn action(input: &Value, tag: impl Into<Tag>) -> Result<Value, ShellError> {
    let tag = tag.into();

    match &input.value {
        UntaggedValue::Primitive(prim) => Ok(UntaggedValue::binary(match prim {
            Primitive::Binary(b) => b.to_vec(),
            Primitive::Int(n_ref) => int_to_endian(*n_ref),
            Primitive::BigInt(n_ref) => bigint_to_endian(n_ref),
            Primitive::Decimal(dec) => match dec.to_bigint() {
                Some(n) => bigint_to_endian(&n),
                None => {
                    return Err(ShellError::unimplemented(
                        "failed to convert decimal to int",
                    ));
                }
            },
            Primitive::Filesize(a_filesize) => match a_filesize.to_bigint() {
                Some(n) => bigint_to_endian(&n),
                None => {
                    return Err(ShellError::unimplemented(
                        "failed to convert filesize to bigint",
                    ));
                }
            },
            Primitive::String(a_string) => a_string.as_bytes().to_vec(),
            Primitive::Boolean(a_bool) => match a_bool {
                false => int_to_endian(0),
                true => int_to_endian(1),
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
                    "'into binary' for non-numeric primitives",
                ))
            }
        })
        .into_value(&tag)),
        UntaggedValue::Row(_) => Err(ShellError::labeled_error(
            "specify column name to use, with 'into binary COLUMN'",
            "found table",
            tag,
        )),
        _ => Err(ShellError::unimplemented(
            "'into binary' for unsupported type",
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
