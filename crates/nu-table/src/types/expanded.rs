use std::cmp::max;

use nu_color_config::{Alignment, StyleComputer, TextStyle};
use nu_engine::column::get_columns;
use nu_protocol::{Config, Record, ShellError, Span, Value};
use tabled::grid::records::vec_records::Cell;

use crate::{
    NuTable, TableOpts, TableOutput,
    common::{
        INDEX_COLUMN_NAME, NuText, StringResult, TableResult, check_value, configure_table,
        error_sign, get_header_style, get_index_style, load_theme, nu_value_to_string,
        nu_value_to_string_clean, nu_value_to_string_colored, wrap_text,
    },
    string_width,
    types::has_index,
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

    pub fn build_value(self, item: &Value, opts: TableOpts<'_>) -> NuText {
        let cfg = Cfg { opts, format: self };
        let cell = expand_entry(item, cfg);
        (cell.text, cell.style)
    }

    pub fn build_map(self, record: &Record, opts: TableOpts<'_>) -> StringResult {
        let cfg = Cfg { opts, format: self };
        expanded_table_kv(record, cfg).map(|cell| cell.map(|cell| cell.text))
    }

    pub fn build_list(self, vals: &[Value], opts: TableOpts<'_>) -> StringResult {
        let cfg = Cfg { opts, format: self };
        let output = expand_list(vals, cfg.clone())?;
        let mut output = match output {
            Some(out) => out,
            None => return Ok(None),
        };

        configure_table(
            &mut output,
            cfg.opts.config,
            &cfg.opts.style_computer,
            cfg.opts.mode,
        );

        maybe_expand_table(output, cfg.opts.width)
    }
}

#[derive(Debug, Clone)]
struct Cfg<'a> {
    opts: TableOpts<'a>,
    format: ExpandedTable,
}

#[derive(Debug, Clone)]
struct CellOutput {
    text: String,
    style: TextStyle,
    size: usize,
    is_expanded: bool,
}

impl CellOutput {
    fn new(text: String, style: TextStyle, size: usize, is_expanded: bool) -> Self {
        Self {
            text,
            style,
            size,
            is_expanded,
        }
    }

    fn clean(text: String, size: usize, is_expanded: bool) -> Self {
        Self::new(text, Default::default(), size, is_expanded)
    }

    fn text(text: String) -> Self {
        Self::styled((text, Default::default()))
    }

    fn styled(text: NuText) -> Self {
        Self::new(text.0, text.1, 1, false)
    }
}

type CellResult = Result<Option<CellOutput>, ShellError>;

fn expand_list(input: &[Value], cfg: Cfg<'_>) -> TableResult {
    const SPLIT_LINE_SPACE: usize = 1;
    const MIN_CELL_WIDTH: usize = 3;
    const TRUNCATE_CONTENT_WIDTH: usize = 3;

    if input.is_empty() {
        return Ok(None);
    }

    let pad_width = cfg.opts.config.table.padding.left + cfg.opts.config.table.padding.right;
    let extra_width = pad_width + SPLIT_LINE_SPACE;
    let truncate_column_width = TRUNCATE_CONTENT_WIDTH + pad_width;

    // 2 - split lines
    let mut available_width = cfg
        .opts
        .width
        .saturating_sub(SPLIT_LINE_SPACE + SPLIT_LINE_SPACE);
    if available_width < MIN_CELL_WIDTH {
        return Ok(None);
    }

    let headers = get_columns(input);
    let with_index = has_index(&cfg.opts, &headers);

    // The header with the INDEX is removed from the table headers since
    // it is added to the natural table index
    let headers: Vec<_> = headers
        .into_iter()
        .filter(|header| header != INDEX_COLUMN_NAME)
        .collect();
    let with_header = !headers.is_empty();
    let row_offset = cfg.opts.index_offset;

    let mut total_rows = 0usize;

    if !with_index && !with_header {
        if available_width <= extra_width {
            // it means we have no space left for actual content;
            // which means there's no point in index itself if it was even used.
            // so we do not print it.
            return Ok(None);
        }

        available_width -= pad_width;

        let mut table = NuTable::new(input.len(), 1);
        table.set_index_style(get_index_style(&cfg.opts.style_computer));
        table.set_header_style(get_header_style(&cfg.opts.style_computer));
        table.set_indent(cfg.opts.config.table.padding);

        for (row, item) in input.iter().enumerate() {
            cfg.opts.signals.check(cfg.opts.span)?;
            check_value(item)?;

            let inner_cfg = cfg_expand_reset_table(cfg.clone(), available_width);
            let cell = expand_entry(item, inner_cfg);

            table.insert((row, 0), cell.text);
            table.insert_style((row, 0), cell.style);

            total_rows = total_rows.saturating_add(cell.size);
        }

        return Ok(Some(TableOutput::new(table, false, false, total_rows)));
    }

    if !with_header && with_index {
        let mut table = NuTable::new(input.len(), 2);
        table.set_index_style(get_index_style(&cfg.opts.style_computer));
        table.set_header_style(get_header_style(&cfg.opts.style_computer));
        table.set_indent(cfg.opts.config.table.padding);

        let mut index_column_width = 0;

        for (row, item) in input.iter().enumerate() {
            cfg.opts.signals.check(cfg.opts.span)?;
            check_value(item)?;

            let index = row + row_offset;
            let index_value = item
                .as_record()
                .ok()
                .and_then(|val| val.get(INDEX_COLUMN_NAME))
                .map(|value| value.to_expanded_string("", cfg.opts.config))
                .unwrap_or_else(|| index.to_string());
            let index_value = NuTable::create(index_value);
            let index_width = index_value.width();
            if available_width <= index_width + extra_width + pad_width {
                // NOTE: we don't wanna wrap index; so we return
                return Ok(None);
            }

            table.insert_value((row, 0), index_value);

            index_column_width = max(index_column_width, index_width);
        }

        available_width -= index_column_width + extra_width + pad_width;

        for (row, item) in input.iter().enumerate() {
            cfg.opts.signals.check(cfg.opts.span)?;
            check_value(item)?;

            let inner_cfg = cfg_expand_reset_table(cfg.clone(), available_width);
            let cell = expand_entry(item, inner_cfg);

            table.insert((row, 1), cell.text);
            table.insert_style((row, 1), cell.style);

            total_rows = total_rows.saturating_add(cell.size);
        }

        return Ok(Some(TableOutput::new(table, false, true, total_rows)));
    }

    // NOTE: redefine to not break above logic (fixme)
    let mut available_width = cfg.opts.width - SPLIT_LINE_SPACE;

    let mut table = NuTable::new(input.len() + 1, headers.len() + with_index as usize);
    table.set_index_style(get_index_style(&cfg.opts.style_computer));
    table.set_header_style(get_header_style(&cfg.opts.style_computer));
    table.set_indent(cfg.opts.config.table.padding);

    let mut widths = Vec::new();

    if with_index {
        table.insert((0, 0), String::from("#"));

        let mut index_column_width = 1;

        for (row, item) in input.iter().enumerate() {
            cfg.opts.signals.check(cfg.opts.span)?;
            check_value(item)?;

            let index = row + row_offset;
            let index_value = item
                .as_record()
                .ok()
                .and_then(|val| val.get(INDEX_COLUMN_NAME))
                .map(|value| value.to_expanded_string("", cfg.opts.config))
                .unwrap_or_else(|| index.to_string());
            let index_value = NuTable::create(index_value);
            let index_width = index_value.width();

            table.insert_value((row + 1, 0), index_value);
            index_column_width = max(index_column_width, index_width);
        }

        if available_width <= index_column_width + extra_width {
            // NOTE: we don't wanna wrap index; so we return
            return Ok(None);
        }

        available_width -= index_column_width + extra_width;
        widths.push(index_column_width);
    }

    let count_columns = headers.len();
    let mut truncate = false;
    let mut rendered_column = 0;
    for (col, header) in headers.into_iter().enumerate() {
        let column = col + with_index as usize;
        if available_width <= extra_width {
            table.pop_column(table.count_columns() - column);
            truncate = true;
            break;
        }

        let mut available = available_width - extra_width;

        // We want to reserver some space for next column
        // If we can't fit it in it will be popped anyhow.
        let is_prelast_column = col + 2 == count_columns;
        let is_last_column = col + 1 == count_columns;
        if is_prelast_column {
            let need_width = MIN_CELL_WIDTH + SPLIT_LINE_SPACE;
            if available > need_width {
                available -= need_width;
            }
        } else if !is_last_column {
            let need_width: usize = truncate_column_width + SPLIT_LINE_SPACE;
            if available > need_width {
                available -= need_width;
            }
        }

        let mut total_column_rows = 0usize;
        let mut column_width = 0;

        for (row, item) in input.iter().enumerate() {
            cfg.opts.signals.check(cfg.opts.span)?;
            check_value(item)?;

            let inner_cfg = cfg_expand_reset_table(cfg.clone(), available);
            let cell = expand_entry_with_header(item, &header, inner_cfg);
            // TODO: optimize cause when we expand we alrready know the width (most of the time or all)
            let mut value = NuTable::create(cell.text);
            let mut value_width = value.width();
            if value_width > available {
                // NOTE:
                // most likely it was emojie which we are not sure about what to do
                // so we truncate it just in case
                //
                // most likely width is 1

                value = NuTable::create(String::from("\u{FFFD}"));
                value_width = 1;
            }

            column_width = max(column_width, value_width);

            table.insert_value((row + 1, column), value);
            table.insert_style((row + 1, column), cell.style);

            total_column_rows = total_column_rows.saturating_add(cell.size);
        }

        let mut head_width = string_width(&header);
        let mut header = header;
        if head_width > available {
            header = wrap_text(&header, available, cfg.opts.config);
            head_width = available;
        }

        table.insert((0, column), header);

        column_width = max(column_width, head_width);
        assert!(column_width <= available);

        widths.push(column_width);

        available_width -= column_width + extra_width;
        rendered_column += 1;

        total_rows = std::cmp::max(total_rows, total_column_rows);
    }

    if truncate {
        if rendered_column == 0 {
            // it means that no actual data was rendered, there might be only index present,
            // so there's no point in rendering the table.
            //
            // It's actually quite important in case it's called recursively,
            // cause we will back up to the basic table view as a string e.g. '[table 123 columns]'.
            //
            // But potentially if its reached as a 1st called function we might would love to see the index.

            return Ok(None);
        }

        if available_width < truncate_column_width {
            // back up by removing last column.
            // it's LIKELY that removing only 1 column will leave us enough space for a shift column.
            while let Some(width) = widths.pop() {
                table.pop_column(1);

                available_width += width + pad_width;
                if !widths.is_empty() {
                    available_width += SPLIT_LINE_SPACE;
                }

                if available_width > truncate_column_width {
                    break;
                }
            }
        }

        // this must be a RARE case or even NEVER happen,
        // but we do check it just in case.
        if available_width < truncate_column_width {
            return Ok(None);
        }

        let is_last_column = widths.len() == count_columns;
        if !is_last_column {
            table.push_column(String::from("..."));
            widths.push(3);
        }
    }

    Ok(Some(TableOutput::new(table, true, with_index, total_rows)))
}

fn expanded_table_kv(record: &Record, cfg: Cfg<'_>) -> CellResult {
    let theme = load_theme(cfg.opts.mode);
    let theme = theme.as_base();
    let key_width = record
        .columns()
        .map(|col| string_width(col))
        .max()
        .unwrap_or(0);
    let count_borders = theme.borders_has_vertical() as usize
        + theme.borders_has_right() as usize
        + theme.borders_has_left() as usize;
    let pad = cfg.opts.config.table.padding.left + cfg.opts.config.table.padding.right;
    if key_width + count_borders + pad + pad > cfg.opts.width {
        return Ok(None);
    }

    let value_width = cfg.opts.width - key_width - count_borders - pad - pad;

    let mut total_rows = 0usize;

    let mut table = NuTable::new(record.len(), 2);
    table.set_index_style(get_key_style(&cfg));
    table.set_indent(cfg.opts.config.table.padding);

    for (i, (key, value)) in record.iter().enumerate() {
        cfg.opts.signals.check(cfg.opts.span)?;

        let cell = match expand_value(value, value_width, &cfg)? {
            Some(val) => val,
            None => return Ok(None),
        };

        let value = cell.text;
        let mut key = key.to_owned();

        // we want to have a key being aligned to 2nd line,
        // we could use Padding for it but,
        // the easiest way to do so is just push a new_line char before
        let is_key_on_next_line = !key.is_empty() && cell.is_expanded && theme.borders_has_top();
        if is_key_on_next_line {
            key.insert(0, '\n');
        }

        table.insert((i, 0), key);
        table.insert((i, 1), value);

        total_rows = total_rows.saturating_add(cell.size);
    }

    let mut out = TableOutput::new(table, false, true, total_rows);

    configure_table(
        &mut out,
        cfg.opts.config,
        &cfg.opts.style_computer,
        cfg.opts.mode,
    );

    maybe_expand_table(out, cfg.opts.width)
        .map(|value| value.map(|value| CellOutput::clean(value, total_rows, false)))
}

// the flag is used as an optimization to not do `value.lines().count()` search.
fn expand_value(value: &Value, width: usize, cfg: &Cfg<'_>) -> CellResult {
    if is_limit_reached(cfg) {
        let value = value_to_string_clean(value, cfg);
        return Ok(Some(CellOutput::clean(value, 1, false)));
    }

    let span = value.span();
    match value {
        Value::List { vals, .. } => {
            let inner_cfg = cfg_expand_reset_table(cfg_expand_next_level(cfg.clone(), span), width);
            let table = expand_list(vals, inner_cfg)?;

            match table {
                Some(mut out) => {
                    table_apply_config(&mut out, cfg);
                    let value = out.table.draw_unchecked(width);
                    match value {
                        Some(value) => Ok(Some(CellOutput::clean(value, out.count_rows, true))),
                        None => Ok(None),
                    }
                }
                None => {
                    // it means that the list is empty
                    let value = value_to_wrapped_string(value, cfg, width);
                    Ok(Some(CellOutput::text(value)))
                }
            }
        }
        Value::Record { val: record, .. } => {
            if record.is_empty() {
                // Like list case return styled string instead of empty value
                let value = value_to_wrapped_string(value, cfg, width);
                return Ok(Some(CellOutput::text(value)));
            }

            let inner_cfg = cfg_expand_reset_table(cfg_expand_next_level(cfg.clone(), span), width);
            let result = expanded_table_kv(record, inner_cfg)?;
            match result {
                Some(result) => Ok(Some(CellOutput::clean(result.text, result.size, true))),
                None => {
                    let value = value_to_wrapped_string(value, cfg, width);
                    Ok(Some(CellOutput::text(value)))
                }
            }
        }
        _ => {
            let value = value_to_wrapped_string_clean(value, cfg, width);
            Ok(Some(CellOutput::text(value)))
        }
    }
}

fn get_key_style(cfg: &Cfg<'_>) -> TextStyle {
    get_header_style(&cfg.opts.style_computer).alignment(Alignment::Left)
}

fn expand_entry_with_header(item: &Value, header: &str, cfg: Cfg<'_>) -> CellOutput {
    match item {
        Value::Record { val, .. } => match val.get(header) {
            Some(val) => expand_entry(val, cfg),
            None => CellOutput::styled(error_sign(
                cfg.opts.config.table.missing_value_symbol.clone(),
                &cfg.opts.style_computer,
            )),
        },
        _ => expand_entry(item, cfg),
    }
}

fn expand_entry(item: &Value, cfg: Cfg<'_>) -> CellOutput {
    if is_limit_reached(&cfg) {
        let value = nu_value_to_string_clean(item, cfg.opts.config, &cfg.opts.style_computer);
        let value = nutext_wrap(value, &cfg);
        return CellOutput::styled(value);
    }

    let span = item.span();
    match &item {
        Value::Record { val: record, .. } => {
            if record.is_empty() {
                let value = nu_value_to_string(item, cfg.opts.config, &cfg.opts.style_computer);
                let value = nutext_wrap(value, &cfg);
                return CellOutput::styled(value);
            }

            // we verify what is the structure of a Record cause it might represent
            let inner_cfg = cfg_expand_next_level(cfg.clone(), span);
            let table = expanded_table_kv(record, inner_cfg);

            match table {
                Ok(Some(table)) => table,
                _ => {
                    let value = nu_value_to_string(item, cfg.opts.config, &cfg.opts.style_computer);
                    let value = nutext_wrap(value, &cfg);
                    CellOutput::styled(value)
                }
            }
        }
        Value::List { vals, .. } => {
            if cfg.format.flatten && is_simple_list(vals) {
                let value = list_to_string(
                    vals,
                    cfg.opts.config,
                    &cfg.opts.style_computer,
                    &cfg.format.flatten_sep,
                );
                return CellOutput::text(value);
            }

            let inner_cfg = cfg_expand_next_level(cfg.clone(), span);
            let table = expand_list(vals, inner_cfg);

            let mut out = match table {
                Ok(Some(out)) => out,
                _ => {
                    let value = nu_value_to_string(item, cfg.opts.config, &cfg.opts.style_computer);
                    let value = nutext_wrap(value, &cfg);
                    return CellOutput::styled(value);
                }
            };

            table_apply_config(&mut out, &cfg);

            let table = out.table.draw_unchecked(cfg.opts.width);
            match table {
                Some(table) => CellOutput::clean(table, out.count_rows, false),
                None => {
                    let value = nu_value_to_string(item, cfg.opts.config, &cfg.opts.style_computer);
                    let value = nutext_wrap(value, &cfg);
                    CellOutput::styled(value)
                }
            }
        }
        _ => {
            let value = nu_value_to_string_clean(item, cfg.opts.config, &cfg.opts.style_computer);
            let value = nutext_wrap(value, &cfg);
            CellOutput::styled(value)
        }
    }
}

fn nutext_wrap(mut text: NuText, cfg: &Cfg<'_>) -> NuText {
    let width = string_width(&text.0);
    if width > cfg.opts.width {
        text.0 = wrap_text(&text.0, cfg.opts.width, cfg.opts.config);
    }

    text
}

fn is_limit_reached(cfg: &Cfg<'_>) -> bool {
    matches!(cfg.format.expand_limit, Some(0))
}

fn is_simple_list(vals: &[Value]) -> bool {
    vals.iter()
        .all(|v| !matches!(v, Value::Record { .. } | Value::List { .. }))
}

fn list_to_string(
    vals: &[Value],
    config: &Config,
    style_computer: &StyleComputer,
    sep: &str,
) -> String {
    let mut buf = String::new();
    for (i, value) in vals.iter().enumerate() {
        if i > 0 {
            buf.push_str(sep);
        }

        let (text, _) = nu_value_to_string_clean(value, config, style_computer);
        buf.push_str(&text);
    }

    buf
}

fn maybe_expand_table(mut out: TableOutput, term_width: usize) -> StringResult {
    let total_width = out.table.total_width();
    if total_width < term_width {
        const EXPAND_THRESHOLD: f32 = 0.80;
        let used_percent = total_width as f32 / term_width as f32;
        let need_expansion = total_width < term_width && used_percent > EXPAND_THRESHOLD;
        if need_expansion {
            out.table.set_strategy(true);
        }
    }

    let table = out.table.draw_unchecked(term_width);

    Ok(table)
}

fn table_apply_config(out: &mut TableOutput, cfg: &Cfg<'_>) {
    configure_table(
        out,
        cfg.opts.config,
        &cfg.opts.style_computer,
        cfg.opts.mode,
    )
}

fn value_to_string(value: &Value, cfg: &Cfg<'_>) -> String {
    nu_value_to_string(value, cfg.opts.config, &cfg.opts.style_computer).0
}

fn value_to_string_clean(value: &Value, cfg: &Cfg<'_>) -> String {
    nu_value_to_string_clean(value, cfg.opts.config, &cfg.opts.style_computer).0
}

fn value_to_wrapped_string(value: &Value, cfg: &Cfg<'_>, value_width: usize) -> String {
    wrap_text(&value_to_string(value, cfg), value_width, cfg.opts.config)
}

fn value_to_wrapped_string_clean(value: &Value, cfg: &Cfg<'_>, value_width: usize) -> String {
    let text = nu_value_to_string_colored(value, cfg.opts.config, &cfg.opts.style_computer);
    wrap_text(&text, value_width, cfg.opts.config)
}

fn cfg_expand_next_level(mut cfg: Cfg<'_>, span: Span) -> Cfg<'_> {
    cfg.opts.span = span;
    if let Some(deep) = cfg.format.expand_limit.as_mut() {
        *deep -= 1
    }

    cfg
}

fn cfg_expand_reset_table(mut cfg: Cfg<'_>, width: usize) -> Cfg<'_> {
    cfg.opts.width = width;
    cfg.opts.index_offset = 0;
    cfg
}
