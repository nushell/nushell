use std::io::Cursor;

use calamine::*;
use chrono::{
    Local, NaiveDateTime, Offset as _, TimeDelta, TimeZone as _, Utc, offset::LocalResult,
};

use nu_engine::command_prelude::*;

pub(super) fn common_sheets_signature(name: &str) -> Signature {
    Signature::build(name)
        .input_output_types(vec![(Type::Binary, Type::record())])
        .allow_variants_without_examples(true)
        .named(
            "sheets",
            SyntaxShape::List(Box::new(SyntaxShape::String)),
            "Only convert specified sheets.",
            Some('s'),
        )
        .switch(
            "noheaders",
            "Don't treat the first row as column names.",
            Some('n'),
        )
        .named(
            "first-row",
            SyntaxShape::Int,
            "The row to start reading the sheets from. \
                By default, reading starts from the firsts non empty row.",
            Some('f'),
        )
        .category(Category::Formats)
}

pub(super) fn collect_binary(
    input: PipelineData,
    head: Span,
) -> Result<Cursor<Vec<u8>>, ShellError> {
    let buf = match input {
        // Deserialize from a byte buffer
        PipelineData::Value(Value::Binary { val: bytes, .. }, _) => Ok(bytes),
        // Deserialize from a raw stream directly without having to collect it
        PipelineData::ByteStream(stream, ..) => stream.into_bytes(),
        input => Err(ShellError::PipelineMismatch {
            exp_input_type: "binary or byte stream".into(),
            dst_span: head,
            src_span: input.span().unwrap_or(head),
        }),
    };
    Ok(Cursor::new(buf?))
}

pub(super) fn from_sheets(
    mut sheets: Sheets<Cursor<Vec<u8>>>,
    input_span: Span,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> std::result::Result<PipelineData, ShellError> {
    let head = call.head;

    let noheaders = call.has_flag(engine_state, stack, "noheaders")?;
    let first_row = call
        .get_flag::<u32>(engine_state, stack, "first-row")?
        .map(HeaderRow::Row)
        .unwrap_or(HeaderRow::FirstNonEmptyRow);
    let sheet_names = {
        let sel_sheets = call
            .get_flag::<Vec<String>>(engine_state, stack, "sheets")?
            .unwrap_or_default();
        let mut sheets = sheets.sheet_names();
        if !sel_sheets.is_empty() {
            sheets.retain(|e| sel_sheets.contains(e));
        }
        sheets
    };

    let tz = match Local.timestamp_opt(0, 0) {
        LocalResult::Single(tz) => *tz.offset(),
        _ => Utc.fix(),
    };

    sheets.with_header_row(first_row);

    let output = sheet_names
        .into_iter()
        .map(|name| {
            let sheet =
                sheets
                    .worksheet_range(&name)
                    .map_err(|_| ShellError::UnsupportedInput {
                        msg: "Could not load sheet".to_string(),
                        input: "value originates from here".into(),
                        msg_span: head,
                        input_span,
                    })?;

            let rows = sheet
                .rows()
                .map(|row| row.iter().map(|cell| cell_to_data(cell, head, tz)));

            if !noheaders && let Some(headers) = sheet.headers() {
                let headers = headers
                    .into_iter()
                    .chain(std::iter::repeat(String::new()))
                    .enumerate()
                    .map(|(idx, s)| {
                        if s.is_empty() {
                            format!("column{idx}")
                        } else {
                            s
                        }
                    });

                // the original iterator must remain immutable. can only be used by cloning
                let headers = &headers;

                let rows = rows
                    .skip(1)
                    .map(|row| {
                        headers
                            .clone()
                            .zip(row)
                            .collect::<Record>()
                            .into_value(head)
                    })
                    .collect::<Vec<_>>();
                Ok((name, rows.into_value(head)))
            } else {
                let rows = rows
                    .map(|row| {
                        row.enumerate()
                            .map(|(idx, value)| (format!("column{idx}"), value))
                            .collect::<Record>()
                            .into_value(head)
                    })
                    .collect::<Vec<_>>();
                Ok((name, rows.into_value(head)))
            }
        })
        .collect::<Result<Record, ShellError>>()?;

    Ok(output.into_value(head).into_pipeline_data())
}

fn cell_to_data(cell: &Data, head: Span, tz: chrono::FixedOffset) -> Value {
    match cell {
        Data::Empty => Value::nothing(head),
        Data::Int(val) => Value::int(*val, head),
        Data::Float(val) => {
            if *val as i64 as f64 == *val {
                Value::int(*val as i64, head)
            } else {
                Value::float(*val, head)
            }
        }
        Data::String(val) => Value::string(val, head),
        Data::Bool(val) => Value::bool(*val, head),
        Data::DateTime(d) => excel_datetime_to_value(d, tz, head),
        d @ Data::DateTimeIso(_) => d
            .as_datetime()
            .map(|naive_datetime| datetime_naive_to_fixed(naive_datetime, tz, head))
            .unwrap_or(Value::nothing(head)),
        d @ Data::DurationIso(_) => d
            .as_duration()
            .map(|time_delta| timedelta_to_value(time_delta, head))
            .unwrap_or(Value::nothing(head)),

        // Not great.
        Data::Error(_) => Value::nothing(head),
    }
}

fn datetime_naive_to_fixed(
    naive_datetime: NaiveDateTime,
    tz: chrono::FixedOffset,
    span: Span,
) -> Value {
    match tz.from_local_datetime(&naive_datetime) {
        LocalResult::Single(d) => d.into_value(span),
        LocalResult::Ambiguous(_, d) => d.into_value(span),
        _ => Value::nothing(span),
    }
}

fn timedelta_to_value(time_delta: TimeDelta, span: Span) -> Value {
    time_delta
        .num_nanoseconds()
        .map(|val| Value::duration(val, span))
        .unwrap_or(Value::nothing(span))
}

fn excel_datetime_to_value(
    excel_datetime: &ExcelDateTime,
    tz: chrono::FixedOffset,
    span: Span,
) -> Value {
    match excel_datetime {
        d if let Some(naive_datetime) = d.as_datetime() => {
            datetime_naive_to_fixed(naive_datetime, tz, span)
        }
        d if let Some(time_delta) = d.as_duration() => timedelta_to_value(time_delta, span),
        _ => Value::nothing(span),
    }
}
