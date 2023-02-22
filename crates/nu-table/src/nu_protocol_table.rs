use std::collections::HashMap;

use crate::{string_width, Alignments, TableTheme};
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
    inner: String,
}

impl NuTable {
    pub fn new(
        value: Value,
        collapse: bool,
        config: &Config,
        style_computer: &StyleComputer,
        theme: &TableTheme,
        with_footer: bool,
    ) -> Self {
        let mut table = tabled::Table::new([""]);
        load_theme(&mut table, style_computer, theme);
        let cfg = table.get_config().clone();

        let val = nu_protocol_value_to_json(value, config, with_footer);
        let table = build_table(val, cfg, collapse);

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
