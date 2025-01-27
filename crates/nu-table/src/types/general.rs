use nu_color_config::TextStyle;
use nu_engine::column::get_columns;
use nu_protocol::{Config, Record, ShellError, Value};

use crate::{
    clean_charset, colorize_space,
    common::{
        check_value, configure_table, get_empty_style, get_header_style, get_index_style,
        get_value_style, nu_value_to_string_colored, NuText, INDEX_COLUMN_NAME,
    },
    types::has_index,
    NuRecordsValue, NuTable, StringResult, TableOpts, TableOutput, TableResult,
};

pub struct JustTable;

impl JustTable {
    pub fn table(input: &[Value], opts: TableOpts<'_>) -> StringResult {
        list_table(input, opts)
    }

    pub fn kv_table(record: &Record, opts: TableOpts<'_>) -> StringResult {
        kv_table(record, opts)
    }
}

fn list_table(input: &[Value], opts: TableOpts<'_>) -> Result<Option<String>, ShellError> {
    let mut out = match create_table(input, &opts)? {
        Some(out) => out,
        None => return Ok(None),
    };

    out.table.set_indent(opts.config.table.padding);

    colorize_space(out.table.get_records_mut(), &opts.style_computer);

    configure_table(&mut out, opts.config, &opts.style_computer, opts.mode);
    let table = out.table.draw(opts.width);

    Ok(table)
}

fn kv_table(record: &Record, opts: TableOpts<'_>) -> StringResult {
    let mut data = vec![Vec::with_capacity(2); record.len()];

    for ((column, value), row) in record.iter().zip(data.iter_mut()) {
        opts.signals.check(opts.span)?;

        let key = NuRecordsValue::new(column.to_string());

        let value = nu_value_to_string_colored(value, opts.config, &opts.style_computer);
        let value = NuRecordsValue::new(value);

        row.push(key);
        row.push(value);
    }

    let mut table = NuTable::from(data);
    table.set_index_style(TextStyle::default_field());
    table.set_indent(opts.config.table.padding);

    let mut out = TableOutput::from_table(table, false, true);
    configure_table(&mut out, opts.config, &opts.style_computer, opts.mode);
    let table = out.table.draw(opts.width);

    Ok(table)
}

fn create_table(input: &[Value], opts: &TableOpts<'_>) -> TableResult {
    if input.is_empty() {
        return Ok(None);
    }

    let headers = get_columns(input);
    let with_index = has_index(opts, &headers);
    let with_header = !headers.is_empty();
    let row_offset = opts.index_offset;

    let table = match (with_header, with_index) {
        (true, true) => create_table_with_header_and_index(input, headers, row_offset, opts)?,
        (true, false) => create_table_with_header(input, headers, opts)?,
        (false, true) => create_table_with_no_header_and_index(input, row_offset, opts)?,
        (false, false) => create_table_with_no_header(input, opts)?,
    };

    let table = table.map(|table| TableOutput::from_table(table, with_header, with_index));

    Ok(table)
}

fn create_table_with_header(
    input: &[Value],
    headers: Vec<String>,
    opts: &TableOpts<'_>,
) -> Result<Option<NuTable>, ShellError> {
    let headers = collect_headers(headers, false);

    let count_rows = input.len() + 1;
    let count_columns = headers.len();
    let mut table = NuTable::new(count_rows, count_columns);

    table.set_header_style(get_header_style(&opts.style_computer));
    table.set_index_style(get_index_style(&opts.style_computer));

    table.set_row(0, headers.clone());

    for (row, item) in input.iter().enumerate() {
        opts.signals.check(opts.span)?;
        check_value(item)?;

        for (col, header) in headers.iter().enumerate() {
            let (text, style) = get_string_value_with_header(item, header.as_ref(), opts);

            let pos = (row + 1, col);
            table.insert(pos, text);
            table.insert_style(pos, style);
        }
    }

    Ok(Some(table))
}

fn create_table_with_header_and_index(
    input: &[Value],
    headers: Vec<String>,
    row_offset: usize,
    opts: &TableOpts<'_>,
) -> Result<Option<NuTable>, ShellError> {
    let headers = collect_headers(headers, true);

    let count_rows = input.len() + 1;
    let count_columns = headers.len();
    let mut table = NuTable::new(count_rows, count_columns);

    table.set_header_style(get_header_style(&opts.style_computer));
    table.set_index_style(get_index_style(&opts.style_computer));

    table.set_row(0, headers.clone());

    for (row, item) in input.iter().enumerate() {
        opts.signals.check(opts.span)?;
        check_value(item)?;

        let text = get_table_row_index(item, opts.config, row, row_offset);
        table.insert((row + 1, 0), text);

        for (col, header) in headers.iter().enumerate().skip(1) {
            let (text, style) = get_string_value_with_header(item, header.as_ref(), opts);

            let pos = (row + 1, col);
            table.insert(pos, text);
            table.insert_style(pos, style);
        }
    }

    Ok(Some(table))
}

fn create_table_with_no_header(
    input: &[Value],
    opts: &TableOpts<'_>,
) -> Result<Option<NuTable>, ShellError> {
    let mut table = NuTable::new(input.len(), 1);
    table.set_index_style(get_index_style(&opts.style_computer));

    for (row, item) in input.iter().enumerate() {
        opts.signals.check(opts.span)?;
        check_value(item)?;

        let (text, style) = get_string_value(item, opts);

        let pos = (row, 0);
        table.insert(pos, text);
        table.insert_style(pos, style);
    }

    Ok(Some(table))
}

fn create_table_with_no_header_and_index(
    input: &[Value],
    row_offset: usize,
    opts: &TableOpts<'_>,
) -> Result<Option<NuTable>, ShellError> {
    let mut table = NuTable::new(input.len(), 1 + 1);
    table.set_index_style(get_index_style(&opts.style_computer));

    for (row, item) in input.iter().enumerate() {
        opts.signals.check(opts.span)?;
        check_value(item)?;

        let text = get_table_row_index(item, opts.config, row, row_offset);
        table.insert((row, 0), text);

        let (text, style) = get_string_value(item, opts);

        let pos = (row, 1);
        table.insert(pos, text);
        table.insert_style(pos, style);
    }

    Ok(Some(table))
}

fn get_string_value_with_header(item: &Value, header: &str, opts: &TableOpts) -> NuText {
    match item {
        Value::Record { val, .. } => match val.get(header) {
            Some(value) => get_string_value(value, opts),
            None => get_empty_style(&opts.style_computer),
        },
        value => get_string_value(value, opts),
    }
}

fn get_string_value(item: &Value, opts: &TableOpts) -> NuText {
    let (mut text, style) = get_value_style(item, opts.config, &opts.style_computer);

    let is_string = matches!(item, Value::String { .. });
    if is_string {
        text = clean_charset(&text);
    }

    (text, style)
}

fn get_table_row_index(item: &Value, config: &Config, row: usize, offset: usize) -> String {
    match item {
        Value::Record { val, .. } => val
            .get(INDEX_COLUMN_NAME)
            .map(|value| value.to_expanded_string("", config))
            .unwrap_or_else(|| (row + offset).to_string()),
        _ => (row + offset).to_string(),
    }
}

fn collect_headers(headers: Vec<String>, index: bool) -> Vec<NuRecordsValue> {
    // The header with the INDEX is removed from the table headers since
    // it is added to the natural table index
    let length = if index {
        headers.len() + 1
    } else {
        headers.len()
    };

    let mut v = Vec::with_capacity(length);

    if index {
        v.insert(0, NuRecordsValue::new("#".into()));
    }

    for text in headers {
        if text == INDEX_COLUMN_NAME {
            continue;
        }

        v.push(NuRecordsValue::new(text));
    }

    v
}
