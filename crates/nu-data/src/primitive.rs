use nu_ansi_term::{Color, Style};
use nu_protocol::{hir::Number, Primitive, Value};
use nu_source::Tag;
use nu_table::{Alignment, TextStyle};
use std::collections::HashMap;

pub fn number(number: impl Into<Number>) -> Primitive {
    let number = number.into();

    match number {
        Number::Int(int) => Primitive::Int(int),
        Number::Decimal(decimal) => Primitive::Decimal(decimal),
    }
}

pub fn lookup_ansi_color_style(s: String) -> Style {
    match s.as_str() {
        "g" | "green" => Color::Green.normal(),
        "gb" | "green_bold" => Color::Green.bold(),
        "gu" | "green_underline" => Color::Green.underline(),
        "gi" | "green_italic" => Color::Green.italic(),
        "gd" | "green_dimmed" => Color::Green.dimmed(),
        "gr" | "green_reverse" => Color::Green.reverse(),
        "gbl" | "green_blink" => Color::Green.blink(),
        "gst" | "green_strike" => Color::Green.strikethrough(),
        "r" | "red" => Color::Red.normal(),
        "rb" | "red_bold" => Color::Red.bold(),
        "ru" | "red_underline" => Color::Red.underline(),
        "ri" | "red_italic" => Color::Red.italic(),
        "rd" | "red_dimmed" => Color::Red.dimmed(),
        "rr" | "red_reverse" => Color::Red.reverse(),
        "rbl" | "red_blink" => Color::Red.blink(),
        "rst" | "red_strike" => Color::Red.strikethrough(),
        "u" | "blue" => Color::Blue.normal(),
        "ub" | "blue_bold" => Color::Blue.bold(),
        "uu" | "blue_underline" => Color::Blue.underline(),
        "ui" | "blue_italic" => Color::Blue.italic(),
        "ud" | "blue_dimmed" => Color::Blue.dimmed(),
        "ur" | "blue_reverse" => Color::Blue.reverse(),
        "ubl" | "blue_blink" => Color::Blue.blink(),
        "ust" | "blue_strike" => Color::Blue.strikethrough(),
        "b" | "black" => Color::Black.normal(),
        "bb" | "black_bold" => Color::Black.bold(),
        "bu" | "black_underline" => Color::Black.underline(),
        "bi" | "black_italic" => Color::Black.italic(),
        "bd" | "black_dimmed" => Color::Black.dimmed(),
        "br" | "black_reverse" => Color::Black.reverse(),
        "bbl" | "black_blink" => Color::Black.blink(),
        "bst" | "black_strike" => Color::Black.strikethrough(),
        "y" | "yellow" => Color::Yellow.normal(),
        "yb" | "yellow_bold" => Color::Yellow.bold(),
        "yu" | "yellow_underline" => Color::Yellow.underline(),
        "yi" | "yellow_italic" => Color::Yellow.italic(),
        "yd" | "yellow_dimmed" => Color::Yellow.dimmed(),
        "yr" | "yellow_reverse" => Color::Yellow.reverse(),
        "ybl" | "yellow_blink" => Color::Yellow.blink(),
        "yst" | "yellow_strike" => Color::Yellow.strikethrough(),
        "p" | "purple" => Color::Purple.normal(),
        "pb" | "purple_bold" => Color::Purple.bold(),
        "pu" | "purple_underline" => Color::Purple.underline(),
        "pi" | "purple_italic" => Color::Purple.italic(),
        "pd" | "purple_dimmed" => Color::Purple.dimmed(),
        "pr" | "purple_reverse" => Color::Purple.reverse(),
        "pbl" | "purple_blink" => Color::Purple.blink(),
        "pst" | "purple_strike" => Color::Purple.strikethrough(),
        "c" | "cyan" => Color::Cyan.normal(),
        "cb" | "cyan_bold" => Color::Cyan.bold(),
        "cu" | "cyan_underline" => Color::Cyan.underline(),
        "ci" | "cyan_italic" => Color::Cyan.italic(),
        "cd" | "cyan_dimmed" => Color::Cyan.dimmed(),
        "cr" | "cyan_reverse" => Color::Cyan.reverse(),
        "cbl" | "cyan_blink" => Color::Cyan.blink(),
        "cst" | "cyan_strike" => Color::Cyan.strikethrough(),
        "w" | "white" => Color::White.normal(),
        "wb" | "white_bold" => Color::White.bold(),
        "wu" | "white_underline" => Color::White.underline(),
        "wi" | "white_italic" => Color::White.italic(),
        "wd" | "white_dimmed" => Color::White.dimmed(),
        "wr" | "white_reverse" => Color::White.reverse(),
        "wbl" | "white_blink" => Color::White.blink(),
        "wst" | "white_strike" => Color::White.strikethrough(),
        _ => Color::White.normal(),
    }
}

pub fn string_to_lookup_value(str_prim: &str) -> String {
    match str_prim {
        "primitive_int" => "Primitive::Int".to_string(),
        "primitive_decimal" => "Primitive::Decimal".to_string(),
        "primitive_filesize" => "Primitive::Filesize".to_string(),
        "primitive_string" => "Primitive::String".to_string(),
        "primitive_line" => "Primitive::Line".to_string(),
        "primitive_columnpath" => "Primitive::ColumnPath".to_string(),
        "primitive_pattern" => "Primitive::GlobPattern".to_string(),
        "primitive_boolean" => "Primitive::Boolean".to_string(),
        "primitive_date" => "Primitive::Date".to_string(),
        "primitive_duration" => "Primitive::Duration".to_string(),
        "primitive_range" => "Primitive::Range".to_string(),
        "primitive_path" => "Primitive::FilePath".to_string(),
        "primitive_binary" => "Primitive::Binary".to_string(),
        "separator_color" => "separator_color".to_string(),
        "header_align" => "header_align".to_string(),
        "header_color" => "header_color".to_string(),
        "header_bold" => "header_bold".to_string(),
        "header_style" => "header_style".to_string(),
        "index_color" => "index_color".to_string(),
        "leading_trailing_space_bg" => "leading_trailing_space_bg".to_string(),
        _ => "Primitive::Nothing".to_string(),
    }
}

fn update_hashmap(key: &str, val: &Value, hm: &mut HashMap<String, Style>) {
    if let Ok(var) = val.as_string() {
        let color = lookup_ansi_color_style(var);
        let prim = string_to_lookup_value(&key);
        if let Some(v) = hm.get_mut(&prim) {
            *v = color;
        } else {
            hm.insert(prim, color);
        }
    }
}

pub fn get_color_config() -> HashMap<String, Style> {
    // create the hashmap
    let mut hm: HashMap<String, Style> = HashMap::new();
    // set some defaults
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
    hm.insert("separator_color".to_string(), Color::White.normal());
    hm.insert("header_align".to_string(), Color::Green.bold());
    hm.insert("header_color".to_string(), Color::Green.bold());
    hm.insert("header_bold".to_string(), Color::Green.bold());
    hm.insert("header_style".to_string(), Style::default());
    hm.insert("index_color".to_string(), Color::Green.bold());
    hm.insert(
        "leading_trailing_space_bg".to_string(),
        Style::default().on(Color::RGB(128, 128, 128)),
    );

    // populate hashmap from config values
    if let Ok(config) = crate::config::config(Tag::unknown()) {
        if let Some(primitive_color_vars) = config.get("color_config") {
            for (key, value) in primitive_color_vars.row_entries() {
                match key.as_ref() {
                    "primitive_int" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "primitive_decimal" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "primitive_filesize" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "primitive_string" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "primitive_line" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "primitive_columnpath" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "primitive_pattern" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "primitive_boolean" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "primitive_date" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "primitive_duration" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "primitive_range" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "primitive_path" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "primitive_binary" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "separator_color" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "header_align" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "header_color" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "header_bold" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "header_style" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "index_color" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "leading_trailing_space_bg" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    _ => (),
                }
            }
        }
    }

    hm
}

// This function will assign a text style to a primitive, or really any string that's
// in the hashmap. The hashmap actually contains the style to be applied.
pub fn style_primitive(primitive: &str, color_hm: &HashMap<String, Style>) -> TextStyle {
    match primitive {
        "Int" => {
            let style = color_hm.get("Primitive::Int");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Right, *s),
                None => TextStyle::basic_right(),
            }
        }
        "Decimal" => {
            let style = color_hm.get("Primitive::Decimal");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Right, *s),
                None => TextStyle::basic_right(),
            }
        }
        "Filesize" => {
            let style = color_hm.get("Primitive::Filesize");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Right, *s),
                None => TextStyle::basic_right(),
            }
        }
        "String" => {
            let style = color_hm.get("Primitive::String");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_left(),
            }
        }
        "Line" => {
            let style = color_hm.get("Primitive::Line");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_left(),
            }
        }
        "ColumnPath" => {
            let style = color_hm.get("Primitive::ColumnPath");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_left(),
            }
        }
        "GlobPattern" => {
            let style = color_hm.get("Primitive::GlobPattern");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_left(),
            }
        }
        "Boolean" => {
            let style = color_hm.get("Primitive::Boolean");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_left(),
            }
        }
        "Date" => {
            let style = color_hm.get("Primitive::Date");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_left(),
            }
        }
        "Duration" => {
            let style = color_hm.get("Primitive::Duration");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_left(),
            }
        }
        "Range" => {
            let style = color_hm.get("Primitive::Range");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_left(),
            }
        }
        "FilePath" => {
            let style = color_hm.get("Primitive::FilePath");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_left(),
            }
        }
        "Binary" => {
            let style = color_hm.get("Primitive::Binary");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_left(),
            }
        }
        "BeginningOfStream" => {
            let style = color_hm.get("Primitive::BeginningOfStream");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_left(),
            }
        }
        "EndOfStream" => {
            let style = color_hm.get("Primitive::EndOfStream");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_left(),
            }
        }
        "Nothing" => {
            let style = color_hm.get("Primitive::Nothing");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_left(),
            }
        }
        "separator_color" => {
            let style = color_hm.get("separator");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_left(),
            }
        }
        "header_align" => {
            let style = color_hm.get("header_align");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Center, *s),
                None => TextStyle::default_header(),
            }
        }
        "header_color" => {
            let style = color_hm.get("header_color");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Center, *s),
                None => TextStyle::default_header(),
            }
        }
        "header_bold" => {
            let style = color_hm.get("header_bold");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Center, *s),
                None => TextStyle::default_header(),
            }
        }
        "header_style" => {
            let style = color_hm.get("header_style");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Center, *s),
                None => TextStyle::default_header(),
            }
        }
        "index_color" => {
            let style = color_hm.get("index_color");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Right, *s),
                None => TextStyle::new()
                    .alignment(Alignment::Right)
                    .fg(Color::Green)
                    .bold(Some(true)),
            }
        }
        _ => TextStyle::basic_center(),
    }
}
