use nu_color_config::{StyleComputer, TextStyle};
use nu_engine::column::get_columns;
use nu_protocol::{ast::PathMember, Config, ShellError, Span, TableIndexMode, Value};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::{Cell, NuTable, NuText};

use super::{
    clean_charset, create_table_config, get_empty_style, get_header_style, get_index_style,
    get_value_style, StringResult, TableOutput, TableResult, INDEX_COLUMN_NAME,
};

pub struct JustTable;

impl JustTable {
    pub fn table(input: &[Value], row_offset: usize, opts: BuildConfig<'_>) -> StringResult {
        let out = match table(input, row_offset, opts.clone())? {
            Some(out) => out,
            None => return Ok(None),
        };

        let table_config = create_table_config(opts.config, opts.style_computer, &out);
        let table = out.table.draw(table_config, opts.term_width);

        Ok(table)
    }

    pub fn kv_table(cols: &[String], vals: &[Value], opts: BuildConfig<'_>) -> StringResult {
        kv_table(cols, vals, opts)
    }
}

#[derive(Debug, Clone)]
pub struct BuildConfig<'a> {
    pub(crate) ctrlc: Option<Arc<AtomicBool>>,
    pub(crate) config: &'a Config,
    pub(crate) style_computer: &'a StyleComputer<'a>,
    pub(crate) span: Span,
    pub(crate) term_width: usize,
}

impl<'a> BuildConfig<'a> {
    pub fn new(
        ctrlc: Option<Arc<AtomicBool>>,
        config: &'a Config,
        style_computer: &'a StyleComputer<'a>,
        span: Span,
        term_width: usize,
    ) -> Self {
        Self {
            ctrlc,
            config,
            style_computer,
            span,
            term_width,
        }
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn term_width(&self) -> usize {
        self.term_width
    }

    pub fn config(&self) -> &Config {
        self.config
    }

    pub fn style_computer(&self) -> &StyleComputer {
        self.style_computer
    }

    pub fn ctrlc(&self) -> Option<&Arc<AtomicBool>> {
        self.ctrlc.as_ref()
    }
}

fn kv_table(cols: &[String], vals: &[Value], opts: BuildConfig<'_>) -> StringResult {
    let mut data = vec![Vec::with_capacity(2); cols.len()];
    for ((column, value), row) in cols.iter().zip(vals.iter()).zip(data.iter_mut()) {
        if nu_utils::ctrl_c::was_pressed(&opts.ctrlc) {
            return Ok(None);
        }

        let is_string_value = matches!(value, Value::String { .. });
        let mut value = value.into_abbreviated_string(opts.config);
        if is_string_value {
            value = clean_charset(&value);
        }

        let key = Cell::new(column.to_string());
        let value = Cell::new(value);
        row.push(key);
        row.push(value);
    }

    let mut table = NuTable::from(data);
    table.set_index_style(TextStyle::default_field());

    let out = TableOutput::new(table, false, true);
    let table_config = create_table_config(opts.config, opts.style_computer, &out);
    let table = out.table.draw(table_config, opts.term_width);

    Ok(table)
}

fn table(input: &[Value], row_offset: usize, opts: BuildConfig<'_>) -> TableResult {
    if input.is_empty() {
        return Ok(None);
    }

    let mut headers = get_columns(input);
    let with_index = match opts.config.table_index_mode {
        TableIndexMode::Always => true,
        TableIndexMode::Never => false,
        TableIndexMode::Auto => headers.iter().any(|header| header == INDEX_COLUMN_NAME),
    };

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
    opts: BuildConfig<'_>,
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

        if let Value::Error { error } = item {
            return Err(*error.clone());
        }

        if with_index {
            let text = get_table_row_index(item, opts.config, row, row_offset);
            table.insert((row + 1, 0), text);
        }

        let skip_index = usize::from(with_index);
        for (col, header) in headers.iter().enumerate().skip(skip_index) {
            let (text, style) = get_string_value_with_header(item, header, &opts);

            table.insert((row + 1, col), text);
            table.set_cell_style((row + 1, col), style);
        }
    }

    Ok(Some(table))
}

fn to_table_with_no_header(
    input: &[Value],
    with_index: bool,
    row_offset: usize,
    opts: BuildConfig<'_>,
) -> Result<Option<NuTable>, ShellError> {
    let mut table = NuTable::new(input.len(), with_index as usize + 1);
    table.set_index_style(get_index_style(opts.style_computer));

    for (row, item) in input.iter().enumerate() {
        if nu_utils::ctrl_c::was_pressed(&opts.ctrlc) {
            return Ok(None);
        }

        if let Value::Error { error } = item {
            return Err(*error.clone());
        }

        if with_index {
            let text = get_table_row_index(item, opts.config, row, row_offset);
            table.insert((row, 0), text);
        }

        let (text, style) = get_string_value(item, &opts);

        let pos = (row, with_index as usize);
        table.insert(pos, text);
        table.set_cell_style(pos, style);
    }

    Ok(Some(table))
}

fn get_string_value_with_header(item: &Value, header: &str, opts: &BuildConfig) -> NuText {
    match item {
        Value::Record { .. } => {
            let path = PathMember::String {
                val: header.to_owned(),
                span: Span::unknown(),
                optional: false,
            };
            let value = item.clone().follow_cell_path(&[path], false);

            match value {
                Ok(value) => get_string_value(&value, opts),
                Err(_) => get_empty_style(opts.style_computer),
            }
        }
        value => get_string_value(value, opts),
    }
}

fn get_string_value(item: &Value, opts: &BuildConfig) -> NuText {
    let (mut text, style) = get_value_style(item, opts.config, opts.style_computer);
    let is_string_value = matches!(item, Value::String { .. });
    if is_string_value {
        text = clean_charset(&text);
    }

    (text, style)
}

fn get_table_row_index(item: &Value, config: &Config, row: usize, offset: usize) -> String {
    match item {
        Value::Record { .. } => item
            .get_data_by_key(INDEX_COLUMN_NAME)
            .map(|value| value.into_string("", config))
            .unwrap_or_else(|| (row + offset).to_string()),
        _ => (row + offset).to_string(),
    }
}
