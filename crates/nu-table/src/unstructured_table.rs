use nu_color_config::StyleComputer;
use nu_protocol::{Config, Record, Span, TableIndent, Value};

use tabled::{
    grid::{
        ansi::ANSIStr,
        config::{Borders, CompactMultilineConfig},
        dimension::{DimensionPriority, PoolTableDimension},
    },
    settings::{Alignment, Color, Padding, TableOption},
    tables::{PoolTable, TableValue},
};

use crate::{TableTheme, is_color_empty, string_width, string_wrap};

/// UnstructuredTable has a recursive table representation of nu_protocol::Value.
///
/// It doesn't support alignment and a proper width control (although it's possible to achieve).
pub struct UnstructuredTable {
    value: TableValue,
}

impl UnstructuredTable {
    pub fn new(value: Value, config: &Config) -> Self {
        let value = convert_nu_value_to_table_value(value, config);
        Self { value }
    }

    pub fn truncate(&mut self, theme: &TableTheme, width: usize) -> bool {
        let mut available = width;
        let has_vertical = theme.as_full().borders_has_left();
        if has_vertical {
            available = available.saturating_sub(2);
        }

        truncate_table_value(&mut self.value, has_vertical, available).is_none()
    }

    pub fn draw(self, theme: &TableTheme, indent: TableIndent, style: &StyleComputer) -> String {
        build_table(self.value, style, theme, indent)
    }
}

fn build_table(
    val: TableValue,
    style: &StyleComputer,
    theme: &TableTheme,
    indent: TableIndent,
) -> String {
    let mut table = PoolTable::from(val);

    let mut theme = theme.as_full().clone();
    theme.set_horizontal_lines(Default::default());

    table.with(Padding::new(indent.left, indent.right, 0, 0));
    table.with(*theme.get_borders());
    table.with(Alignment::left());
    table.with(PoolTableDimension::new(
        DimensionPriority::Last,
        DimensionPriority::Last,
    ));

    if let Some(color) = get_border_color(style)
        && !is_color_empty(&color)
    {
        return build_table_with_border_color(table, color);
    }

    table.to_string()
}

fn convert_nu_value_to_table_value(value: Value, config: &Config) -> TableValue {
    match value {
        Value::Record { val, .. } => build_vertical_map(val.into_owned(), config),
        Value::List { vals, .. } => {
            let rebuild_array_as_map = is_valid_record(&vals) && count_columns_in_record(&vals) > 0;
            if rebuild_array_as_map {
                build_map_from_record(vals, config)
            } else {
                build_vertical_array(vals, config)
            }
        }
        value => build_string_value(value, config),
    }
}

fn build_string_value(value: Value, config: &Config) -> TableValue {
    const MAX_STRING_WIDTH: usize = 50;
    const WRAP_STRING_WIDTH: usize = 30;

    let mut text = value.to_abbreviated_string(config);
    if string_width(&text) > MAX_STRING_WIDTH {
        text = string_wrap(&text, WRAP_STRING_WIDTH, false);
    }

    TableValue::Cell(text)
}

fn build_vertical_map(record: Record, config: &Config) -> TableValue {
    let max_key_width = record
        .iter()
        .map(|(k, _)| string_width(k))
        .max()
        .unwrap_or(0);

    let mut rows = Vec::with_capacity(record.len());
    for (mut key, value) in record {
        string_append_to_width(&mut key, max_key_width);

        let value = convert_nu_value_to_table_value(value, config);

        let row = TableValue::Row(vec![TableValue::Cell(key), value]);
        rows.push(row);
    }

    TableValue::Column(rows)
}

fn string_append_to_width(key: &mut String, max: usize) {
    let width = string_width(key);
    let rest = max - width;
    key.extend(std::iter::repeat_n(' ', rest));
}

fn build_vertical_array(vals: Vec<Value>, config: &Config) -> TableValue {
    let map = vals
        .into_iter()
        .map(|val| convert_nu_value_to_table_value(val, config))
        .collect();

    TableValue::Column(map)
}

fn is_valid_record(vals: &[Value]) -> bool {
    if vals.is_empty() {
        return true;
    }

    let first_value = match &vals[0] {
        Value::Record { val, .. } => val,
        _ => return false,
    };

    for val in &vals[1..] {
        match val {
            Value::Record { val, .. } => {
                let equal = val.columns().eq(first_value.columns());
                if !equal {
                    return false;
                }
            }
            _ => return false,
        }
    }

    true
}

fn count_columns_in_record(vals: &[Value]) -> usize {
    match vals.iter().next() {
        Some(Value::Record { val, .. }) => val.len(),
        _ => 0,
    }
}

fn build_map_from_record(vals: Vec<Value>, config: &Config) -> TableValue {
    // assumes that we have a valid record structure (checked by is_valid_record)

    let head = get_columns_in_record(&vals);
    let mut list = Vec::with_capacity(head.len());
    for col in head {
        list.push(TableValue::Column(vec![TableValue::Cell(col)]));
    }

    for val in vals {
        let val = get_as_record(val);
        for (i, (_, val)) in val.into_owned().into_iter().enumerate() {
            let value = convert_nu_value_to_table_value(val, config);
            let list = get_table_value_column_mut(&mut list[i]);

            list.push(value);
        }
    }

    TableValue::Row(list)
}

fn get_table_value_column_mut(val: &mut TableValue) -> &mut Vec<TableValue> {
    match val {
        TableValue::Column(row) => row,
        _ => {
            unreachable!();
        }
    }
}

fn get_as_record(val: Value) -> nu_utils::SharedCow<Record> {
    match val {
        Value::Record { val, .. } => val,
        _ => unreachable!(),
    }
}

fn get_columns_in_record(vals: &[Value]) -> Vec<String> {
    match vals.iter().next() {
        Some(Value::Record { val, .. }) => val.columns().cloned().collect(),
        _ => vec![],
    }
}

fn truncate_table_value(
    value: &mut TableValue,
    has_vertical: bool,
    available: usize,
) -> Option<usize> {
    const MIN_CONTENT_WIDTH: usize = 10;
    const TRUNCATE_CELL_WIDTH: usize = 3;
    const PAD: usize = 2;

    match value {
        TableValue::Row(row) => {
            if row.is_empty() {
                return Some(PAD);
            }

            if row.len() == 1 {
                return truncate_table_value(&mut row[0], has_vertical, available);
            }

            let count_cells = row.len();
            let mut row_width = 0;
            let mut i = 0;
            let mut last_used_width = 0;
            for cell in row.iter_mut() {
                let vertical = (has_vertical && i + 1 != count_cells) as usize;
                if available < row_width + vertical {
                    break;
                }

                let available = available - row_width - vertical;
                let width = match truncate_table_value(cell, has_vertical, available) {
                    Some(width) => width,
                    None => break,
                };

                row_width += width + vertical;
                last_used_width = row_width;
                i += 1;
            }

            if i == row.len() {
                return Some(row_width);
            }

            if i == 0 {
                if available >= PAD + TRUNCATE_CELL_WIDTH {
                    *value = TableValue::Cell(String::from("..."));
                    return Some(PAD + TRUNCATE_CELL_WIDTH);
                } else {
                    return None;
                }
            }

            let available = available - row_width;
            let has_space_empty_cell = available >= PAD + TRUNCATE_CELL_WIDTH;
            if has_space_empty_cell {
                row[i] = TableValue::Cell(String::from("..."));
                row.truncate(i + 1);
                row_width += PAD + TRUNCATE_CELL_WIDTH;
            } else if i == 0 {
                return None;
            } else {
                row[i - 1] = TableValue::Cell(String::from("..."));
                row.truncate(i);
                row_width -= last_used_width;
                row_width += PAD + TRUNCATE_CELL_WIDTH;
            }

            Some(row_width)
        }
        TableValue::Column(column) => {
            let mut max_width = PAD;
            for cell in column.iter_mut() {
                let width = truncate_table_value(cell, has_vertical, available)?;
                max_width = std::cmp::max(max_width, width);
            }

            Some(max_width)
        }
        TableValue::Cell(text) => {
            if available <= PAD {
                return None;
            }

            let available = available - PAD;
            let width = string_width(text);

            if width > available {
                if available > MIN_CONTENT_WIDTH {
                    *text = string_wrap(text, available, false);
                    Some(available + PAD)
                } else if available >= 3 {
                    *text = String::from("...");
                    Some(3 + PAD)
                } else {
                    // situation where we have too little space
                    None
                }
            } else {
                Some(width + PAD)
            }
        }
    }
}

fn build_table_with_border_color(mut table: PoolTable, color: Color) -> String {
    // NOTE: We have this function presizely because of color_into_ansistr internals
    // color must be alive  why we build table

    let color = color_into_ansistr(&color);
    table.with(SetBorderColor(color));
    table.to_string()
}

fn color_into_ansistr(color: &Color) -> ANSIStr<'static> {
    // # SAFETY
    //
    // It's perfectly save to do cause table does not store the reference internally.
    // We just need this unsafe section to cope with some limitations of [`PoolTable`].
    // Mitigation of this is definitely on a todo list.

    let prefix = color.get_prefix();
    let suffix = color.get_suffix();
    let prefix: &'static str = unsafe { std::mem::transmute(prefix) };
    let suffix: &'static str = unsafe { std::mem::transmute(suffix) };

    ANSIStr::new(prefix, suffix)
}

struct SetBorderColor(ANSIStr<'static>);

impl<R, D> TableOption<R, CompactMultilineConfig, D> for SetBorderColor {
    fn change(self, _: &mut R, cfg: &mut CompactMultilineConfig, _: &mut D) {
        let borders = Borders::filled(self.0);
        cfg.set_borders_color(borders);
    }
}

fn get_border_color(style: &StyleComputer<'_>) -> Option<Color> {
    // color_config closures for "separator" are just given a null.
    let color = style.compute("separator", &Value::nothing(Span::unknown()));
    let color = color.paint(" ").to_string();
    let color = Color::try_from(color);
    color.ok()
}
