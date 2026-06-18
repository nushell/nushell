use std::io::Cursor;

use calamine::*;
use chrono::{
    Local, NaiveDateTime, Offset as _, TimeDelta, TimeZone as _, Utc, offset::LocalResult,
};

use nu_engine::command_prelude::*;

pub(super) fn collect_binary(input: PipelineData, head: Span) -> Result<Vec<u8>, ShellError> {
    match input {
        // Deserialize from a byte buffer
        PipelineData::Value(Value::Binary { val: bytes, .. }, _) => Ok(bytes),
        // Deserialize from a raw stream directly without having to collect it
        PipelineData::ByteStream(stream, ..) => stream.into_bytes(),
        input => Err(ShellError::PipelineMismatch {
            exp_input_type: "binary or byte stream".into(),
            dst_span: head,
            src_span: input.span().unwrap_or(head),
        }),
    }
}

pub(super) fn from_sheets(
    mut sheets: Sheets<Cursor<Vec<u8>>>,
    sel_sheets: Vec<String>,
    input_span: Span,
    head: Span,
) -> std::result::Result<PipelineData, ShellError> {
    let sheet_names = {
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
                .map(|row| {
                    row.iter()
                        .enumerate()
                        .map(|(idx, cell)| (format!("column{idx}"), cell_to_data(cell, head, tz)))
                        .collect::<Record>()
                        .into_value(head)
                })
                .collect::<Vec<_>>();

            Ok((name, rows.into_value(head)))
        })
        .collect::<Result<Record, ShellError>>()?;

    Ok(output.into_value(head).into_pipeline_data())
}

fn cell_to_data(cell: &Data, head: Span, tz: chrono::FixedOffset) -> Value {
    match cell {
        Data::Empty => Value::nothing(head),
        Data::Int(val) => Value::int(*val, head),
        Data::Float(val) => Value::float(*val, head),
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
