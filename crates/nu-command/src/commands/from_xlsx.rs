use crate::prelude::*;
use calamine::*;
use nu_data::TaggedListBuilder;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, TaggedDictBuilder, UntaggedValue};
use std::io::Cursor;

pub struct FromXLSX;

#[derive(Deserialize)]
pub struct FromXLSXArgs {
    noheaders: bool,
}

#[async_trait]
impl WholeStreamCommand for FromXLSX {
    fn name(&self) -> &str {
        "from xlsx"
    }

    fn signature(&self) -> Signature {
        Signature::build("from xlsx").switch(
            "noheaders",
            "don't treat the first row as column names",
            Some('n'),
        )
    }

    fn usage(&self) -> &str {
        "Parse binary Excel(.xlsx) data and create table."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        from_xlsx(args).await
    }
}

async fn from_xlsx(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let span = tag.span;
    let (
        FromXLSXArgs {
            noheaders: _noheaders,
        },
        input,
    ) = args.process().await?;
    let value = input.collect_binary(tag.clone()).await?;

    let buf: Cursor<Vec<u8>> = Cursor::new(value.item);
    let mut xls = Xlsx::<_>::new(buf).map_err(|_| {
        ShellError::labeled_error("Could not load xlsx file", "could not load xlsx file", &tag)
    })?;

    let mut dict = TaggedDictBuilder::new(&tag);

    let sheet_names = xls.sheet_names().to_owned();

    for sheet_name in &sheet_names {
        let mut sheet_output = TaggedListBuilder::new(&tag);

        if let Some(Ok(current_sheet)) = xls.worksheet_range(sheet_name) {
            for row in current_sheet.rows() {
                let mut row_output = TaggedDictBuilder::new(&tag);
                for (i, cell) in row.iter().enumerate() {
                    let value = match cell {
                        DataType::Empty => UntaggedValue::nothing(),
                        DataType::String(s) => UntaggedValue::string(s),
                        DataType::Float(f) => UntaggedValue::decimal_from_float(*f, span),
                        DataType::Int(i) => UntaggedValue::int(*i),
                        DataType::Bool(b) => UntaggedValue::boolean(*b),
                        _ => UntaggedValue::nothing(),
                    };

                    row_output.insert_untagged(&format!("Column{}", i), value);
                }

                sheet_output.push_untagged(row_output.into_untagged_value());
            }

            dict.insert_untagged(sheet_name, sheet_output.into_untagged_value());
        } else {
            return Err(ShellError::labeled_error(
                "Could not load sheet",
                "could not load sheet",
                &tag,
            ));
        }
    }

    Ok(OutputStream::one(ReturnSuccess::value(dict.into_value())))
}

#[cfg(test)]
mod tests {
    use super::FromXLSX;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(FromXLSX {})
    }
}
