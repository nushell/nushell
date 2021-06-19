use crate::prelude::*;
use calamine::*;
use nu_data::TaggedListBuilder;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Primitive, Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue, Value};
use std::io::Cursor;

pub struct FromOds;

impl WholeStreamCommand for FromOds {
    fn name(&self) -> &str {
        "from ods"
    }

    fn signature(&self) -> Signature {
        Signature::build("from ods").named(
            "sheets",
            SyntaxShape::Table,
            "Only convert specified sheets",
            Some('s'),
        )
    }

    fn usage(&self) -> &str {
        "Parse OpenDocument Spreadsheet(.ods) data and create table."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        from_ods(args)
    }
}

// Adapted from crates/nu-command/src/commands/dataframe/utils.rs
fn convert_columns(columns: &[Value]) -> Result<Vec<String>, ShellError> {
    let res = columns
        .iter()
        .map(|value| match &value.value {
            UntaggedValue::Primitive(Primitive::String(s)) => Ok(s.clone()),
            _ => Err(ShellError::labeled_error(
                "Incorrect column format",
                "Only string as column name",
                &value.tag,
            )),
        })
        .collect::<Result<Vec<String>, _>>()?;

    Ok(res)
}

fn from_ods(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let span = tag.span;

    let mut sel_sheets = vec![];

    if let Some(columns) = args.get_flag::<Vec<Value>>("sheets")? {
        sel_sheets = convert_columns(columns.as_slice())?;
    }

    let bytes = args.input.collect_binary(tag.clone())?;
    let buf: Cursor<Vec<u8>> = Cursor::new(bytes.item);
    let mut ods = Ods::<_>::new(buf).map_err(|_| {
        ShellError::labeled_error("Could not load ods file", "could not load ods file", &tag)
    })?;

    let mut dict = TaggedDictBuilder::new(&tag);

    let mut sheet_names = ods.sheet_names().to_owned();
    if !sel_sheets.is_empty() {
        sheet_names.retain(|e| sel_sheets.contains(e));
    }

    for sheet_name in &sheet_names {
        let mut sheet_output = TaggedListBuilder::new(&tag);

        if let Some(Ok(current_sheet)) = ods.worksheet_range(sheet_name) {
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

    Ok(OutputStream::one(dict.into_value()))
}

#[cfg(test)]
mod tests {
    use super::FromOds;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(FromOds {})
    }
}
