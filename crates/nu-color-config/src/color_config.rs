use crate::{
    nu_style::{color_from_hex, lookup_style},
    parse_nustyle, NuStyle,
};
use nu_ansi_term::Style;
use nu_protocol::Value;
use std::collections::HashMap;

pub fn lookup_ansi_color_style(s: &str) -> Style {
    if s.starts_with('#') {
        color_from_hex(s)
            .ok()
            .and_then(|c| c.map(|c| c.normal()))
            .unwrap_or_default()
    } else if s.starts_with('{') {
        color_string_to_nustyle(s.to_string())
    } else {
        lookup_style(s)
    }
}

// These two are used only for Explore's very limited color config
fn update_hashmap(key: &str, val: &str, hm: &mut HashMap<String, Style>) {
    // eprintln!("key: {}, val: {}", &key, &val);
    let color = lookup_ansi_color_style(val);
    if let Some(v) = hm.get_mut(key) {
        *v = color;
    } else {
        hm.insert(key.to_string(), color);
    }
}

pub fn get_color_map(colors: &HashMap<String, Value>) -> HashMap<String, Style> {
    let mut hm: HashMap<String, Style> = HashMap::new();

    for (key, value) in colors {
        if let Value::String { val, .. } = value {
            update_hashmap(key, val, &mut hm);
        }
    }

    hm
}

fn color_string_to_nustyle(color_string: String) -> Style {
    // eprintln!("color_string: {}", &color_string);
    if color_string.is_empty() {
        return Style::default();
    }

    let nu_style = match nu_json::from_str::<NuStyle>(&color_string) {
        Ok(s) => s,
        Err(_) => return Style::default(),
    };

    parse_nustyle(nu_style)
}
