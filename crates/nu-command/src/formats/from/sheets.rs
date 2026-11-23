use std::io::Cursor;

use calamine::*;
use jiff::{civil, tz::TimeZone};
use nu_protocol::DateTime;

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
        .switch(
            "prefer-integers",
            "Convert whole-number floats (for example, 40.0) to integers, \
                leaving non-whole floats unchanged.",
            Some('i'),
        )
        .category(Category::Formats)
}

pub(super) fn collect_binary(
    input: PipelineData,
    head: Span,
) -> Result<Cursor<Vec<u8>>, ShellError> {
    let buf = match input {
        // Deserialize from a byte buffer
        PipelineData::Value(Value::Binary { val: bytes, .. }, _) => Ok(bytes.into_owned()),
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
    let prefer_integers = call.has_flag(engine_state, stack, "prefer-integers")?;
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

    // Cells hold wall-clock times, read in the system time zone at the offset of their date.
    let tz = TimeZone::system();

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

            let rows = sheet.rows().map(|row| {
                row.iter()
                    .map(|cell| cell_to_data(cell, head, &tz, prefer_integers))
            });

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

fn cell_to_data(cell: &Data, head: Span, tz: &TimeZone, prefer_integers: bool) -> Value {
    match cell {
        Data::Empty => Value::nothing(head),
        Data::Int(val) => Value::int(*val, head),
        // Calamine discards number format information, so numeric cells
        // come through as Data::Float even when the spreadsheet stores
        // them as integers. When --prefer-integers is set, check whether
        // a float is a whole number and convert it to int if so.
        Data::Float(val) if prefer_integers && *val as i64 as f64 == *val => {
            Value::int(*val as i64, head)
        }
        Data::Float(val) => Value::float(*val, head),
        Data::String(val) => Value::string(val, head),
        Data::Bool(val) => Value::bool(*val, head),
        Data::DateTime(d) => excel_datetime_to_value(d, tz, head),
        Data::DateTimeIso(datetime_str) => parse_iso_datetime(datetime_str, tz)
            .map(|d| Value::date(d, head))
            .unwrap_or_else(|| datetime_str.as_str().into_value(head)),
        Data::DurationIso(duration_str) => iso_duration_nanos(duration_str)
            .map(|nanos| Value::duration(nanos, head))
            .unwrap_or(Value::string(duration_str, head)),

        // Not great.
        Data::Error(_) => Value::nothing(head),
    }
}

/// An ISO date-time cell: RFC 3339 with an offset, or a date and time or a date alone, read in
/// `tz`.
fn parse_iso_datetime(datetime_str: &str, tz: &TimeZone) -> Option<DateTime> {
    if let Ok(date) = datetime_str.parse::<DateTime>() {
        return Some(date);
    }
    let civil = datetime_str.parse::<civil::DateTime>().ok().or_else(|| {
        datetime_str
            .parse::<civil::Date>()
            .ok()
            .map(|date| date.to_datetime(civil::Time::midnight()))
    })?;
    DateTime::from_civil_in(civil, tz)
}

/// An ISO duration cell written `PT01H02M03.5S`, with hours, minutes and seconds under a day, in
/// nanoseconds.
fn iso_duration_nanos(duration_str: &str) -> Option<i64> {
    let time = jiff::fmt::strtime::parse("PT%HH%MM%S%.fS", duration_str)
        .ok()?
        .to_time()
        .ok()?;
    let seconds =
        i64::from(time.hour()) * 3600 + i64::from(time.minute()) * 60 + i64::from(time.second());
    Some(seconds * 1_000_000_000 + i64::from(time.subsec_nanosecond()))
}

fn excel_datetime_to_value(excel_datetime: &ExcelDateTime, tz: &TimeZone, span: Span) -> Value {
    if excel_datetime.is_datetime() {
        let (year, month, day, hour, minute, second, milli) = excel_datetime.to_ymd_hms_milli();
        let civil = civil::DateTime::new(
            i16::try_from(year).unwrap_or(i16::MAX),
            month as i8,
            day as i8,
            hour as i8,
            minute as i8,
            second as i8,
            i32::from(milli) * 1_000_000,
        );
        civil
            .ok()
            .and_then(|civil| DateTime::from_civil_in(civil, tz))
            .map_or_else(|| Value::nothing(span), |date| Value::date(date, span))
    } else if excel_datetime.is_duration() {
        // A duration is a number of days, kept to the millisecond.
        let millis = (excel_datetime.as_f64() * 86_400_000.0).round();
        if millis.abs() < (i64::MAX / 1_000_000) as f64 {
            Value::duration(millis as i64 * 1_000_000, span)
        } else {
            Value::nothing(span)
        }
    } else {
        // not great, but better than just returning `null`
        excel_datetime.as_f64().into_value(span)
    }
}
