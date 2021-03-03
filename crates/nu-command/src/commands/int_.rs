use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    ColumnPath, Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};

use num_bigint::{BigInt, ToBigInt};

pub struct Int_;

#[derive(Deserialize)]
pub struct IntArgs {
    pub rest: Vec<ColumnPath>,
}

#[async_trait]
impl WholeStreamCommand for Int_ {
    fn name(&self) -> &str {
        "int"
    }

    fn signature(&self) -> Signature {
        Signature::build("int").rest(
            SyntaxShape::ColumnPath,
            "column paths to convert to int (for table input)",
        )
    }

    fn usage(&self) -> &str {
        "Convert value to integer"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        int_(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Convert string to integer in table",
                example: "echo [[num]; ['-5'] [4] [1.5]] | int num",
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
                example: "echo '2' | int",
                result: Some(vec![UntaggedValue::int(2).into()]),
            },
            Example {
                description: "Convert decimal to integer",
                example: "echo 5.9 | int",
                result: Some(vec![UntaggedValue::int(5).into()]),
            },
            Example {
                description: "Convert decimal string to integer",
                example: "echo '5.9' | int",
                result: Some(vec![UntaggedValue::int(5).into()]),
            },
            Example {
                description: "Convert file size to integer",
                example: "echo 4KB | int",
                result: Some(vec![UntaggedValue::int(4000).into()]),
            },
            Example {
                description: "Convert bool to integer",
                example: "echo $false $true | int",
                result: Some(vec![
                    UntaggedValue::int(0).into(),
                    UntaggedValue::int(1).into(),
                ]),
            },
        ]
    }
}

async fn int_(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let (IntArgs { rest: column_paths }, input) = args.process().await?;

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
            Primitive::Int(n_ref) => n_ref.to_bigint().expect("unexpected error"),
            Primitive::Boolean(a_bool) => match a_bool {
                false => 0.to_bigint().expect("unexpected error"),
                true => 1.to_bigint().expect("unexpected error"),
            },
            Primitive::Filesize(a_filesize) => a_filesize
                .to_bigint()
                .expect("Conversion should never fail."),
            _ => {
                return Err(ShellError::unimplemented(
                    "'int' for non-numeric primitives",
                ))
            }
        })
        .into_value(&tag)),
        UntaggedValue::Row(_) => Err(ShellError::labeled_error(
            "specify column name to use, with 'int COLUMN'",
            "found table",
            tag,
        )),
        _ => Err(ShellError::unimplemented("'int' for unsupported type")),
    }
}

fn int_from_string(a_string: &str, tag: &Tag) -> Result<BigInt, ShellError> {
    match a_string.parse::<BigInt>() {
        Ok(n) => Ok(n),
        Err(_) => match a_string.parse::<f64>() {
            Ok(res_float) => match res_float.to_bigint() {
                Some(n) => Ok(n),
                None => Err(ShellError::unimplemented(
                    "failed to convert decimal to int",
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
    use super::Int_;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Int_ {})
    }
}
