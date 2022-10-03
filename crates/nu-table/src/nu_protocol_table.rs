use std::collections::HashMap;

use nu_protocol::{Config, Span, Value};
use tabled::{color::Color, papergrid::records::Records, Table};

use crate::{table::TrimStrategyModifier, TableTheme};

/// NuTable has a recursive table representation of nu_prorocol::Value.
///
/// It doesn't support alignement and a proper width controll.
pub struct NuTable {
    inner: tabled::Table,
}

impl NuTable {
    pub fn new(
        value: Value,
        collapse: bool,
        termwidth: usize,
        config: &Config,
        color_hm: &HashMap<String, nu_ansi_term::Style>,
        theme: &TableTheme,
        with_footer: bool,
    ) -> Self {
        let mut table = tabled::Table::new([""]);
        load_theme(&mut table, color_hm, theme);
        let cfg = table.get_config().clone();

        let val = nu_protocol_value_to_json(value, config, with_footer);
        let mut table = json_to_table::json_to_table(&val);
        table.set_config(cfg);

        if collapse {
            table.collapse();
        }

        let mut table: Table<_> = table.into();
        table.with(TrimStrategyModifier::new(termwidth, &config.trim_strategy));

        Self { inner: table }
    }

    pub fn draw(&self) -> Option<String> {
        Some(self.inner.to_string())
    }
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
                    Value::Record { cols, .. } => match &used_cols {
                        Some(_cols) => {
                            if _cols != cols {
                                used_cols = None;
                                break;
                            }
                        }
                        None => used_cols = Some(cols),
                    },
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

    // if last_val.is_some() && map.is_empty() {
    //     let val = nu_protocol_value_to_json(last_val.unwrap());
    //     return serde_json::Value::Array(vec![val]);
    // }

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

fn load_theme<R>(
    table: &mut tabled::Table<R>,
    color_hm: &HashMap<String, nu_ansi_term::Style>,
    theme: &TableTheme,
) where
    R: Records,
{
    let mut theme = theme.theme.clone();
    theme.set_horizontals(HashMap::default());

    table.with(theme);

    if let Some(color) = color_hm.get("separator") {
        let color = color.paint(" ").to_string();
        if let Ok(color) = Color::try_from(color) {
            table.with(color);
        }
    }
}
