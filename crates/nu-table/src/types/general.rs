use super::has_index;
use crate::{
    clean_charset, colorize_space,
    common::{
        create_nu_table_config, get_empty_style, get_header_style, get_index_style,
        get_value_style, nu_value_to_string_colored, NuText, INDEX_COLUMN_NAME,
    },
    NuTable, NuTableCell, StringResult, TableOpts, TableOutput, TableResult,
};
use nu_color_config::TextStyle;
use nu_engine::column::get_columns;
use nu_protocol::{Config, Record, ShellError, Value};

pub struct JustTable;

impl JustTable {
    pub fn table(input: &[Value], opts: TableOpts<'_>) -> StringResult {
        create_table(input, opts)
    }

    pub fn kv_table(record: &Record, opts: TableOpts<'_>) -> StringResult {
        kv_table(record, opts)
    }
}

fn create_table(input: &[Value], opts: TableOpts<'_>) -> Result<Option<String>, ShellError> {
    match table(input, &opts)? {
        Some(mut out) => {
            let left = opts.config.table_indent.left;
            let right = opts.config.table_indent.right;
            out.table.set_indent(left, right);

            colorize_space(out.table.get_records_mut(), opts.style_computer);

            let table_config =
                create_nu_table_config(opts.config, opts.style_computer, &out, false, opts.mode);
            Ok(out.table.draw(table_config, opts.width))
        }
        None => Ok(None),
    }
}

fn kv_table(record: &Record, opts: TableOpts<'_>) -> StringResult {
    let mut data = vec![Vec::with_capacity(2); record.len()];
    for ((column, value), row) in record.iter().zip(data.iter_mut()) {
        if nu_utils::ctrl_c::was_pressed(&opts.ctrlc) {
            return Ok(None);
        }

        let value = nu_value_to_string_colored(value, opts.config, opts.style_computer);

        let key = NuTableCell::new(column.to_string());
        let value = NuTableCell::new(value);

        row.push(key);
        row.push(value);
    }

    let mut table = NuTable::from(data);
    table.set_index_style(TextStyle::default_field());

    let mut out = TableOutput::new(table, false, true);

    let left = opts.config.table_indent.left;
    let right = opts.config.table_indent.right;
    out.table.set_indent(left, right);

    let table_config =
        create_nu_table_config(opts.config, opts.style_computer, &out, false, opts.mode);
    let table = out.table.draw(table_config, opts.width);

    Ok(table)
}

fn table(input: &[Value], opts: &TableOpts<'_>) -> TableResult {
    if input.is_empty() {
        return Ok(None);
    }

    let mut headers = get_columns(input);
    let with_index = has_index(opts, &headers);
    let row_offset = opts.index_offset;

    let with_header = !headers.is_empty();
    if !with_header {
        let table = to_table_with_no_header(input, with_index, row_offset, opts)?;
        let table = table.map(|table| TableOutput::new(table, false, with_index));
        return Ok(table);
    }

    if with_header && with_index {
        headers.insert(0, "#".into());
    }

    // The header with the INDEX is removed from the table headers since
    // it is added to the natural table index
    let headers: Vec<_> = headers
        .into_iter()
        .filter(|header| header != INDEX_COLUMN_NAME)
        .collect();

    let table = to_table_with_header(input, headers, with_index, row_offset, opts)?;
    let table = table.map(|table| TableOutput::new(table, true, with_index));

    Ok(table)
}

fn to_table_with_header(
    input: &[Value],
    headers: Vec<String>,
    with_index: bool,
    row_offset: usize,
    opts: &TableOpts<'_>,
) -> Result<Option<NuTable>, ShellError> {
    let count_rows = input.len() + 1;
    let count_columns = headers.len();
    let mut table = NuTable::new(count_rows, count_columns);
    table.set_header_style(get_header_style(opts.style_computer));
    table.set_index_style(get_index_style(opts.style_computer));

    for (i, text) in headers.iter().enumerate() {
        table.insert((0, i), text.to_owned());
    }

    for (row, item) in input.iter().enumerate() {
        if nu_utils::ctrl_c::was_pressed(&opts.ctrlc) {
            return Ok(None);
        }

        if let Value::Error { error, .. } = item {
            return Err(*error.clone());
        }

        if with_index {
            let text = get_table_row_index(item, opts.config, row, row_offset);
            table.insert((row + 1, 0), text);
        }

        let skip_index = usize::from(with_index);
        for (col, header) in headers.iter().enumerate().skip(skip_index) {
            let (text, style) = get_string_value_with_header(item, header, opts);

            table.insert((row + 1, col), text);
            table.insert_style((row + 1, col), style);
        }
    }

    Ok(Some(table))
}

fn to_table_with_no_header(
    input: &[Value],
    with_index: bool,
    row_offset: usize,
    opts: &TableOpts<'_>,
) -> Result<Option<NuTable>, ShellError> {
    let mut table = NuTable::new(input.len(), with_index as usize + 1);
    table.set_index_style(get_index_style(opts.style_computer));

    for (row, item) in input.iter().enumerate() {
        if nu_utils::ctrl_c::was_pressed(&opts.ctrlc) {
            return Ok(None);
        }

        if let Value::Error { error, .. } = item {
            return Err(*error.clone());
        }

        if with_index {
            let text = get_table_row_index(item, opts.config, row, row_offset);
            table.insert((row, 0), text);
        }

        let (text, style) = get_string_value(item, opts);

        let pos = (row, with_index as usize);
        table.insert(pos, text);
        table.insert_style(pos, style);
    }

    Ok(Some(table))
}

fn get_string_value_with_header(item: &Value, header: &str, opts: &TableOpts) -> NuText {
    match item {
        Value::Record { val, .. } => match val.get(header) {
            Some(value) => get_string_value(value, opts),
            None => get_empty_style(opts.style_computer),
        },
        value => get_string_value(value, opts),
    }
}

fn get_string_value(item: &Value, opts: &TableOpts) -> NuText {
    let (mut text, style) = get_value_style(item, opts.config, opts.style_computer);
    let is_string_value = matches!(item, Value::String { .. });
    if is_string_value {
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
