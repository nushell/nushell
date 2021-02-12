use crate::prelude::*;
use nu_data::base::shape::InlineShape;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    ColumnPath, Primitive::Filesize, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::Tagged;
use nu_value_ext::get_data_by_column_path;

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
            Ok(s) => {
                if let Value {
                    value: UntaggedValue::Primitive(Filesize(fs)),
                    ..
                } = s
                {
                    let byte_format = InlineShape::format_bytes(&fs, Some(&format.item));
                    let byte_value = Value::from(byte_format.1);
                    OutputStream::one(ReturnSuccess::value(
                        input.replace_data_at_column_path(&field, byte_value).expect("Given that the existence check was already done, this shouldn't trigger never"),
                    ))
                } else {
                    return Err(ShellError::labeled_error(
                        "the data in this row is not of the type filesize",
                        "invalid datatype in row",
                        input.tag(),
                    ));
                }
            }
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

#[cfg(test)]
mod tests {
    use super::FileSize;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(FileSize {})
    }
}
