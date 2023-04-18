use std::collections::HashMap;

use nu_color_config::StyleComputer;
use nu_protocol::{Config, Span, Value};
use tabled::{
    grid::{
        color::{AnsiColor, StaticColor},
        config::{AlignmentHorizontal, Borders, CompactMultilineConfig},
        dimension::{DimensionPriority, PoolTableDimension},
    },
    settings::{style::RawStyle, Color, TableOption},
    tables::{PoolTable, TableValue},
};

use crate::{string_width, string_wrap, TableTheme};

/// UnstructuredTable has a recursive table representation of nu_protocol::Value.
///
/// It doesn't support alignment and a proper width control.
pub struct UnstructuredTable {
    inner: String,
}

impl UnstructuredTable {
    pub fn new(
        value: Value,
        config: &Config,
        style_computer: &StyleComputer,
        theme: &TableTheme,
    ) -> Self {
        let val = convert_nu_value_to_table_value(value, config);
        let table = build_table(val, style_computer, theme);

        Self { inner: table }
    }

    pub fn draw(&self, termwidth: usize) -> Option<String> {
        let table_width = string_width(&self.inner);
        if table_width > termwidth {
            None
        } else {
            Some(self.inner.clone())
        }
    }
}

fn build_table(val: TableValue, style_computer: &StyleComputer, theme: &TableTheme) -> String {
    let mut table = PoolTable::from(val);

    let mut theme = theme.get_theme_full();
    theme.set_horizontals(HashMap::default());

    table.with(SetRawStyle(theme));
    table.with(SetAlignemnt(AlignmentHorizontal::Left));
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
        // Mitigation of this is definetely on a todo list.

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
        Value::Record { cols, vals, .. } => build_vertical_map(cols, vals, config),
        Value::List { vals, .. } => {
            let rebuild_array_as_map = is_valid_record(&vals) && count_columns_in_record(&vals) > 0;
            if rebuild_array_as_map {
                build_map_from_record(vals, config)
            } else {
                build_vertical_array(vals, config)
            }
        }
        value => {
            let mut text = value.into_abbreviated_string(config);
            if string_width(&text) > 50 {
                text = string_wrap(&text, 30, false);
            }

            TableValue::Cell(text)
        }
    }
}

fn build_vertical_map(cols: Vec<String>, vals: Vec<Value>, config: &Config) -> TableValue {
    let mut rows = Vec::with_capacity(cols.len());
    for (key, value) in cols.into_iter().zip(vals) {
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
    let mut used_cols: Option<&[String]> = None;
    for val in vals {
        match val {
            Value::Record { cols, .. } => {
                let cols_are_not_equal =
                    used_cols.is_some() && !matches!(used_cols, Some(used) if cols == used);
                if cols_are_not_equal {
                    return false;
                }

                used_cols = Some(cols);
            }
            _ => return false,
        }
    }

    true
}

fn count_columns_in_record(vals: &[Value]) -> usize {
    match vals.iter().next() {
        Some(Value::Record { cols, .. }) => cols.len(),
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
            Value::Record { vals, .. } => {
                for (i, cell) in vals.into_iter().take(count_columns).enumerate() {
                    let cell = convert_nu_value_to_table_value(cell, config);
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
        Some(Value::Record { cols, .. }) => cols.clone(),
        _ => vec![],
    }
}

struct SetRawStyle(RawStyle);

impl<R, D> TableOption<R, D, CompactMultilineConfig> for SetRawStyle {
    fn change(&mut self, _: &mut R, cfg: &mut CompactMultilineConfig, _: &mut D) {
        let borders = self.0.get_borders();
        *cfg = cfg.set_borders(borders);
    }
}

struct SetBorderColor(StaticColor);

impl<R, D> TableOption<R, D, CompactMultilineConfig> for SetBorderColor {
    fn change(&mut self, _: &mut R, cfg: &mut CompactMultilineConfig, _: &mut D) {
        let borders = Borders::filled(self.0);
        *cfg = cfg.set_borders_color(borders);
    }
}

struct SetAlignemnt(AlignmentHorizontal);

impl<R, D> TableOption<R, D, CompactMultilineConfig> for SetAlignemnt {
    fn change(&mut self, _: &mut R, cfg: &mut CompactMultilineConfig, _: &mut D) {
        *cfg = cfg.set_alignment_horizontal(self.0);
    }
}
