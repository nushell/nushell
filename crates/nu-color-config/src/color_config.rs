use crate::nu_style::{color_from_hex, color_string_to_nustyle, lookup_style};
use nu_ansi_term::{Color, Style};
use nu_protocol::{Config, Value};
use nu_table::{Alignment, TextStyle};
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

fn update_hashmap(key: &str, val: &str, hm: &mut HashMap<String, Style>) {
    // eprintln!("key: {}, val: {}", &key, &val);
    let color = lookup_ansi_color_style(val);
    if let Some(v) = hm.get_mut(key) {
        *v = color;
    } else {
        hm.insert(key.to_string(), color);
    }
}

pub fn get_color_config(config: &Config) -> HashMap<String, Style> {
    let config = config;

    // create the hashmap
    let mut hm: HashMap<String, Style> = HashMap::new();
    // set some defaults
    // hm.insert("primitive_line".to_string(), Color::White.normal());
    // hm.insert("primitive_pattern".to_string(), Color::White.normal());
    // hm.insert("primitive_path".to_string(), Color::White.normal());
    hm.insert("separator".to_string(), Color::White.normal());
    hm.insert(
        "leading_trailing_space_bg".to_string(),
        Style::default().on(Color::Rgb(128, 128, 128)),
    );
    hm.insert("header".to_string(), Color::Green.bold());
    hm.insert("empty".to_string(), Color::Blue.normal());
    hm.insert("bool".to_string(), Color::White.normal());
    hm.insert("int".to_string(), Color::White.normal());
    hm.insert("filesize".to_string(), Color::White.normal());
    hm.insert("duration".to_string(), Color::White.normal());
    hm.insert("date".to_string(), Color::White.normal());
    hm.insert("range".to_string(), Color::White.normal());
    hm.insert("float".to_string(), Color::White.normal());
    hm.insert("string".to_string(), Color::White.normal());
    hm.insert("nothing".to_string(), Color::White.normal());
    hm.insert("binary".to_string(), Color::White.normal());
    hm.insert("cellpath".to_string(), Color::White.normal());
    hm.insert("row_index".to_string(), Color::Green.bold());
    hm.insert("record".to_string(), Color::White.normal());
    hm.insert("list".to_string(), Color::White.normal());
    hm.insert("block".to_string(), Color::White.normal());
    hm.insert("hints".to_string(), Color::DarkGray.normal());

    for (key, value) in &config.color_config {
        let value = value
            .as_string()
            .expect("the only values for config color must be strings");
        update_hashmap(key, &value, &mut hm);

        // eprintln!(
        //     "config: {}:{}\t\t\thashmap: {}:{:?}",
        //     &key, &value, &key, &hm[key]
        // );
    }

    hm
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

// This function will assign a text style to a primitive, or really any string that's
// in the hashmap. The hashmap actually contains the style to be applied.
pub fn style_primitive(primitive: &str, color_hm: &HashMap<String, Style>) -> TextStyle {
    match primitive {
        "bool" => {
            let style = color_hm.get(primitive);
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_left(),
            }
        }

        "int" => {
            let style = color_hm.get(primitive);
            match style {
                Some(s) => TextStyle::with_style(Alignment::Right, *s),
                None => TextStyle::basic_right(),
            }
        }

        "filesize" => {
            let style = color_hm.get(primitive);
            match style {
                Some(s) => TextStyle::with_style(Alignment::Right, *s),
                None => TextStyle::basic_right(),
            }
        }

        "duration" => {
            let style = color_hm.get(primitive);
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_left(),
            }
        }

        "date" => {
            let style = color_hm.get(primitive);
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_left(),
            }
        }

        "range" => {
            let style = color_hm.get(primitive);
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_left(),
            }
        }

        "float" => {
            let style = color_hm.get(primitive);
            match style {
                Some(s) => TextStyle::with_style(Alignment::Right, *s),
                None => TextStyle::basic_right(),
            }
        }

        "string" => {
            let style = color_hm.get(primitive);
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_left(),
            }
        }

        "nothing" => {
            let style = color_hm.get(primitive);
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_left(),
            }
        }

        // not sure what to do with error
        // "error" => {}
        "binary" => {
            let style = color_hm.get(primitive);
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_left(),
            }
        }

        "cellpath" => {
            let style = color_hm.get(primitive);
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_left(),
            }
        }

        "row_index" => {
            let style = color_hm.get(primitive);
            match style {
                Some(s) => TextStyle::with_style(Alignment::Right, *s),
                None => TextStyle::new()
                    .alignment(Alignment::Right)
                    .fg(Color::Green)
                    .bold(Some(true)),
            }
        }

        "record" | "list" | "block" => {
            let style = color_hm.get(primitive);
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_left(),
            }
        }

        // types in nushell but not in engine-q
        // "Line" => {
        //     let style = color_hm.get("Primitive::Line");
        //     match style {
        //         Some(s) => TextStyle::with_style(Alignment::Left, *s),
        //         None => TextStyle::basic_left(),
        //     }
        // }
        // "GlobPattern" => {
        //     let style = color_hm.get("Primitive::GlobPattern");
        //     match style {
        //         Some(s) => TextStyle::with_style(Alignment::Left, *s),
        //         None => TextStyle::basic_left(),
        //     }
        // }
        // "FilePath" => {
        //     let style = color_hm.get("Primitive::FilePath");
        //     match style {
        //         Some(s) => TextStyle::with_style(Alignment::Left, *s),
        //         None => TextStyle::basic_left(),
        //     }
        // }
        // "BeginningOfStream" => {
        //     let style = color_hm.get("Primitive::BeginningOfStream");
        //     match style {
        //         Some(s) => TextStyle::with_style(Alignment::Left, *s),
        //         None => TextStyle::basic_left(),
        //     }
        // }
        // "EndOfStream" => {
        //     let style = color_hm.get("Primitive::EndOfStream");
        //     match style {
        //         Some(s) => TextStyle::with_style(Alignment::Left, *s),
        //         None => TextStyle::basic_left(),
        //     }
        // }
        _ => TextStyle::basic_left(),
    }
}

#[test]
fn test_hm() {
    use nu_ansi_term::{Color, Style};

    let mut hm: HashMap<String, Style> = HashMap::new();
    hm.insert("primitive_int".to_string(), Color::White.normal());
    hm.insert("primitive_decimal".to_string(), Color::White.normal());
    hm.insert("primitive_filesize".to_string(), Color::White.normal());
    hm.insert("primitive_string".to_string(), Color::White.normal());
    hm.insert("primitive_line".to_string(), Color::White.normal());
    hm.insert("primitive_columnpath".to_string(), Color::White.normal());
    hm.insert("primitive_pattern".to_string(), Color::White.normal());
    hm.insert("primitive_boolean".to_string(), Color::White.normal());
    hm.insert("primitive_date".to_string(), Color::White.normal());
    hm.insert("primitive_duration".to_string(), Color::White.normal());
    hm.insert("primitive_range".to_string(), Color::White.normal());
    hm.insert("primitive_path".to_string(), Color::White.normal());
    hm.insert("primitive_binary".to_string(), Color::White.normal());
    hm.insert("separator".to_string(), Color::White.normal());
    hm.insert("header_align".to_string(), Color::Green.bold());
    hm.insert("header".to_string(), Color::Green.bold());
    hm.insert("header_style".to_string(), Style::default());
    hm.insert("row_index".to_string(), Color::Green.bold());
    hm.insert(
        "leading_trailing_space_bg".to_string(),
        Style::default().on(Color::Rgb(128, 128, 128)),
    );

    update_hashmap("primitive_int", "green", &mut hm);

    assert_eq!(hm["primitive_int"], Color::Green.normal());
}
