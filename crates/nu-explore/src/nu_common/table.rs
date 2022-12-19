use nu_color_config::{Alignment, StyleComputer, TextStyle};
use nu_engine::column::get_columns;
use nu_protocol::FooterMode;
use nu_protocol::{ast::PathMember, Config, ShellError, Span, TableIndexMode, Value};
use nu_table::{string_width, Table as NuTable, TableConfig, TableTheme};
use std::sync::Arc;
use std::{
    cmp::max,
    sync::atomic::{AtomicBool, Ordering},
};

const INDEX_COLUMN_NAME: &str = "index";

type NuText = (String, TextStyle);
use crate::nu_common::NuConfig;

pub fn try_build_table(
    ctrlc: Option<Arc<AtomicBool>>,
    config: &NuConfig,
    style_computer: &StyleComputer,
    value: Value,
) -> String {
    match value {
        Value::List { vals, span } => try_build_list(vals, &ctrlc, config, span, style_computer),
        Value::Record { cols, vals, span } => {
            try_build_map(cols, vals, span, style_computer, ctrlc, config)
        }
        val => value_to_styled_string(&val, config, style_computer).0,
    }
}

fn try_build_map(
    cols: Vec<String>,
    vals: Vec<Value>,
    span: Span,
    style_computer: &StyleComputer,
    ctrlc: Option<Arc<AtomicBool>>,
    config: &NuConfig,
) -> String {
    let result = build_expanded_table(
        cols.clone(),
        vals.clone(),
        span,
        ctrlc,
        config,
        style_computer,
        usize::MAX,
        None,
        false,
        "",
    );
    match result {
        Ok(Some(result)) => result,
        Ok(None) | Err(_) => {
            value_to_styled_string(&Value::Record { cols, vals, span }, config, style_computer).0
        }
    }
}

fn try_build_list(
    vals: Vec<Value>,
    ctrlc: &Option<Arc<AtomicBool>>,
    config: &NuConfig,
    span: Span,
    style_computer: &StyleComputer,
) -> String {
    let table = convert_to_table2(
        0,
        vals.iter(),
        ctrlc.clone(),
        config,
        span,
        style_computer,
        None,
        false,
        "",
        usize::MAX,
    );
    match table {
        Ok(Some((table, with_header, with_index))) => {
            let table_config = create_table_config(
                config,
                style_computer,
                table.count_rows(),
                with_header,
                with_index,
                false,
            );

            let val = table.draw(table_config, usize::MAX);

            match val {
                Some(result) => result,
                None => {
                    value_to_styled_string(&Value::List { vals, span }, config, style_computer).0
                }
            }
        }
        Ok(None) | Err(_) => {
            // it means that the list is empty
            value_to_styled_string(&Value::List { vals, span }, config, style_computer).0
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn build_expanded_table(
    cols: Vec<String>,
    vals: Vec<Value>,
    span: Span,
    ctrlc: Option<Arc<AtomicBool>>,
    config: &Config,
    style_computer: &StyleComputer,
    term_width: usize,
    expand_limit: Option<usize>,
    flatten: bool,
    flatten_sep: &str,
) -> Result<Option<String>, ShellError> {
    let theme = load_theme_from_config(config);

    // calculate the width of a key part + the rest of table so we know the rest of the table width available for value.
    let key_width = cols.iter().map(|col| string_width(col)).max().unwrap_or(0);
    let key = NuTable::create_cell(" ".repeat(key_width), TextStyle::default());
    let key_table = NuTable::new(vec![vec![key]], (1, 2));
    let key_width = key_table
        .draw(
            create_table_config(config, style_computer, 1, false, false, false),
            usize::MAX,
        )
        .map(|table| string_width(&table))
        .unwrap_or(0);

    // 3 - count borders (left, center, right)
    // 2 - padding
    if key_width + 3 + 2 > term_width {
        return Ok(None);
    }

    let remaining_width = term_width - key_width - 3 - 2;

    let mut data = Vec::with_capacity(cols.len());
    for (key, value) in cols.into_iter().zip(vals) {
        // handle CTRLC event
        if let Some(ctrlc) = &ctrlc {
            if ctrlc.load(Ordering::SeqCst) {
                return Ok(None);
            }
        }

        let is_limited = matches!(expand_limit, Some(0));
        let mut is_expanded = false;
        let value = if is_limited {
            value_to_styled_string(&value, config, style_computer).0
        } else {
            let deep = expand_limit.map(|i| i - 1);

            match value {
                Value::List { vals, .. } => {
                    let table = convert_to_table2(
                        0,
                        vals.iter(),
                        ctrlc.clone(),
                        config,
                        span,
                        style_computer,
                        deep,
                        flatten,
                        flatten_sep,
                        remaining_width,
                    )?;

                    match table {
                        Some((mut table, with_header, with_index)) => {
                            // controll width via removing table columns.
                            let theme = load_theme_from_config(config);
                            table.truncate(remaining_width, &theme);

                            is_expanded = true;

                            let table_config = create_table_config(
                                config,
                                style_computer,
                                table.count_rows(),
                                with_header,
                                with_index,
                                false,
                            );

                            let val = table.draw(table_config, remaining_width);
                            match val {
                                Some(result) => result,
                                None => return Ok(None),
                            }
                        }
                        None => {
                            // it means that the list is empty
                            let value = Value::List { vals, span };
                            value_to_styled_string(&value, config, style_computer).0
                        }
                    }
                }
                Value::Record { cols, vals, span } => {
                    let result = build_expanded_table(
                        cols.clone(),
                        vals.clone(),
                        span,
                        ctrlc.clone(),
                        config,
                        style_computer,
                        remaining_width,
                        deep,
                        flatten,
                        flatten_sep,
                    )?;

                    match result {
                        Some(result) => {
                            is_expanded = true;
                            result
                        }
                        None => {
                            let failed_value = value_to_styled_string(
                                &Value::Record { cols, vals, span },
                                config,
                                style_computer,
                            );

                            nu_table::wrap_string(&failed_value.0, remaining_width)
                        }
                    }
                }
                val => {
                    let text = value_to_styled_string(&val, config, style_computer).0;
                    nu_table::wrap_string(&text, remaining_width)
                }
            }
        };

        // we want to have a key being aligned to 2nd line,
        // we could use Padding for it but,
        // the easiest way to do so is just push a new_line char before
        let mut key = key;
        if !key.is_empty() && is_expanded && theme.has_top_line() {
            key.insert(0, '\n');
        }

        let key = NuTable::create_cell(key, TextStyle::default_field());
        let val = NuTable::create_cell(value, TextStyle::default());

        let row = vec![key, val];
        data.push(row);
    }

    let table_config = create_table_config(config, style_computer, data.len(), false, false, false);

    let data_len = data.len();
    let table = NuTable::new(data, (data_len, 2));

    let table_s = table.clone().draw(table_config.clone(), term_width);

    let table = match table_s {
        Some(s) => {
            // check whether we need to expand table or not,
            // todo: we can make it more effitient

            const EXPAND_TREASHHOLD: f32 = 0.80;

            let width = string_width(&s);
            let used_percent = width as f32 / term_width as f32;

            if width < term_width && used_percent > EXPAND_TREASHHOLD {
                let table_config = table_config.expand();
                table.draw(table_config, term_width)
            } else {
                Some(s)
            }
        }
        None => None,
    };

    Ok(table)
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::into_iter_on_ref)]
fn convert_to_table2<'a>(
    row_offset: usize,
    input: impl Iterator<Item = &'a Value> + ExactSizeIterator + Clone,
    ctrlc: Option<Arc<AtomicBool>>,
    config: &Config,
    head: Span,
    style_computer: &StyleComputer,
    deep: Option<usize>,
    flatten: bool,
    flatten_sep: &str,
    available_width: usize,
) -> Result<Option<(NuTable, bool, bool)>, ShellError> {
    const PADDING_SPACE: usize = 2;
    const SPLIT_LINE_SPACE: usize = 1;
    const ADDITIONAL_CELL_SPACE: usize = PADDING_SPACE + SPLIT_LINE_SPACE;
    const TRUNCATE_CELL_WIDTH: usize = 3;
    const MIN_CELL_CONTENT_WIDTH: usize = 1;
    const OK_CELL_CONTENT_WIDTH: usize = 25;

    if input.len() == 0 {
        return Ok(None);
    }

    // 2 - split lines
    let mut available_width = available_width.saturating_sub(SPLIT_LINE_SPACE + SPLIT_LINE_SPACE);
    if available_width < MIN_CELL_CONTENT_WIDTH {
        return Ok(None);
    }

    let headers = get_columns(input.clone());

    let with_index = match config.table_index_mode {
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

    let mut data = vec![vec![]; input.len()];
    if !headers.is_empty() {
        data.push(vec![]);
    };

    if with_index {
        let mut column_width = 0;

        if with_header {
            data[0].push(NuTable::create_cell(
                "#",
                header_style(style_computer, String::from("#")),
            ));
        }

        for (row, item) in input.clone().into_iter().enumerate() {
            if let Some(ctrlc) = &ctrlc {
                if ctrlc.load(Ordering::SeqCst) {
                    return Ok(None);
                }
            }

            if let Value::Error { error } = item {
                return Err(error.clone());
            }

            let index = row + row_offset;
            let text = matches!(item, Value::Record { .. })
                .then(|| lookup_index_value(item, config).unwrap_or_else(|| index.to_string()))
                .unwrap_or_else(|| index.to_string());

            let value = make_index_string(text, style_computer);

            let width = string_width(&value.0);
            column_width = max(column_width, width);

            let value = NuTable::create_cell(value.0, value.1);

            let row = if with_header { row + 1 } else { row };
            data[row].push(value);
        }

        if column_width + ADDITIONAL_CELL_SPACE > available_width {
            available_width = 0;
        } else {
            available_width -= column_width + ADDITIONAL_CELL_SPACE;
        }
    }

    if !with_header {
        for (row, item) in input.into_iter().enumerate() {
            if let Some(ctrlc) = &ctrlc {
                if ctrlc.load(Ordering::SeqCst) {
                    return Ok(None);
                }
            }

            if let Value::Error { error } = item {
                return Err(error.clone());
            }

            let value = convert_to_table2_entry(
                item,
                config,
                &ctrlc,
                style_computer,
                deep,
                flatten,
                flatten_sep,
                available_width,
            );

            let value = NuTable::create_cell(value.0, value.1);
            data[row].push(value);
        }

        let count_columns = if with_index { 2 } else { 1 };
        let size = (data.len(), count_columns);

        let table = NuTable::new(data, size);

        return Ok(Some((table, with_header, with_index)));
    }

    let mut widths = Vec::new();
    let mut truncate = false;
    let count_columns = headers.len();
    for (col, header) in headers.into_iter().enumerate() {
        let is_last_col = col + 1 == count_columns;

        let mut nessary_space = PADDING_SPACE;
        if !is_last_col {
            nessary_space += SPLIT_LINE_SPACE;
        }

        if available_width == 0 || available_width <= nessary_space {
            // MUST NEVER HAPPEN (ideally)
            // but it does...

            truncate = true;
            break;
        }

        available_width -= nessary_space;

        let mut column_width = string_width(&header);

        data[0].push(NuTable::create_cell(
            &header,
            header_style(style_computer, header.clone()),
        ));

        for (row, item) in input.clone().into_iter().enumerate() {
            if let Some(ctrlc) = &ctrlc {
                if ctrlc.load(Ordering::SeqCst) {
                    return Ok(None);
                }
            }

            if let Value::Error { error } = item {
                return Err(error.clone());
            }

            let value = create_table2_entry(
                item,
                &header,
                head,
                config,
                &ctrlc,
                style_computer,
                deep,
                flatten,
                flatten_sep,
                available_width,
            );

            let value_width = string_width(&value.0);
            column_width = max(column_width, value_width);

            let value = NuTable::create_cell(value.0, value.1);

            data[row + 1].push(value);
        }

        if column_width >= available_width
            || (!is_last_col && column_width + nessary_space >= available_width)
        {
            // so we try to do soft landing
            // by doing a truncating in case there will be enough space for it.

            column_width = string_width(&header);

            for (row, item) in input.clone().into_iter().enumerate() {
                if let Some(ctrlc) = &ctrlc {
                    if ctrlc.load(Ordering::SeqCst) {
                        return Ok(None);
                    }
                }

                let value = create_table2_entry_basic(item, &header, head, config, style_computer);
                let value = wrap_nu_text(value, available_width);

                let value_width = string_width(&value.0);
                column_width = max(column_width, value_width);

                let value = NuTable::create_cell(value.0, value.1);

                *data[row + 1].last_mut().expect("unwrap") = value;
            }
        }

        let is_suitable_for_wrap =
            available_width >= string_width(&header) && available_width >= OK_CELL_CONTENT_WIDTH;
        if column_width >= available_width && is_suitable_for_wrap {
            // so we try to do soft landing ONCE AGAIN
            // but including a wrap

            column_width = string_width(&header);

            for (row, item) in input.clone().into_iter().enumerate() {
                if let Some(ctrlc) = &ctrlc {
                    if ctrlc.load(Ordering::SeqCst) {
                        return Ok(None);
                    }
                }

                let value = create_table2_entry_basic(item, &header, head, config, style_computer);
                let value = wrap_nu_text(value, OK_CELL_CONTENT_WIDTH);

                let value = NuTable::create_cell(value.0, value.1);

                *data[row + 1].last_mut().expect("unwrap") = value;
            }
        }

        if column_width > available_width {
            // remove just added column
            for row in &mut data {
                row.pop();
            }

            available_width += nessary_space;

            truncate = true;
            break;
        }

        available_width -= column_width;
        widths.push(column_width);
    }

    if truncate {
        if available_width <= TRUNCATE_CELL_WIDTH + PADDING_SPACE {
            // back up by removing last column.
            // it's ALWAYS MUST has us enough space for a shift column
            while let Some(width) = widths.pop() {
                for row in &mut data {
                    row.pop();
                }

                available_width += width + PADDING_SPACE + SPLIT_LINE_SPACE;

                if available_width > TRUNCATE_CELL_WIDTH + PADDING_SPACE {
                    break;
                }
            }
        }

        // this must be a RARE case or even NEVER happen,
        // but we do check it just in case.
        if widths.is_empty() {
            return Ok(None);
        }

        let shift = NuTable::create_cell(String::from("..."), TextStyle::default());
        for row in &mut data {
            row.push(shift.clone());
        }

        widths.push(3);
    }

    let count_columns = widths.len() + with_index as usize;
    let count_rows = data.len();
    let size = (count_rows, count_columns);

    let table = NuTable::new(data, size);

    Ok(Some((table, with_header, with_index)))
}

fn lookup_index_value(item: &Value, config: &Config) -> Option<String> {
    item.get_data_by_key(INDEX_COLUMN_NAME)
        .map(|value| value.into_string("", config))
}

fn header_style(style_computer: &StyleComputer, header: String) -> TextStyle {
    let style = style_computer.compute("header", &Value::string(header.as_str(), Span::unknown()));
    TextStyle {
        alignment: Alignment::Center,
        color_style: Some(style),
    }
}

#[allow(clippy::too_many_arguments)]
fn create_table2_entry_basic(
    item: &Value,
    header: &str,
    head: Span,
    config: &Config,
    style_computer: &StyleComputer,
) -> NuText {
    match item {
        Value::Record { .. } => {
            let val = header.to_owned();
            let path = PathMember::String { val, span: head };
            let val = item.clone().follow_cell_path(&[path], false);

            match val {
                Ok(val) => value_to_styled_string(&val, config, style_computer),
                Err(_) => error_sign(style_computer),
            }
        }
        _ => value_to_styled_string(item, config, style_computer),
    }
}

#[allow(clippy::too_many_arguments)]
fn create_table2_entry(
    item: &Value,
    header: &str,
    head: Span,
    config: &Config,
    ctrlc: &Option<Arc<AtomicBool>>,
    style_computer: &StyleComputer,
    deep: Option<usize>,
    flatten: bool,
    flatten_sep: &str,
    width: usize,
) -> NuText {
    match item {
        Value::Record { .. } => {
            let val = header.to_owned();
            let path = PathMember::String { val, span: head };
            let val = item.clone().follow_cell_path(&[path], false);

            match val {
                Ok(val) => convert_to_table2_entry(
                    &val,
                    config,
                    ctrlc,
                    style_computer,
                    deep,
                    flatten,
                    flatten_sep,
                    width,
                ),
                Err(_) => wrap_nu_text(error_sign(style_computer), width),
            }
        }
        _ => convert_to_table2_entry(
            item,
            config,
            ctrlc,
            style_computer,
            deep,
            flatten,
            flatten_sep,
            width,
        ),
    }
}

fn error_sign(style_computer: &StyleComputer) -> (String, TextStyle) {
    make_styled_string(style_computer, String::from("❎"), None, 0)
}

fn wrap_nu_text(mut text: NuText, width: usize) -> NuText {
    text.0 = nu_table::wrap_string(&text.0, width);
    text
}

#[allow(clippy::too_many_arguments)]
fn convert_to_table2_entry(
    item: &Value,
    config: &Config,
    ctrlc: &Option<Arc<AtomicBool>>,
    // This is passed in, even though it could be retrieved from config,
    // to save reallocation (because it's presumably being used upstream).
    style_computer: &StyleComputer,
    deep: Option<usize>,
    flatten: bool,
    flatten_sep: &str,
    width: usize,
) -> NuText {
    let is_limit_reached = matches!(deep, Some(0));
    if is_limit_reached {
        return wrap_nu_text(value_to_styled_string(item, config, style_computer), width);
    }

    match &item {
        Value::Record { span, cols, vals } => {
            if cols.is_empty() && vals.is_empty() {
                wrap_nu_text(value_to_styled_string(item, config, style_computer), width)
            } else {
                let table = convert_to_table2(
                    0,
                    std::iter::once(item),
                    ctrlc.clone(),
                    config,
                    *span,
                    style_computer,
                    deep.map(|i| i - 1),
                    flatten,
                    flatten_sep,
                    width,
                );

                let inner_table = table.map(|table| {
                    table.and_then(|(table, with_header, with_index)| {
                        let table_config = create_table_config(
                            config,
                            style_computer,
                            table.count_rows(),
                            with_header,
                            with_index,
                            false,
                        );

                        table.draw(table_config, usize::MAX)
                    })
                });

                if let Ok(Some(table)) = inner_table {
                    (table, TextStyle::default())
                } else {
                    // error so back down to the default
                    wrap_nu_text(value_to_styled_string(item, config, style_computer), width)
                }
            }
        }
        Value::List { vals, span } => {
            let is_simple_list = vals
                .iter()
                .all(|v| !matches!(v, Value::Record { .. } | Value::List { .. }));

            if flatten && is_simple_list {
                wrap_nu_text(
                    convert_value_list_to_string(vals, config, style_computer, flatten_sep),
                    width,
                )
            } else {
                let table = convert_to_table2(
                    0,
                    vals.iter(),
                    ctrlc.clone(),
                    config,
                    *span,
                    style_computer,
                    deep.map(|i| i - 1),
                    flatten,
                    flatten_sep,
                    width,
                );

                let inner_table = table.map(|table| {
                    table.and_then(|(table, with_header, with_index)| {
                        let table_config = create_table_config(
                            config,
                            style_computer,
                            table.count_rows(),
                            with_header,
                            with_index,
                            false,
                        );

                        table.draw(table_config, usize::MAX)
                    })
                });

                if let Ok(Some(table)) = inner_table {
                    (table, TextStyle::default())
                } else {
                    // error so back down to the default

                    wrap_nu_text(value_to_styled_string(item, config, style_computer), width)
                }
            }
        }
        _ => wrap_nu_text(value_to_styled_string(item, config, style_computer), width), // unknown type.
    }
}

fn convert_value_list_to_string(
    vals: &[Value],
    config: &Config,
    // This is passed in, even though it could be retrieved from config,
    // to save reallocation (because it's presumably being used upstream).
    style_computer: &StyleComputer,
    flatten_sep: &str,
) -> NuText {
    let mut buf = Vec::new();
    for value in vals {
        let (text, _) = value_to_styled_string(value, config, style_computer);

        buf.push(text);
    }
    let text = buf.join(flatten_sep);
    (text, TextStyle::default())
}

fn value_to_styled_string(
    value: &Value,
    config: &Config,
    // This is passed in, even though it could be retrieved from config,
    // to save reallocation (because it's presumably being used upstream).
    style_computer: &StyleComputer,
) -> NuText {
    let float_precision = config.float_precision as usize;
    make_styled_string(
        style_computer,
        value.into_abbreviated_string(config),
        Some(value),
        float_precision,
    )
}

fn make_styled_string(
    style_computer: &StyleComputer,
    text: String,
    value: Option<&Value>, // None represents table holes.
    float_precision: usize,
) -> NuText {
    match value {
        Some(value) => {
            match value {
                Value::Float { .. } => {
                    // set dynamic precision from config
                    let precise_number = match convert_with_precision(&text, float_precision) {
                        Ok(num) => num,
                        Err(e) => e.to_string(),
                    };
                    (precise_number, style_computer.style_primitive(value))
                }
                _ => (text, style_computer.style_primitive(value)),
            }
        }
        None => {
            // Though holes are not the same as null, the closure for "empty" is passed a null anyway.
            (
                text,
                TextStyle::with_style(
                    Alignment::Center,
                    style_computer.compute("empty", &Value::nothing(Span::unknown())),
                ),
            )
        }
    }
}

fn make_index_string(text: String, style_computer: &StyleComputer) -> NuText {
    let style = style_computer.compute("row_index", &Value::string(text.as_str(), Span::unknown()));
    (text, TextStyle::with_style(Alignment::Right, style))
}

fn convert_with_precision(val: &str, precision: usize) -> Result<String, ShellError> {
    // vall will always be a f64 so convert it with precision formatting
    let val_float = match val.trim().parse::<f64>() {
        Ok(f) => f,
        Err(e) => {
            return Err(ShellError::GenericError(
                format!("error converting string [{}] to f64", &val),
                "".to_string(),
                None,
                Some(e.to_string()),
                Vec::new(),
            ));
        }
    };
    Ok(format!("{:.prec$}", val_float, prec = precision))
}

fn load_theme_from_config(config: &Config) -> TableTheme {
    match config.table_mode.as_str() {
        "basic" => nu_table::TableTheme::basic(),
        "thin" => nu_table::TableTheme::thin(),
        "light" => nu_table::TableTheme::light(),
        "compact" => nu_table::TableTheme::compact(),
        "with_love" => nu_table::TableTheme::with_love(),
        "compact_double" => nu_table::TableTheme::compact_double(),
        "rounded" => nu_table::TableTheme::rounded(),
        "reinforced" => nu_table::TableTheme::reinforced(),
        "heavy" => nu_table::TableTheme::heavy(),
        "none" => nu_table::TableTheme::none(),
        _ => nu_table::TableTheme::rounded(),
    }
}

fn create_table_config(
    config: &Config,
    style_computer: &StyleComputer,
    count_records: usize,
    with_header: bool,
    with_index: bool,
    expand: bool,
) -> TableConfig {
    let theme = load_theme_from_config(config);
    let append_footer = with_footer(config, with_header, count_records);

    let mut table_cfg = TableConfig::new(theme, with_header, with_index, append_footer);

    table_cfg = table_cfg.splitline_style(lookup_separator_color(style_computer));

    if expand {
        table_cfg = table_cfg.expand();
    }

    table_cfg.trim(config.trim_strategy.clone())
}

fn lookup_separator_color(style_computer: &StyleComputer) -> nu_ansi_term::Style {
    style_computer.compute("separator", &Value::nothing(Span::unknown()))
}

fn with_footer(config: &Config, with_header: bool, count_records: usize) -> bool {
    with_header && need_footer(config, count_records as u64)
}

fn need_footer(config: &Config, count_records: u64) -> bool {
    matches!(config.footer_mode, FooterMode::RowCount(limit) if count_records > limit)
        || matches!(config.footer_mode, FooterMode::Always)
}
