use crate::nu_style::{color_from_hex, color_string_to_nustyle};
use nu_ansi_term::{Color, Style};
use nu_protocol::Config;
use nu_table::{Alignment, TextStyle};
use std::collections::HashMap;

//TODO: should this be implemented again?
// pub fn number(number: impl Into<Number>) -> Primitive {
//     let number = number.into();

//     match number {
//         Number::BigInt(int) => Primitive::BigInt(int),
//         Number::Int(int) => Primitive::Int(int),
//         Number::Decimal(decimal) => Primitive::Decimal(decimal),
//     }
// }

pub fn lookup_ansi_color_style(s: String) -> Style {
    if s.starts_with('#') {
        match color_from_hex(&s) {
            Ok(c) => match c {
                Some(c) => c.normal(),
                None => Style::default(),
            },
            Err(_) => Style::default(),
        }
    } else if s.starts_with('{') {
        color_string_to_nustyle(s)
    } else {
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
}

// TODO: i'm not sure how this ever worked but leaving it in case it's used elsewhere but not implemented yet
// pub fn string_to_lookup_value(str_prim: &str) -> String {
//     match str_prim {
//         "primitive_int" => "Primitive::Int".to_string(),
//         "primitive_decimal" => "Primitive::Decimal".to_string(),
//         "primitive_filesize" => "Primitive::Filesize".to_string(),
//         "primitive_string" => "Primitive::String".to_string(),
//         "primitive_line" => "Primitive::Line".to_string(),
//         "primitive_columnpath" => "Primitive::ColumnPath".to_string(),
//         "primitive_pattern" => "Primitive::GlobPattern".to_string(),
//         "primitive_boolean" => "Primitive::Boolean".to_string(),
//         "primitive_date" => "Primitive::Date".to_string(),
//         "primitive_duration" => "Primitive::Duration".to_string(),
//         "primitive_range" => "Primitive::Range".to_string(),
//         "primitive_path" => "Primitive::FilePath".to_string(),
//         "primitive_binary" => "Primitive::Binary".to_string(),
//         "separator_color" => "separator_color".to_string(),
//         "header_align" => "header_align".to_string(),
//         "header_color" => "header_color".to_string(),
//         "header_style" => "header_style".to_string(),
//         "index_color" => "index_color".to_string(),
//         "leading_trailing_space_bg" => "leading_trailing_space_bg".to_string(),
//         _ => "Primitive::Nothing".to_string(),
//     }
// }

fn update_hashmap(key: &str, val: &str, hm: &mut HashMap<String, Style>) {
    // eprintln!("key: {}, val: {}", &key, &val);
    let color = lookup_ansi_color_style(val.to_string());
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
    // hm.insert("primitive_int".to_string(), Color::White.normal());
    // hm.insert("primitive_decimal".to_string(), Color::White.normal());
    // hm.insert("primitive_filesize".to_string(), Color::White.normal());
    // hm.insert("primitive_string".to_string(), Color::White.normal());
    // hm.insert("primitive_line".to_string(), Color::White.normal());
    // hm.insert("primitive_columnpath".to_string(), Color::White.normal());
    // hm.insert("primitive_pattern".to_string(), Color::White.normal());
    // hm.insert("primitive_boolean".to_string(), Color::White.normal());
    // hm.insert("primitive_date".to_string(), Color::White.normal());
    // hm.insert("primitive_duration".to_string(), Color::White.normal());
    // hm.insert("primitive_range".to_string(), Color::White.normal());
    // hm.insert("primitive_path".to_string(), Color::White.normal());
    // hm.insert("primitive_binary".to_string(), Color::White.normal());
    // hm.insert("separator_color".to_string(), Color::White.normal());
    // hm.insert("header_align".to_string(), Color::Green.bold());
    // hm.insert("header_color".to_string(), Color::Green.bold());
    // hm.insert("header_style".to_string(), Style::default());
    // hm.insert("index_color".to_string(), Color::Green.bold());
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

    for (key, value) in &config.color_config {
        update_hashmap(key, value, &mut hm);

        // eprintln!(
        //     "config: {}:{}\t\t\thashmap: {}:{:?}",
        //     &key, &value, &key, &hm[key]
        // );
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

        "record" | "list" | "block" => {
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
        // "separator_color" => {
        //     let style = color_hm.get("separator");
        //     match style {
        //         Some(s) => TextStyle::with_style(Alignment::Left, *s),
        //         None => TextStyle::basic_left(),
        //     }
        // }
        // "header_align" => {
        //     let style = color_hm.get("header_align");
        //     match style {
        //         Some(s) => TextStyle::with_style(Alignment::Center, *s),
        //         None => TextStyle::default_header(),
        //     }
        // }
        // "header_color" => {
        //     let style = color_hm.get("header_color");
        //     match style {
        //         Some(s) => TextStyle::with_style(Alignment::Center, *s),
        //         None => TextStyle::default_header().bold(Some(true)),
        //     }
        // }
        // "header_style" => {
        //     let style = color_hm.get("header_style");
        //     match style {
        //         Some(s) => TextStyle::with_style(Alignment::Center, *s),
        //         None => TextStyle::default_header(),
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
