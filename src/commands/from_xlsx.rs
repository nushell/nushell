use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use crate::TaggedListBuilder;
use calamine::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, TaggedDictBuilder, UntaggedValue};
use std::io::Cursor;

pub struct FromXLSX;

#[derive(Deserialize)]
pub struct FromXLSXArgs {
    headerless: bool,
}

impl WholeStreamCommand for FromXLSX {
    fn name(&self) -> &str {
        "from-xlsx"
    }

    fn signature(&self) -> Signature {
        Signature::build("from-xlsx").switch(
            "headerless",
            "don't treat the first row as column names",
            None,
        )
    }

    fn usage(&self) -> &str {
        "Parse binary Excel(.xlsx) data and create table."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, from_xlsx)?.run()
    }
}

fn from_xlsx(
    FromXLSXArgs {
        headerless: _headerless,
    }: FromXLSXArgs,
    runnable_context: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let input = runnable_context.input;
    let tag = runnable_context.name;

    let stream = async_stream! {
        let value = input.collect_binary(tag.clone()).await?;

        let mut buf: Cursor<Vec<u8>> = Cursor::new(value.item);
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
                yield Err(ShellError::labeled_error(
                    "Could not load sheet",
                    "could not load sheet",
                    &tag,
                ));
            }
        }

        yield ReturnSuccess::value(dict.into_value());
    };

    Ok(stream.to_output_stream())
}
