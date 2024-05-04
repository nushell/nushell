use crate::{
    nu_style::{color_from_hex, lookup_style},
    parse_nustyle, NuStyle,
};
use nu_ansi_term::Style;
use nu_protocol::{Record, Value};
use std::collections::HashMap;

pub fn lookup_ansi_color_style(s: &str) -> Style {
    if s.starts_with('#') {
        color_from_hex(s)
            .ok()
            .and_then(|c| c.map(|c| c.normal()))
            .unwrap_or_default()
    } else if s.starts_with('{') {
        color_string_to_nustyle(s)
    } else {
        lookup_style(s)
    }
}

pub fn get_color_map(colors: &HashMap<String, Value>) -> HashMap<String, Style> {
    let mut hm: HashMap<String, Style> = HashMap::new();

    for (key, value) in colors {
        parse_map_entry(&mut hm, key, value);
    }

    hm
}

fn parse_map_entry(hm: &mut HashMap<String, Style>, key: &str, value: &Value) {
    let value = match value {
        Value::String { val, .. } => Some(lookup_ansi_color_style(val)),
        Value::Record { val, .. } => get_style_from_value(val).map(parse_nustyle),
        _ => None,
    };
    if let Some(value) = value {
        hm.entry(key.to_owned()).or_insert(value);
    }
}

fn get_style_from_value(record: &Record) -> Option<NuStyle> {
    let mut was_set = false;
    let mut style = NuStyle::from(Style::default());
    for (col, val) in record {
        match col.as_str() {
            "bg" => {
                if let Value::String { val, .. } = val {
                    style.bg = Some(val.clone());
                    was_set = true;
                }
            }
            "fg" => {
                if let Value::String { val, .. } = val {
                    style.fg = Some(val.clone());
                    was_set = true;
                }
            }
            "attr" => {
                if let Value::String { val, .. } = val {
                    style.attr = Some(val.clone());
                    was_set = true;
                }
            }
            _ => (),
        }
    }

    if was_set {
        Some(style)
    } else {
        None
    }
}

fn color_string_to_nustyle(color_string: &str) -> Style {
    // eprintln!("color_string: {}", &color_string);
    if color_string.is_empty() {
        return Style::default();
    }

    let nu_style = match nu_json::from_str::<NuStyle>(color_string) {
        Ok(s) => s,
        Err(_) => return Style::default(),
    };

    parse_nustyle(nu_style)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_ansi_term::{Color, Style};
    use nu_protocol::{record, Span, Value};

    #[test]
    fn test_color_string_to_nustyle_empty_string() {
        let color_string = String::new();
        let style = color_string_to_nustyle(&color_string);
        assert_eq!(style, Style::default());
    }

    #[test]
    fn test_color_string_to_nustyle_valid_string() {
        let color_string = r#"{"fg": "black", "bg": "white", "attr": "b"}"#;
        let style = color_string_to_nustyle(color_string);
        assert_eq!(style.foreground, Some(Color::Black));
        assert_eq!(style.background, Some(Color::White));
        assert!(style.is_bold);
    }

    #[test]
    fn test_color_string_to_nustyle_invalid_string() {
        let color_string = "invalid string";
        let style = color_string_to_nustyle(color_string);
        assert_eq!(style, Style::default());
    }

    #[test]
    fn test_get_style_from_value() {
        // Test case 1: all values are valid
        let record = record! {
            "bg" =>   Value::test_string("red"),
            "fg" =>   Value::test_string("blue"),
            "attr" => Value::test_string("bold"),
        };
        let expected_style = NuStyle {
            bg: Some("red".to_string()),
            fg: Some("blue".to_string()),
            attr: Some("bold".to_string()),
        };
        assert_eq!(get_style_from_value(&record), Some(expected_style));

        // Test case 2: no values are valid
        let record = record! {
            "invalid" => Value::nothing(Span::test_data()),
        };
        assert_eq!(get_style_from_value(&record), None);

        // Test case 3: some values are valid
        let record = record! {
            "bg" =>      Value::test_string("green"),
            "invalid" => Value::nothing(Span::unknown()),
        };
        let expected_style = NuStyle {
            bg: Some("green".to_string()),
            fg: None,
            attr: None,
        };
        assert_eq!(get_style_from_value(&record), Some(expected_style));
    }

    #[test]
    fn test_parse_map_entry() {
        let mut hm = HashMap::new();
        let key = "test_key".to_owned();
        let value = Value::string("red", Span::unknown());
        parse_map_entry(&mut hm, &key, &value);
        assert_eq!(hm.get(&key), Some(&lookup_ansi_color_style("red")));
    }
}
