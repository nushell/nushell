use crate::prelude::*;
use nu_errors::ShellError;

use nu_engine::WholeStreamCommand;
use nu_protocol::{
    ColumnPath, Primitive::Filesize, ReturnSuccess, Signature, SyntaxShape, UntaggedValue,
    UntaggedValue::Primitive, Value,
};
use nu_source::Tagged;
use nu_value_ext::get_data_by_column_path;

use num_format::{Locale, ToFormattedString};

pub struct FileSize;

#[derive(Deserialize)]
pub struct Arguments {
    field: ColumnPath,
    format: Tagged<String>,
}

#[async_trait]
impl WholeStreamCommand for FileSize {
    fn name(&self) -> &str {
        "format filesize"
    }

    fn signature(&self) -> Signature {
        Signature::build("format filesize")
            .required(
                "field",
                SyntaxShape::ColumnPath,
                "the name of the column to update",
            )
            .required(
                "format value",
                SyntaxShape::String,
                "the format into which convert the filesizes",
            )
    }

    fn usage(&self) -> &str {
        "Converts a column of filesizes to some specified format"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        filesize(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Convert the size row to KB",
                example: "ls | format filesize size KB",
                result: None,
            },
            Example {
                description: "Convert the apparent row to B",
                example: "du | format filesize apparent B",
                result: None,
            },
        ]
    }
}

async fn process_row(
    input: Value,
    format: Tagged<String>,
    field: Arc<ColumnPath>,
) -> Result<OutputStream, ShellError> {
    Ok({
        let replace_for = get_data_by_column_path(&input, &field, move |_, _, error| error);
        match replace_for {
            Ok(s) => match convert_bytes_to_string_using_format(s, format) {
                Ok(b) => OutputStream::one(ReturnSuccess::value(
                    input.replace_data_at_column_path(&field, b).expect("Given that the existence check was already done, this shouldn't trigger never"),
                )),
                Err(e) => OutputStream::one(Err(e)),
            },
            Err(e) => OutputStream::one(Err(e)),
        }
    })
}

async fn filesize(raw_args: CommandArgs) -> Result<OutputStream, ShellError> {
    let (Arguments { field, format }, input) = raw_args.process().await?;
    let field = Arc::new(field);

    Ok(input
        .then(move |input| {
            let format = format.clone();
            let field = field.clone();

            async {
                match process_row(input, format, field).await {
                    Ok(s) => s,
                    Err(e) => OutputStream::one(Err(e)),
                }
            }
        })
        .flatten()
        .to_output_stream())
}

fn convert_bytes_to_string_using_format(
    bytes: Value,
    format: Tagged<String>,
) -> Result<Value, ShellError> {
    match bytes.value {
        Primitive(Filesize(b)) => {
            if let Some(value) = b.to_u128() {
                let byte = byte_unit::Byte::from_bytes(value);
                let value = match format.item().to_lowercase().as_str() {
                    "b" => Ok(UntaggedValue::string(
                        value.to_formatted_string(&Locale::en),
                    )),
                    "kb" => Ok(UntaggedValue::string(
                        byte.get_adjusted_unit(byte_unit::ByteUnit::KB).to_string(),
                    )),
                    "kib" => Ok(UntaggedValue::string(
                        byte.get_adjusted_unit(byte_unit::ByteUnit::KiB).to_string(),
                    )),
                    "mb" => Ok(UntaggedValue::string(
                        byte.get_adjusted_unit(byte_unit::ByteUnit::MB).to_string(),
                    )),
                    "mib" => Ok(UntaggedValue::string(
                        byte.get_adjusted_unit(byte_unit::ByteUnit::MiB).to_string(),
                    )),
                    "gb" => Ok(UntaggedValue::string(
                        byte.get_adjusted_unit(byte_unit::ByteUnit::GB).to_string(),
                    )),
                    "gib" => Ok(UntaggedValue::string(
                        byte.get_adjusted_unit(byte_unit::ByteUnit::GiB).to_string(),
                    )),
                    "tb" => Ok(UntaggedValue::string(
                        byte.get_adjusted_unit(byte_unit::ByteUnit::TB).to_string(),
                    )),
                    "tib" => Ok(UntaggedValue::string(
                        byte.get_adjusted_unit(byte_unit::ByteUnit::TiB).to_string(),
                    )),
                    "pb" => Ok(UntaggedValue::string(
                        byte.get_adjusted_unit(byte_unit::ByteUnit::PB).to_string(),
                    )),
                    "pib" => Ok(UntaggedValue::string(
                        byte.get_adjusted_unit(byte_unit::ByteUnit::PiB).to_string(),
                    )),
                    "eb" => Ok(UntaggedValue::string(
                        byte.get_adjusted_unit(byte_unit::ByteUnit::EB).to_string(),
                    )),
                    "eib" => Ok(UntaggedValue::string(
                        byte.get_adjusted_unit(byte_unit::ByteUnit::EiB).to_string(),
                    )),
                    "zb" => Ok(UntaggedValue::string(
                        byte.get_adjusted_unit(byte_unit::ByteUnit::ZB).to_string(),
                    )),
                    "zib" => Ok(UntaggedValue::string(
                        byte.get_adjusted_unit(byte_unit::ByteUnit::ZiB).to_string(),
                    )),
                    _ => Err(ShellError::labeled_error(
                        format!("Invalid format code: {:}", format.item()),
                        "invalid format",
                        format.tag(),
                    )),
                };
                match value {
                    Ok(b) => Ok(Value { value: b, ..bytes }),
                    Err(e) => Err(e),
                }
            } else {
                Err(ShellError::labeled_error(
                    "Value too large to fit in 128 bits",
                    "value too large to fit in format",
                    format.span(),
                ))
            }
        }
        _ => Err(ShellError::labeled_error(
            "the data in this row is not of the type filesize",
            "invalid row type",
            bytes.tag(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::FileSize;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        Ok(test_examples(FileSize {})?)
    }
}
