use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use crate::TaggedListBuilder;
use calamine::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, TaggedDictBuilder, UntaggedValue};
use std::io::Cursor;

pub struct FromODS;

#[derive(Deserialize)]
pub struct FromODSArgs {
    headerless: bool,
}

#[async_trait]
impl WholeStreamCommand for FromODS {
    fn name(&self) -> &str {
        "from ods"
    }

    fn signature(&self) -> Signature {
        Signature::build("from ods").switch(
            "headerless",
            "don't treat the first row as column names",
            None,
        )
    }

    fn usage(&self) -> &str {
        "Parse OpenDocument Spreadsheet(.ods) data and create table."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        from_ods(args, registry).await
    }
}

async fn from_ods(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let registry = registry.clone();

    let (
        FromODSArgs {
            headerless: _headerless,
        },
        input,
    ) = args.process(&registry).await?;
    let bytes = input.collect_binary(tag.clone()).await?;
    let buf: Cursor<Vec<u8>> = Cursor::new(bytes.item);
    let mut ods = Ods::<_>::new(buf).map_err(|_| {
        ShellError::labeled_error("Could not load ods file", "could not load ods file", &tag)
    })?;

    let mut dict = TaggedDictBuilder::new(&tag);

    let sheet_names = ods.sheet_names().to_owned();

    for sheet_name in &sheet_names {
        let mut sheet_output = TaggedListBuilder::new(&tag);

        if let Some(Ok(current_sheet)) = ods.worksheet_range(sheet_name) {
            for row in current_sheet.rows() {
                let mut row_output = TaggedDictBuilder::new(&tag);
                for (i, cell) in row.iter().enumerate() {
                    let value = match cell {
                        DataType::Empty => UntaggedValue::nothing(),
                        DataType::String(s) => UntaggedValue::string(s),
                        DataType::Float(f) => UntaggedValue::decimal(*f),
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
    use super::FromODS;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(FromODS {})
    }
}
