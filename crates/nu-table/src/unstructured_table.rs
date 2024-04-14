use crate::{string_width, string_wrap, TableTheme};
use nu_color_config::StyleComputer;
use nu_protocol::{Config, Record, Span, Value};
use tabled::{
    grid::{
        color::{AnsiColor, StaticColor},
        config::{AlignmentHorizontal, Borders, CompactMultilineConfig},
        dimension::{DimensionPriority, PoolTableDimension},
    },
    settings::{style::RawStyle, Color, Padding, TableOption},
    tables::{PoolTable, TableValue},
};

/// UnstructuredTable has a recursive table representation of nu_protocol::Value.
///
/// It doesn't support alignment and a proper width control.
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
        let has_vertical = theme.has_left();
        if has_vertical {
            available = available.saturating_sub(2);
        }

        truncate_table_value(&mut self.value, has_vertical, available).is_none()
    }

    pub fn draw(
        self,
        style_computer: &StyleComputer,
        theme: &TableTheme,
        indent: (usize, usize),
    ) -> String {
        build_table(self.value, style_computer, theme, indent)
    }
}

fn build_table(
    val: TableValue,
    style_computer: &StyleComputer,
    theme: &TableTheme,
    indent: (usize, usize),
) -> String {
    let mut table = PoolTable::from(val);

    let mut theme = theme.get_theme_full();
    theme.set_horizontals(std::collections::HashMap::default());

    table.with(Padding::new(indent.0, indent.1, 0, 0));
    table.with(SetRawStyle(theme));
    table.with(SetAlignment(AlignmentHorizontal::Left));
    table.with(PoolTableDimension::new(
        DimensionPriority::Last,
        DimensionPriority::Last,
    ));

    // color_config closures for "separator" are just given a null.
    let color = style_computer.compute("separator", &Value::nothing(Span::unknown()));
    let color = color.paint(" ").to_string();
    if let Ok(color) = Color::try_from(color) {
        // # SAFETY
        //
        // It's perfectly save to do cause table does not store the reference internally.
        // We just need this unsafe section to cope with some limitations of [`PoolTable`].
        // Mitigation of this is definitely on a todo list.

        let color: AnsiColor<'_> = color.into();
        let prefix = color.get_prefix();
        let suffix = color.get_suffix();
        let prefix: &'static str = unsafe { std::mem::transmute(prefix) };
        let suffix: &'static str = unsafe { std::mem::transmute(suffix) };
        table.with(SetBorderColor(StaticColor::new(prefix, suffix)));
        let table = table.to_string();

        return table;
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
        value => {
            let mut text = value.to_abbreviated_string(config);
            if string_width(&text) > 50 {
                text = string_wrap(&text, 30, false);
            }

            TableValue::Cell(text)
        }
    }
}

fn build_vertical_map(record: Record, config: &Config) -> TableValue {
    let mut rows = Vec::with_capacity(record.len());
    for (key, value) in record {
        let val = convert_nu_value_to_table_value(value, config);
        let row = TableValue::Row(vec![TableValue::Cell(key), val]);
        rows.push(row);
    }

    let max_key_width = rows
        .iter()
        .map(|row| match row {
            TableValue::Row(list) => match &list[0] {
                TableValue::Cell(key) => string_width(key),
                _ => unreachable!(),
            },
            _ => unreachable!(),
        })
        .max()
        .unwrap_or(0);

    rows.iter_mut().for_each(|row| {
        match row {
            TableValue::Row(list) => match &mut list[0] {
                TableValue::Cell(key) => {
                    let width = string_width(key);
                    let rest = max_key_width - width;
                    key.extend(std::iter::repeat(' ').take(rest));
                }
                _ => unreachable!(),
            },
            _ => unreachable!(),
        };
    });

    TableValue::Column(rows)
}

fn build_vertical_array(vals: Vec<Value>, config: &Config) -> TableValue {
    let map = vals
        .into_iter()
        .map(|val| convert_nu_value_to_table_value(val, config))
        .collect::<Vec<_>>();

    TableValue::Column(map)
}

fn is_valid_record(vals: &[Value]) -> bool {
    let mut first_record: Option<&Record> = None;
    for val in vals {
        match val {
            Value::Record { val, .. } => {
                if let Some(known) = first_record {
                    let equal = known.columns().eq(val.columns());
                    if !equal {
                        return false;
                    }
                } else {
                    first_record = Some(val)
                };
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
    let mut list = vec![];

    let head = get_columns_in_record(&vals);
    let count_columns = head.len();
    for col in head {
        list.push(vec![TableValue::Cell(col)]);
    }

    for val in vals {
        match val {
            Value::Record { val, .. } => {
                for (i, (_key, val)) in val.into_owned().into_iter().take(count_columns).enumerate()
                {
                    let cell = convert_nu_value_to_table_value(val, config);
                    list[i].push(cell);
                }
            }
            _ => unreachable!(),
        }
    }

    let columns = list.into_iter().map(TableValue::Column).collect::<Vec<_>>();

    TableValue::Row(columns)
}

fn get_columns_in_record(vals: &[Value]) -> Vec<String> {
    match vals.iter().next() {
        Some(Value::Record { val, .. }) => val.columns().cloned().collect(),
        _ => vec![],
    }
}

struct SetRawStyle(RawStyle);

impl<R, D> TableOption<R, D, CompactMultilineConfig> for SetRawStyle {
    fn change(self, _: &mut R, cfg: &mut CompactMultilineConfig, _: &mut D) {
        let borders = self.0.get_borders();
        *cfg = cfg.set_borders(borders);
    }
}

struct SetBorderColor(StaticColor);

impl<R, D> TableOption<R, D, CompactMultilineConfig> for SetBorderColor {
    fn change(self, _: &mut R, cfg: &mut CompactMultilineConfig, _: &mut D) {
        let borders = Borders::filled(self.0);
        *cfg = cfg.set_borders_color(borders);
    }
}

struct SetAlignment(AlignmentHorizontal);

impl<R, D> TableOption<R, D, CompactMultilineConfig> for SetAlignment {
    fn change(self, _: &mut R, cfg: &mut CompactMultilineConfig, _: &mut D) {
        *cfg = cfg.set_alignment_horizontal(self.0);
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
