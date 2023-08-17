use calamine::*;
use indexmap::map::IndexMap;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SpannedValue, SyntaxShape, Type,
};
use std::io::Cursor;

#[derive(Clone)]
pub struct FromXlsx;

impl Command for FromXlsx {
    fn name(&self) -> &str {
        "from xlsx"
    }

    fn signature(&self) -> Signature {
        Signature::build("from xlsx")
            .input_output_types(vec![(Type::Binary, Type::Table(vec![]))])
            .allow_variants_without_examples(true)
            .named(
                "sheets",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "Only convert specified sheets",
                Some('s'),
            )
            .category(Category::Formats)
    }

    fn usage(&self) -> &str {
        "Parse binary Excel(.xlsx) data and create table."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;

        let sel_sheets = if let Some(SpannedValue::List { vals: columns, .. }) =
            call.get_flag(engine_state, stack, "sheets")?
        {
            convert_columns(columns.as_slice(), call.head)?
        } else {
            vec![]
        };

        from_xlsx(input, head, sel_sheets)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Convert binary .xlsx data to a table",
                example: "open --raw test.xlsx | from xlsx",
                result: None,
            },
            Example {
                description: "Convert binary .xlsx data to a table, specifying the tables",
                example: "open --raw test.xlsx | from xlsx -s [Spreadsheet1]",
                result: None,
            },
        ]
    }
}

fn convert_columns(columns: &[SpannedValue], span: Span) -> Result<Vec<String>, ShellError> {
    let res = columns
        .iter()
        .map(|value| match &value {
            SpannedValue::String { val: s, .. } => Ok(s.clone()),
            _ => Err(ShellError::IncompatibleParametersSingle {
                msg: "Incorrect column format, Only string as column name".to_string(),
                span: value.span(),
            }),
        })
        .collect::<Result<Vec<String>, _>>()?;

    Ok(res)
}

fn collect_binary(input: PipelineData, span: Span) -> Result<Vec<u8>, ShellError> {
    let mut bytes = vec![];
    let mut values = input.into_iter();

    loop {
        match values.next() {
            Some(SpannedValue::Binary { val: b, .. }) => {
                bytes.extend_from_slice(&b);
            }
            Some(x) => {
                return Err(ShellError::UnsupportedInput(
                    "Expected binary from pipeline".to_string(),
                    "value originates from here".into(),
                    span,
                    x.span(),
                ))
            }
            None => break,
        }
    }

    Ok(bytes)
}

fn from_xlsx(
    input: PipelineData,
    head: Span,
    sel_sheets: Vec<String>,
) -> Result<PipelineData, ShellError> {
    let span = input.span();
    let bytes = collect_binary(input, head)?;
    let buf: Cursor<Vec<u8>> = Cursor::new(bytes);
    let mut xlsx = Xlsx::<_>::new(buf).map_err(|_| {
        ShellError::UnsupportedInput(
            "Could not load XLSX file".to_string(),
            "value originates from here".into(),
            head,
            span.unwrap_or(head),
        )
    })?;

    let mut dict = IndexMap::new();

    let mut sheet_names = xlsx.sheet_names().to_owned();
    if !sel_sheets.is_empty() {
        sheet_names.retain(|e| sel_sheets.contains(e));
    }

    for sheet_name in &sheet_names {
        let mut sheet_output = vec![];

        if let Some(Ok(current_sheet)) = xlsx.worksheet_range(sheet_name) {
            for row in current_sheet.rows() {
                let mut row_output = IndexMap::new();
                for (i, cell) in row.iter().enumerate() {
                    let value = match cell {
                        DataType::Empty => SpannedValue::nothing(head),
                        DataType::String(s) => SpannedValue::string(s, head),
                        DataType::Float(f) => SpannedValue::float(*f, head),
                        DataType::Int(i) => SpannedValue::int(*i, head),
                        DataType::Bool(b) => SpannedValue::bool(*b, head),
                        _ => SpannedValue::nothing(head),
                    };

                    row_output.insert(format!("column{i}"), value);
                }

                let (cols, vals) =
                    row_output
                        .into_iter()
                        .fold((vec![], vec![]), |mut acc, (k, v)| {
                            acc.0.push(k);
                            acc.1.push(v);
                            acc
                        });

                let record = SpannedValue::Record {
                    cols,
                    vals,
                    span: head,
                };

                sheet_output.push(record);
            }

            dict.insert(
                sheet_name,
                SpannedValue::List {
                    vals: sheet_output,
                    span: head,
                },
            );
        } else {
            return Err(ShellError::UnsupportedInput(
                "Could not load sheet".to_string(),
                "value originates from here".into(),
                head,
                span.unwrap_or(head),
            ));
        }
    }

    let (cols, vals) = dict.into_iter().fold((vec![], vec![]), |mut acc, (k, v)| {
        acc.0.push(k.clone());
        acc.1.push(v);
        acc
    });

    let record = SpannedValue::Record {
        cols,
        vals,
        span: head,
    };

    Ok(PipelineData::Value(record, None))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(FromXlsx {})
    }
}
