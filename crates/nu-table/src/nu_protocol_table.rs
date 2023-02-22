use std::collections::HashMap;

use crate::{string_truncate, string_width, Alignments, TableTheme};
use nu_color_config::StyleComputer;
use nu_protocol::{Config, Span, Value};
use tabled::{
    color::Color,
    formatting::AlignmentStrategy,
    object::Segment,
    papergrid::{records::Records, GridConfig},
    Alignment, Modify,
};

use serde_json::Value as Json;

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
        let table_width = string_width(&table);

        if table_width > termwidth {
            // Doing a soffisticated width control would require some deep rooted changes.
            // (Which is might neessery to be done)
            //
            // Instead we peek biggest cells 1 by 1 and truncating them.

            loop {
                match get_biggest_value(&mut val) {
                    Some((value, width)) => {
                        if width == 0 {
                            return Self { inner: None };
                        }

                        let need_to_cut = width - 1;
                        __truncate_value(value, need_to_cut);

                        let table = build_table(val.clone(), cfg.clone(), collapse);
                        let table_width = string_width(&table);

                        if table_width <= termwidth {
                            return Self { inner: Some(table) };
                        }
                    }
                    None => return Self { inner: None },
                }
            }
        }

        Self { inner: Some(table) }
    }

    pub fn draw(&self) -> Option<String> {
        self.inner.clone()
    }
}

fn build_table(val: Json, cfg: GridConfig, collapse: bool) -> String {
    let mut table = json_to_table::json_to_table(&val);
    table.set_config(cfg);

    if collapse {
        table.collapse();
    }

    table.to_string()
}

fn nu_protocol_value_to_json(value: Value, config: &Config, with_footer: bool) -> Json {
    match value {
        Value::Record { cols, vals, .. } => {
            let mut map = serde_json::Map::new();
            for (key, value) in cols.into_iter().zip(vals) {
                let val = nu_protocol_value_to_json(value, config, false);
                map.insert(key, val);
            }

            Json::Object(map)
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

                    arr.push(Json::Object(head.clone()));

                    for value in &vals {
                        if let Ok((_, vals)) = value.as_record() {
                            let vals = build_map(vals.iter().cloned(), config);

                            let mut map = serde_json::Map::new();
                            connect_maps(&mut map, Json::Object(vals));

                            arr.push(Json::Object(map));
                        }
                    }

                    if with_footer {
                        arr.push(Json::Object(head));
                    }

                    return Json::Array(arr);
                } else {
                    let mut map = vec![];
                    let head = Json::Array(vec![Json::String(cols[0].to_owned())]);

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

                    return Json::Array(map);
                };
            }

            let mut map = Vec::new();
            for value in vals {
                let val = nu_protocol_value_to_json(value, config, false);
                map.push(val);
            }

            Json::Array(map)
        }
        val => Json::String(val.into_abbreviated_string(config)),
    }
}

fn build_map(
    values: impl Iterator<Item = Value> + DoubleEndedIterator,
    config: &Config,
) -> serde_json::Map<String, Json> {
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

            new_m.insert(col, Json::Object(map));
            map = new_m;
        }
    }

    map
}

fn connect_maps(map: &mut serde_json::Map<String, Json>, value: Json) {
    if let Json::Object(m) = value {
        for (key, value) in m {
            if value.is_object() {
                let mut new_m = serde_json::Map::new();
                connect_maps(&mut new_m, value);
                map.insert(key, Json::Object(new_m));
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
    let mut theme = theme.into_full().unwrap_or_else(|| theme.theme.clone());
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

fn __truncate_value(value: &mut Json, width: usize) {
    match value {
        Json::Null => *value = Json::String(string_truncate("null", width)),
        Json::Bool(b) => {
            let val = if *b { "true" } else { "false" };

            *value = Json::String(string_truncate(val, width));
        }
        Json::Number(n) => {
            let n = n.to_string();
            *value = Json::String(string_truncate(&n, width));
        }
        Json::String(s) => {
            *value = Json::String(string_truncate(s, width));
        }
        Json::Array(_) | Json::Object(_) => {
            unreachable!("must never happen")
        }
    }
}

fn get_biggest_value(value: &mut Json) -> Option<(&mut Json, usize)> {
    match value {
        Json::Null => Some((value, 4)),
        Json::Bool(_) => Some((value, 4)),
        Json::Number(n) => {
            let width = n.to_string().len();
            Some((value, width))
        }
        Json::String(s) => {
            let width = string_width(s);
            Some((value, width))
        }
        Json::Array(arr) => {
            if arr.is_empty() {
                return None;
            }

            let mut width = 0;
            let mut index = 0;
            for (i, value) in arr.iter_mut().enumerate() {
                if let Some((_, w)) = get_biggest_value(value) {
                    if w >= width {
                        index = i;
                        width = w;
                    }
                }
            }

            get_biggest_value(&mut arr[index])
        }
        Json::Object(map) => {
            if map.is_empty() {
                return None;
            }

            let mut width = 0;
            let mut index = String::new();
            for (key, mut value) in map.clone() {
                if let Some((_, w)) = get_biggest_value(&mut value) {
                    if w >= width {
                        index = key;
                        width = w;
                    }
                }
            }

            get_biggest_value(&mut map[&index])
        }
    }
}
