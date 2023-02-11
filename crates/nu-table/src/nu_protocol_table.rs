use std::collections::HashMap;

use crate::{
    string_truncate, string_width, string_wrap, Alignments, TableTheme,
};
use nu_color_config::StyleComputer;
use nu_protocol::{Config, Span, TrimStrategy, Value};
use tabled::{
    color::Color,
    formatting::AlignmentStrategy,
    object::Segment,
    papergrid::{records::Records, GridConfig},
    Alignment, Modify,
};

/// NuTable has a recursive table representation of nu_protocol::Value.
///
/// It doesn't support alignment and a proper width control.
pub struct NuTable {
    inner: Option<String>,
}

impl NuTable {
    pub fn new(
        value: Value,
        collapse: bool,
        termwidth: usize,
        config: &Config,
        style_computer: &StyleComputer,
        theme: &TableTheme,
        with_footer: bool,
    ) -> Self {
        let mut table = tabled::Table::new([""]);
        load_theme(&mut table, style_computer, theme);
        let cfg = table.get_config().clone();

        let mut val = nu_protocol_value_to_json(value, config, with_footer);
        let table = build_table(val.clone(), cfg.clone(), collapse);

        if string_width(&table) > termwidth {
            // Doing a soffisticated width control would require some deep rooted changes.
            // (Which is might neessery to be done)
            //
            // Instead we try 3 truncations and if we don't succssed if consider it's not possible to
            // fit in.

            let mut width = termwidth;
            for _ in 0..5 {
                width /= 2;

                truncate_values_to(&mut val, &config.trim_strategy, width);
                let table = build_table(val.clone(), cfg.clone(), collapse);

                if string_width(&table) <= termwidth {
                    return Self { inner: Some(table) };
                }
            }

            return Self { inner: None };
        }

        Self { inner: Some(table) }
    }

    pub fn draw(&self) -> Option<String> {
        self.inner.clone()
    }
}

fn build_table(val: serde_json::Value, cfg: GridConfig, collapse: bool) -> String {
    let mut table = json_to_table::json_to_table(&val);
    table.set_config(cfg);

    if collapse {
        table.collapse();
    }

    table.to_string()
}

fn nu_protocol_value_to_json(
    value: Value,
    config: &Config,
    with_footer: bool,
) -> serde_json::Value {
    match value {
        Value::Record { cols, vals, .. } => {
            let mut map = serde_json::Map::new();
            for (key, value) in cols.into_iter().zip(vals) {
                let val = nu_protocol_value_to_json(value, config, false);
                map.insert(key, val);
            }

            serde_json::Value::Object(map)
        }
        Value::List { vals, .. } => {
            let mut used_cols: Option<&[String]> = None;
            for val in &vals {
                match val {
                    Value::Record { cols, .. } => {
                        if let Some(_cols) = &used_cols {
                            if _cols != cols {
                                used_cols = None;
                                break;
                            }
                        } else {
                            used_cols = Some(cols)
                        }
                    }
                    _ => {
                        used_cols = None;
                        break;
                    }
                }
            }

            if let Some(cols) = used_cols {
                // rebuild array as a map
                if cols.len() > 1 {
                    let mut arr = vec![];

                    let head = cols.iter().map(|s| Value::String {
                        val: s.to_owned(),
                        span: Span::new(0, 0),
                    });
                    let head = build_map(head, config);

                    arr.push(serde_json::Value::Object(head.clone()));

                    for value in &vals {
                        if let Ok((_, vals)) = value.as_record() {
                            let vals = build_map(vals.iter().cloned(), config);

                            let mut map = serde_json::Map::new();
                            connect_maps(&mut map, serde_json::Value::Object(vals));

                            arr.push(serde_json::Value::Object(map));
                        }
                    }

                    if with_footer {
                        arr.push(serde_json::Value::Object(head));
                    }

                    return serde_json::Value::Array(arr);
                } else {
                    let mut map = vec![];
                    let head = serde_json::Value::Array(vec![serde_json::Value::String(
                        cols[0].to_owned(),
                    )]);

                    map.push(head.clone());
                    for value in vals {
                        if let Value::Record { vals, .. } = value {
                            let list = Value::List {
                                vals,
                                span: Span::new(0, 0),
                            };
                            let val = nu_protocol_value_to_json(list, config, false); // rebuild array as a map

                            map.push(val);
                        }
                    }

                    if with_footer {
                        map.push(head);
                    }

                    return serde_json::Value::Array(map);
                };
            }

            let mut map = Vec::new();
            for value in vals {
                let val = nu_protocol_value_to_json(value, config, false);
                map.push(val);
            }

            serde_json::Value::Array(map)
        }
        val => serde_json::Value::String(val.into_abbreviated_string(config)),
    }
}

fn build_map(
    values: impl Iterator<Item = Value> + DoubleEndedIterator,
    config: &Config,
) -> serde_json::Map<String, serde_json::Value> {
    let mut map = serde_json::Map::new();
    let mut last_val: Option<Value> = None;
    for val in values.rev() {
        if map.is_empty() {
            match last_val.take() {
                Some(prev_val) => {
                    let col = val.into_abbreviated_string(&Config::default());
                    let prev = nu_protocol_value_to_json(prev_val, config, false);
                    map.insert(col, prev);
                }
                None => {
                    last_val = Some(val);
                }
            }
        } else {
            let mut new_m = serde_json::Map::new();
            let col = val.into_abbreviated_string(&Config::default());

            new_m.insert(col, serde_json::Value::Object(map));
            map = new_m;
        }
    }

    map
}

fn connect_maps(map: &mut serde_json::Map<String, serde_json::Value>, value: serde_json::Value) {
    if let serde_json::Value::Object(m) = value {
        for (key, value) in m {
            if value.is_object() {
                let mut new_m = serde_json::Map::new();
                connect_maps(&mut new_m, value);
                map.insert(key, serde_json::Value::Object(new_m));
            } else {
                map.insert(key, value);
            }
        }
    }
}

//
fn load_theme<R>(table: &mut tabled::Table<R>, style_computer: &StyleComputer, theme: &TableTheme)
where
    R: Records,
{
    let mut theme = theme.theme.clone();
    theme.set_horizontals(HashMap::default());

    table.with(theme);

    // color_config closures for "separator" are just given a null.
    let color = style_computer.compute("separator", &Value::nothing(Span::unknown()));
    let color = color.paint(" ").to_string();
    if let Ok(color) = Color::try_from(color) {
        table.with(color);
    }

    table.with(
        Modify::new(Segment::all())
            .with(Alignment::Horizontal(Alignments::default().data))
            .with(AlignmentStrategy::PerLine),
    );
}

fn truncate_values_to(value: &mut serde_json::Value, strategy: &TrimStrategy, width: usize) {
    _truncate_value(value, strategy, width)
}

fn _truncate_value(value: &mut serde_json::Value, strategy: &TrimStrategy, width: usize) {
    match value {
        serde_json::Value::Null => {}
        serde_json::Value::Bool(_) => {}
        serde_json::Value::Number(n) => {
            let n = n.to_string();
            if n.len() > width {
                let s = truncate_strategy(&n, strategy, width);
                *value = serde_json::Value::String(s);
            }
        }
        serde_json::Value::String(s) => {
            if string_width(s) > width {
                *s = truncate_strategy(s, strategy, width);
            }
        }
        serde_json::Value::Array(arr) => {
            for value in arr {
                _truncate_value(value, strategy, width);
            }
        }
        serde_json::Value::Object(vals) => {
            let mut map = serde_json::Map::with_capacity(vals.len());

            for (key, val) in vals.iter() {
                let k = if string_width(key) > width {
                    truncate_strategy(key, strategy, width)
                } else {
                    key.clone()
                };

                let mut val = val.clone();
                _truncate_value(&mut val, strategy, width);

                map.insert(k, val);
            }

            *vals = map; 
        }
    }
}

fn truncate_strategy(val: &str, strategy: &TrimStrategy, width: usize) -> String {
    match strategy {
        TrimStrategy::Wrap { try_to_keep_words } => string_wrap(val, width, *try_to_keep_words),
        TrimStrategy::Truncate { .. } => string_truncate(val, width),
    }
}
