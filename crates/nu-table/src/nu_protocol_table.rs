use nu_protocol::{Config, Span, Value};

pub(crate) fn nu_protocol_value_to_json(value: Value) -> serde_json::Value {
    match value {
        Value::Bool { val, .. } => serde_json::Value::Bool(val),
        Value::Int { val, .. } => serde_json::Value::Number(val.into()),
        Value::Float { val, .. } => serde_json::Value::String(val.to_string()),
        Value::Filesize { val, .. } => serde_json::Value::Number(val.into()),
        Value::Duration { val, .. } => serde_json::Value::Number(val.into()),
        Value::Date { val, .. } => serde_json::Value::String(val.to_string()),
        Value::Error { error } => serde_json::Value::String(error.to_string()),
        Value::Range { .. } => todo!(),
        Value::String { val, .. } => serde_json::Value::String(val),
        Value::Record { cols, vals, .. } => {
            let mut map = serde_json::Map::new();
            for (key, value) in cols.into_iter().zip(vals) {
                let val = nu_protocol_value_to_json(value);
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

                    let head = build_map(cols.iter().map(|s| Value::String {
                        val: s.to_owned(),
                        span: Span::new(0, 0),
                    }));

                    arr.push(serde_json::Value::Object(head));

                    for value in &vals {
                        let vals = value.as_record().unwrap().1;
                        let vals = build_map(vals.iter().cloned());

                        let mut map = serde_json::Map::new();
                        connect_maps(&mut map, serde_json::Value::Object(vals));

                        arr.push(serde_json::Value::Object(map));
                    }

                    return serde_json::Value::Array(arr);
                } else {
                    let mut map = vec![];
                    let cols = serde_json::Value::Array(vec![serde_json::Value::String(
                        cols[0].to_owned(),
                    )]);

                    map.push(cols);
                    for value in vals {
                        if let Value::Record { vals, .. } = value {
                            let val = nu_protocol_value_to_json(Value::List {
                                vals,
                                span: Span::new(0, 0),
                            }); // rebuild array as a map

                            map.push(val);
                        }
                    }

                    return serde_json::Value::Array(map);
                };
            }

            let mut map = Vec::new();
            for value in vals {
                let val = nu_protocol_value_to_json(value);
                map.push(val);
            }

            serde_json::Value::Array(map)
        }
        Value::Block { .. } => todo!(),
        Value::Nothing { .. } => todo!(),
        Value::Binary { .. } => todo!(),
        Value::CellPath { .. } => todo!(),
        Value::CustomValue { .. } => todo!(),
    }
}

fn build_map(
    values: impl Iterator<Item = Value> + DoubleEndedIterator,
) -> serde_json::Map<String, serde_json::Value> {
    let mut map = serde_json::Map::new();
    let mut last_val: Option<Value> = None;
    for val in values.rev() {
        if map.is_empty() {
            match last_val.take() {
                Some(prev_val) => {
                    let col = val.into_abbreviated_string(&Config::default());
                    let prev = nu_protocol_value_to_json(prev_val);
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
