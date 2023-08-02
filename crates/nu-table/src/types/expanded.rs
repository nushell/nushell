use nu_color_config::{Alignment, StyleComputer, TextStyle};
use nu_engine::column::get_columns;
use nu_protocol::{ast::PathMember, Config, Span, TableIndexMode, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::{cmp::max, sync::atomic::AtomicBool};

use crate::{string_width, Cell, NuTable};

use super::{clean_charset, value_to_clean_styled_string};
use super::{
    create_table_config, error_sign, general::BuildConfig, get_header_style, get_index_style,
    load_theme_from_config, set_data_styles, value_to_styled_string, wrap_text, NuText,
    StringResult, TableOutput, TableResult, INDEX_COLUMN_NAME,
};

#[derive(Debug, Clone)]
pub struct ExpandedTable {
    expand_limit: Option<usize>,
    flatten: bool,
    flatten_sep: String,
}

impl ExpandedTable {
    pub fn new(expand_limit: Option<usize>, flatten: bool, flatten_sep: String) -> Self {
        Self {
            expand_limit,
            flatten,
            flatten_sep,
        }
    }

    pub fn build_value(&self, item: &Value, opts: BuildConfig<'_>) -> NuText {
        let opts = Options {
            ctrlc: opts.ctrlc,
            config: opts.config,
            style_computer: opts.style_computer,
            available_width: opts.term_width,
            span: opts.span,
            format: self.clone(),
        };
        expanded_table_entry2(item, opts)
    }

    pub fn build_map(
        &self,
        cols: &[String],
        vals: &[Value],
        opts: BuildConfig<'_>,
    ) -> StringResult {
        let opts = Options {
            ctrlc: opts.ctrlc,
            config: opts.config,
            style_computer: opts.style_computer,
            available_width: opts.term_width,
            span: opts.span,
            format: self.clone(),
        };
        expanded_table_kv(cols, vals, opts)
    }

    pub fn build_list(
        &self,
        vals: &[Value],
        opts: BuildConfig<'_>,
        row_offset: usize,
    ) -> StringResult {
        let opts1 = Options {
            ctrlc: opts.ctrlc,
            config: opts.config,
            style_computer: opts.style_computer,
            available_width: opts.term_width,
            span: opts.span,
            format: self.clone(),
        };
        let out = match expanded_table_list(vals, row_offset, opts1)? {
            Some(out) => out,
            None => return Ok(None),
        };

        maybe_expand_table(out, opts.term_width, opts.config, opts.style_computer)
    }
}

#[derive(Debug, Clone)]
struct Options<'a> {
    ctrlc: Option<Arc<AtomicBool>>,
    config: &'a Config,
    style_computer: &'a StyleComputer,
    available_width: usize,
    format: ExpandedTable,
    span: Span,
}

fn expanded_table_list(input: &[Value], row_offset: usize, opts: Options) -> TableResult {
    const PADDING_SPACE: usize = 2;
    const SPLIT_LINE_SPACE: usize = 1;
    const ADDITIONAL_CELL_SPACE: usize = PADDING_SPACE + SPLIT_LINE_SPACE;
    const MIN_CELL_CONTENT_WIDTH: usize = 1;
    const TRUNCATE_CONTENT_WIDTH: usize = 3;
    const TRUNCATE_CELL_WIDTH: usize = TRUNCATE_CONTENT_WIDTH + PADDING_SPACE;

    if input.is_empty() {
        return Ok(None);
    }

    // 2 - split lines
    let mut available_width = opts
        .available_width
        .saturating_sub(SPLIT_LINE_SPACE + SPLIT_LINE_SPACE);
    if available_width < MIN_CELL_CONTENT_WIDTH {
        return Ok(None);
    }

    let headers = get_columns(input);

    let with_index = match opts.config.table_index_mode {
        TableIndexMode::Always => true,
        TableIndexMode::Never => false,
        TableIndexMode::Auto => headers.iter().any(|header| header == INDEX_COLUMN_NAME),
    };

    // The header with the INDEX is removed from the table headers since
    // it is added to the natural table index
    let headers: Vec<_> = headers
        .into_iter()
        .filter(|header| header != INDEX_COLUMN_NAME)
        .collect();

    let with_header = !headers.is_empty();

    let mut data = vec![vec![]; input.len() + with_header as usize];
    let mut data_styles = HashMap::new();

    if with_index {
        if with_header {
            data[0].push(Cell::exact(String::from("#"), 1, vec![]));
        }

        for (row, item) in input.iter().enumerate() {
            if nu_utils::ctrl_c::was_pressed(&opts.ctrlc) {
                return Ok(None);
            }

            if let Value::Error { error } = item {
                return Err(*error.clone());
            }

            let index = row + row_offset;
            let text = matches!(item, Value::Record { .. })
                .then(|| lookup_index_value(item, opts.config).unwrap_or_else(|| index.to_string()))
                .unwrap_or_else(|| index.to_string());

            let value = Cell::new(text);

            let row = row + with_header as usize;
            data[row].push(value);
        }

        let column_width = string_width(data[data.len() - 1][0].as_ref());

        if column_width + ADDITIONAL_CELL_SPACE > available_width {
            available_width = 0;
        } else {
            available_width -= column_width + ADDITIONAL_CELL_SPACE;
        }
    }

    if !with_header {
        if available_width > ADDITIONAL_CELL_SPACE {
            available_width -= PADDING_SPACE;
        } else {
            // it means we have no space left for actual content;
            // which means there's no point in index itself if it was even used.
            // so we do not print it.
            return Ok(None);
        }

        for (row, item) in input.iter().enumerate() {
            if nu_utils::ctrl_c::was_pressed(&opts.ctrlc) {
                return Ok(None);
            }

            if let Value::Error { error } = item {
                return Err(*error.clone());
            }

            let mut oopts = opts.clone();
            oopts.available_width = available_width;
            let (mut text, style) = expanded_table_entry2(item, oopts.clone());

            let value_width = string_width(&text);
            if value_width > available_width {
                // it must only happen when a string is produced, so we can safely wrap it.
                // (it might be string table representation as well) (I guess I mean default { table ...} { list ...})
                //
                // todo: Maybe convert_to_table2_entry could do for strings to not mess caller code?

                text = wrap_text(&text, available_width, opts.config);
            }

            let value = Cell::new(text);
            data[row].push(value);
            data_styles.insert((row, with_index as usize), style);
        }

        let mut table = NuTable::from(data);
        table.set_index_style(get_index_style(opts.style_computer));
        set_data_styles(&mut table, data_styles);

        return Ok(Some(TableOutput::new(table, false, with_index)));
    }

    if !headers.is_empty() {
        let mut pad_space = PADDING_SPACE;
        if headers.len() > 1 {
            pad_space += SPLIT_LINE_SPACE;
        }

        if available_width < pad_space {
            // there's no space for actual data so we don't return index if it's present.
            // (also see the comment after the loop)

            return Ok(None);
        }
    }

    let count_columns = headers.len();
    let mut widths = Vec::new();
    let mut truncate = false;
    let mut rendered_column = 0;
    for (col, header) in headers.into_iter().enumerate() {
        let is_last_column = col + 1 == count_columns;
        let mut pad_space = PADDING_SPACE;
        if !is_last_column {
            pad_space += SPLIT_LINE_SPACE;
        }

        let mut available = available_width - pad_space;
        let mut column_width = string_width(&header);

        if !is_last_column {
            // we need to make sure that we have a space for a next column if we use available width
            // so we might need to decrease a bit it.

            // we consider a header width be a minimum width
            let pad_space = PADDING_SPACE + TRUNCATE_CONTENT_WIDTH;

            if available > pad_space {
                // In we have no space for a next column,
                // We consider showing something better then nothing,
                // So we try to decrease the width to show at least a truncution column

                available -= pad_space;
            } else {
                truncate = true;
                break;
            }

            if available < column_width {
                truncate = true;
                break;
            }
        }

        for (row, item) in input.iter().enumerate() {
            if nu_utils::ctrl_c::was_pressed(&opts.ctrlc) {
                return Ok(None);
            }

            if let Value::Error { error } = item {
                return Err(*error.clone());
            }

            let mut oopts = opts.clone();
            oopts.available_width = available;
            let (mut text, style) = expanded_table_entry(item, header.as_str(), oopts);

            let mut value_width = string_width(&text);
            if value_width > available {
                // it must only happen when a string is produced, so we can safely wrap it.
                // (it might be string table representation as well)

                text = wrap_text(&text, available, opts.config);
                value_width = available;
            }

            column_width = max(column_width, value_width);

            let value = Cell::new(text);
            data[row + 1].push(value);
            data_styles.insert((row + 1, col + with_index as usize), style);
        }

        let head_cell = Cell::new(header);
        data[0].push(head_cell);

        if column_width > available {
            // remove the column we just inserted
            for row in &mut data {
                row.pop();
            }

            truncate = true;
            break;
        }

        widths.push(column_width);

        available_width -= pad_space + column_width;
        rendered_column += 1;
    }

    if truncate && rendered_column == 0 {
        // it means that no actual data was rendered, there might be only index present,
        // so there's no point in rendering the table.
        //
        // It's actually quite important in case it's called recursively,
        // cause we will back up to the basic table view as a string e.g. '[table 123 columns]'.
        //
        // But potentially if its reached as a 1st called function we might would love to see the index.

        return Ok(None);
    }

    if truncate {
        if available_width < TRUNCATE_CELL_WIDTH {
            // back up by removing last column.
            // it's LIKELY that removing only 1 column will leave us enough space for a shift column.

            while let Some(width) = widths.pop() {
                for row in &mut data {
                    row.pop();
                }

                available_width += width + PADDING_SPACE;
                if !widths.is_empty() {
                    available_width += SPLIT_LINE_SPACE;
                }

                if available_width > TRUNCATE_CELL_WIDTH {
                    break;
                }
            }
        }

        // this must be a RARE case or even NEVER happen,
        // but we do check it just in case.
        if available_width < TRUNCATE_CELL_WIDTH {
            return Ok(None);
        }

        let is_last_column = widths.len() == count_columns;
        if !is_last_column {
            let shift = Cell::exact(String::from("..."), 3, vec![]);
            for row in &mut data {
                row.push(shift.clone());
            }

            widths.push(3);
        }
    }

    let mut table = NuTable::from(data);
    table.set_index_style(get_index_style(opts.style_computer));
    table.set_header_style(get_header_style(opts.style_computer));
    set_data_styles(&mut table, data_styles);

    Ok(Some(TableOutput::new(table, true, with_index)))
}

fn expanded_table_kv(cols: &[String], vals: &[Value], opts: Options<'_>) -> StringResult {
    let theme = load_theme_from_config(opts.config);
    let key_width = cols.iter().map(|col| string_width(col)).max().unwrap_or(0);
    let count_borders =
        theme.has_inner() as usize + theme.has_right() as usize + theme.has_left() as usize;
    let padding = 2;
    if key_width + count_borders + padding + padding > opts.available_width {
        return Ok(None);
    }

    let value_width = opts.available_width - key_width - count_borders - padding - padding;

    let mut data = Vec::with_capacity(cols.len());
    for (key, value) in cols.iter().zip(vals) {
        if nu_utils::ctrl_c::was_pressed(&opts.ctrlc) {
            return Ok(None);
        }

        let is_limited = matches!(opts.format.expand_limit, Some(0));
        let mut is_expanded = false;
        let value = if is_limited {
            let (text, _) = value_to_styled_string(value, opts.config, opts.style_computer);
            clean_charset(&text)
        } else {
            match value {
                Value::List { vals, span } => {
                    let mut oopts = dive_options(&opts, *span);
                    oopts.available_width = value_width;
                    let table = expanded_table_list(vals, 0, oopts)?;

                    match table {
                        Some(out) => {
                            is_expanded = true;

                            let table_config =
                                create_table_config(opts.config, opts.style_computer, &out);
                            let value = out.table.draw(table_config, value_width);
                            match value {
                                Some(result) => result,
                                None => return Ok(None),
                            }
                        }
                        None => {
                            // it means that the list is empty
                            let text =
                                value_to_styled_string(value, opts.config, opts.style_computer).0;
                            wrap_text(&text, value_width, opts.config)
                        }
                    }
                }
                Value::Record { cols, vals, span } => {
                    if cols.is_empty() {
                        // Like list case return styled string instead of empty value
                        let text =
                            value_to_styled_string(value, opts.config, opts.style_computer).0;
                        wrap_text(&text, value_width, opts.config)
                    } else {
                        let mut oopts = dive_options(&opts, *span);
                        oopts.available_width = value_width;
                        let result = expanded_table_kv(cols, vals, oopts)?;
                        match result {
                            Some(result) => {
                                is_expanded = true;
                                result
                            }
                            None => {
                                let failed_value =
                                    value_to_styled_string(value, opts.config, opts.style_computer);
                                wrap_text(&failed_value.0, value_width, opts.config)
                            }
                        }
                    }
                }
                val => {
                    let text =
                        value_to_clean_styled_string(val, opts.config, opts.style_computer).0;
                    wrap_text(&text, value_width, opts.config)
                }
            }
        };

        // we want to have a key being aligned to 2nd line,
        // we could use Padding for it but,
        // the easiest way to do so is just push a new_line char before
        let mut key = key.to_owned();
        if !key.is_empty() && is_expanded && theme.has_top_line() {
            key.insert(0, '\n');
        }

        let key = Cell::new(key);
        let val = Cell::new(value);

        let row = vec![key, val];
        data.push(row);
    }

    let mut table = NuTable::from(data);
    let keys_style = get_header_style(opts.style_computer).alignment(Alignment::Left);
    table.set_index_style(keys_style);

    let out = TableOutput::new(table, false, true);

    maybe_expand_table(out, opts.available_width, opts.config, opts.style_computer)
}

fn expanded_table_entry(item: &Value, header: &str, opts: Options<'_>) -> NuText {
    match item {
        Value::Record { .. } => {
            let val = header.to_owned();
            let path = PathMember::String {
                val,
                span: opts.span,
                optional: false,
            };
            let val = item.clone().follow_cell_path(&[path], false);

            match val {
                Ok(val) => expanded_table_entry2(&val, opts),
                Err(_) => error_sign(opts.style_computer),
            }
        }
        _ => expanded_table_entry2(item, opts),
    }
}

fn expanded_table_entry2(item: &Value, opts: Options<'_>) -> NuText {
    let is_limit_reached = matches!(opts.format.expand_limit, Some(0));
    if is_limit_reached {
        return value_to_clean_styled_string(item, opts.config, opts.style_computer);
    }

    match &item {
        Value::Record { cols, vals, span } => {
            if cols.is_empty() && vals.is_empty() {
                return value_to_styled_string(item, opts.config, opts.style_computer);
            }

            // we verify what is the structure of a Record cause it might represent
            let oopts = dive_options(&opts, *span);
            let table = expanded_table_kv(cols, vals, oopts);

            match table {
                Ok(Some(table)) => (table, TextStyle::default()),
                _ => value_to_styled_string(item, opts.config, opts.style_computer),
            }
        }
        Value::List { vals, span } => {
            if opts.format.flatten && is_simple_list(vals) {
                return value_list_to_string(
                    vals,
                    opts.config,
                    opts.style_computer,
                    &opts.format.flatten_sep,
                );
            }

            let oopts = dive_options(&opts, *span);
            let table = expanded_table_list(vals, 0, oopts);

            let out = match table {
                Ok(Some(out)) => out,
                _ => return value_to_styled_string(item, opts.config, opts.style_computer),
            };

            let table_config = create_table_config(opts.config, opts.style_computer, &out);

            let table = out.table.draw(table_config, usize::MAX);
            match table {
                Some(table) => (table, TextStyle::default()),
                None => value_to_styled_string(item, opts.config, opts.style_computer),
            }
        }
        _ => value_to_clean_styled_string(item, opts.config, opts.style_computer),
    }
}

fn is_simple_list(vals: &[Value]) -> bool {
    vals.iter()
        .all(|v| !matches!(v, Value::Record { .. } | Value::List { .. }))
}

fn value_list_to_string(
    vals: &[Value],
    config: &Config,
    style_computer: &StyleComputer,
    flatten_sep: &str,
) -> NuText {
    let mut buf = String::new();
    for (i, value) in vals.iter().enumerate() {
        if i > 0 {
            buf.push_str(flatten_sep);
        }

        let (text, _) = value_to_clean_styled_string(value, config, style_computer);
        buf.push_str(&text);
    }

    (buf, TextStyle::default())
}

fn dive_options<'b>(opts: &Options<'b>, span: Span) -> Options<'b> {
    let mut opts = opts.clone();
    opts.span = span;
    if let Some(deep) = opts.format.expand_limit.as_mut() {
        *deep -= 1
    }

    opts
}

fn lookup_index_value(item: &Value, config: &Config) -> Option<String> {
    item.get_data_by_key(INDEX_COLUMN_NAME)
        .map(|value| value.into_string("", config))
}

fn maybe_expand_table(
    out: TableOutput,
    term_width: usize,
    config: &Config,
    style_computer: &StyleComputer,
) -> StringResult {
    let mut table_config = create_table_config(config, style_computer, &out);
    let total_width = out.table.total_width(&table_config);
    if total_width < term_width {
        const EXPAND_THRESHOLD: f32 = 0.80;
        let used_percent = total_width as f32 / term_width as f32;
        let need_expansion = total_width < term_width && used_percent > EXPAND_THRESHOLD;
        if need_expansion {
            table_config = table_config.expand(true);
        }
    }

    let output = out.table.draw(table_config, term_width);
    Ok(output)
}
