use std::{cmp::max, collections::HashMap};

use nu_color_config::{Alignment, StyleComputer, TextStyle};
use nu_engine::column::get_columns;
use nu_protocol::{Config, Record, ShellError, Span, Value};

use tabled::grid::config::Position;

use crate::{
    common::{
        check_value, configure_table, error_sign, get_header_style, get_index_style, load_theme,
        nu_value_to_string, nu_value_to_string_clean, nu_value_to_string_colored, wrap_text,
        NuText, StringResult, TableResult, INDEX_COLUMN_NAME,
    },
    string_width,
    types::has_index,
    NuRecordsValue, NuTable, TableOpts, TableOutput,
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
    let mut available_width = cfg
        .opts
        .width
        .saturating_sub(SPLIT_LINE_SPACE + SPLIT_LINE_SPACE);
    if available_width < MIN_CELL_CONTENT_WIDTH {
        return Ok(None);
    }

    let headers = get_columns(input);

    let with_index = has_index(&cfg.opts, &headers);
    let row_offset = cfg.opts.index_offset;
    let mut rows_count = 0usize;

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
            data[0].push(NuRecordsValue::exact(String::from("#"), 1, vec![]));
        }

        for (row, item) in input.iter().enumerate() {
            cfg.opts.signals.check(cfg.opts.span)?;
            check_value(item)?;

            let index = row + row_offset;
            let text = item
                .as_record()
                .ok()
                .and_then(|val| val.get(INDEX_COLUMN_NAME))
                .map(|value| value.to_expanded_string("", cfg.opts.config))
                .unwrap_or_else(|| index.to_string());

            let row = row + with_header as usize;
            let value = NuRecordsValue::new(text);
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
            cfg.opts.signals.check(cfg.opts.span)?;
            check_value(item)?;

            let inner_cfg = cfg_expand_reset_table(cfg.clone(), available_width);
            let mut cell = expand_entry(item, inner_cfg);

            let value_width = string_width(&cell.text);
            if value_width > available_width {
                // it must only happen when a string is produced, so we can safely wrap it.
                // (it might be string table representation as well) (I guess I mean default { table ...} { list ...})
                //
                // todo: Maybe convert_to_table2_entry could do for strings to not mess caller code?

                cell.text = wrap_text(&cell.text, available_width, cfg.opts.config);
            }

            let value = NuRecordsValue::new(cell.text);
            data[row].push(value);
            data_styles.insert((row, with_index as usize), cell.style);

            rows_count = rows_count.saturating_add(cell.size);
        }

        let mut table = NuTable::from(data);
        table.set_indent(cfg.opts.config.table.padding);
        table.set_index_style(get_index_style(&cfg.opts.style_computer));
        set_data_styles(&mut table, data_styles);

        return Ok(Some(TableOutput::new(table, false, with_index, rows_count)));
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

        let mut column_rows = 0usize;

        for (row, item) in input.iter().enumerate() {
            cfg.opts.signals.check(cfg.opts.span)?;
            check_value(item)?;

            let inner_cfg = cfg_expand_reset_table(cfg.clone(), available);
            let mut cell = expand_entry_with_header(item, &header, inner_cfg);

            let mut value_width = string_width(&cell.text);
            if value_width > available {
                // it must only happen when a string is produced, so we can safely wrap it.
                // (it might be string table representation as well)

                cell.text = wrap_text(&cell.text, available, cfg.opts.config);
                value_width = available;
            }

            column_width = max(column_width, value_width);

            let value = NuRecordsValue::new(cell.text);
            data[row + 1].push(value);
            data_styles.insert((row + 1, col + with_index as usize), cell.style);

            column_rows = column_rows.saturating_add(cell.size);
        }

        let head_cell = NuRecordsValue::new(header);
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

        rows_count = std::cmp::max(rows_count, column_rows);
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
            let shift = NuRecordsValue::exact(String::from("..."), 3, vec![]);
            for row in &mut data {
                row.push(shift.clone());
            }

            widths.push(3);
        }
    }

    let mut table = NuTable::from(data);
    table.set_index_style(get_index_style(&cfg.opts.style_computer));
    table.set_header_style(get_header_style(&cfg.opts.style_computer));
    table.set_indent(cfg.opts.config.table.padding);
    set_data_styles(&mut table, data_styles);

    Ok(Some(TableOutput::new(table, true, with_index, rows_count)))
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
    let padding = 2;
    if key_width + count_borders + padding + padding > cfg.opts.width {
        return Ok(None);
    }

    let value_width = cfg.opts.width - key_width - count_borders - padding - padding;

    let mut count_rows = 0usize;

    let mut data = Vec::with_capacity(record.len());
    for (key, value) in record {
        cfg.opts.signals.check(cfg.opts.span)?;

        let cell = match expand_value(value, value_width, &cfg)? {
            Some(val) => val,
            None => return Ok(None),
        };

        // we want to have a key being aligned to 2nd line,
        // we could use Padding for it but,
        // the easiest way to do so is just push a new_line char before
        let mut key = key.to_owned();
        let is_key_on_next_line = !key.is_empty() && cell.is_expanded && theme.borders_has_top();
        if is_key_on_next_line {
            key.insert(0, '\n');
        }

        let key = NuRecordsValue::new(key);
        let val = NuRecordsValue::new(cell.text);
        let row = vec![key, val];

        data.push(row);

        count_rows = count_rows.saturating_add(cell.size);
    }

    let mut table = NuTable::from(data);
    table.set_index_style(get_key_style(&cfg));
    table.set_indent(cfg.opts.config.table.padding);

    let mut out = TableOutput::new(table, false, true, count_rows);

    configure_table(
        &mut out,
        cfg.opts.config,
        &cfg.opts.style_computer,
        cfg.opts.mode,
    );

    maybe_expand_table(out, cfg.opts.width)
        .map(|value| value.map(|value| CellOutput::clean(value, count_rows, false)))
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
                    let value = out.table.draw(width);
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
            None => CellOutput::styled(error_sign(&cfg.opts.style_computer)),
        },
        _ => expand_entry(item, cfg),
    }
}

fn expand_entry(item: &Value, cfg: Cfg<'_>) -> CellOutput {
    if is_limit_reached(&cfg) {
        let value = nu_value_to_string_clean(item, cfg.opts.config, &cfg.opts.style_computer);
        return CellOutput::styled(value);
    }

    let span = item.span();
    match &item {
        Value::Record { val: record, .. } => {
            if record.is_empty() {
                let value = nu_value_to_string(item, cfg.opts.config, &cfg.opts.style_computer);
                return CellOutput::styled(value);
            }

            // we verify what is the structure of a Record cause it might represent
            let inner_cfg = cfg_expand_next_level(cfg.clone(), span);
            let table = expanded_table_kv(record, inner_cfg);

            match table {
                Ok(Some(table)) => table,
                _ => {
                    let value = nu_value_to_string(item, cfg.opts.config, &cfg.opts.style_computer);
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
                    return CellOutput::styled(value);
                }
            };

            table_apply_config(&mut out, &cfg);

            let table = out.table.draw(usize::MAX);
            match table {
                Some(table) => CellOutput::clean(table, out.count_rows, false),
                None => {
                    let value = nu_value_to_string(item, cfg.opts.config, &cfg.opts.style_computer);
                    CellOutput::styled(value)
                }
            }
        }
        _ => {
            let value = nu_value_to_string_clean(item, cfg.opts.config, &cfg.opts.style_computer);
            CellOutput::styled(value)
        }
    }
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

    let table = out.table.draw(term_width);

    Ok(table)
}

fn set_data_styles(table: &mut NuTable, styles: HashMap<Position, TextStyle>) {
    for (pos, style) in styles {
        table.insert_style(pos, style);
    }
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
