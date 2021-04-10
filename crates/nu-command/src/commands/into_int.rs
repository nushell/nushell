use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    ColumnPath, Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};

use num_bigint::{BigInt, ToBigInt};

pub struct IntoInt;

#[derive(Deserialize)]
pub struct IntoIntArgs {
    pub rest: Vec<ColumnPath>,
}

impl WholeStreamCommand for IntoInt {
    fn name(&self) -> &str {
        "into int"
    }

    fn signature(&self) -> Signature {
        Signature::build("into int").rest(
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
    let (IntoIntArgs { rest: column_paths }, input) = args.process()?;

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
                Some(n) => n,
                None => {
                    return Err(ShellError::unimplemented(
                        "failed to convert decimal to int",
                    ));
                }
            },
            Primitive::Int(n_ref) => BigInt::from(n_ref.to_owned()),
            Primitive::Boolean(a_bool) => match a_bool {
                false => BigInt::from(0),
                true => BigInt::from(1),
            },
            Primitive::Filesize(a_filesize) => match a_filesize.to_bigint() {
                Some(n) => n,
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

fn int_from_string(a_string: &str, tag: &Tag) -> Result<BigInt, ShellError> {
    match a_string.parse::<BigInt>() {
        Ok(n) => Ok(n),
        Err(_) => match a_string.parse::<f64>() {
            Ok(res_float) => match res_float.to_bigint() {
                Some(n) => Ok(n),
                None => Err(ShellError::unimplemented("failed to convert f64 to int")),
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
    use super::IntoInt;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(IntoInt {})
    }
}
