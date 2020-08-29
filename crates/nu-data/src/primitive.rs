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

pub fn lookup_ansi_color_style(s: String) -> ansi_term::Style {
    match s.as_str() {
        "g" | "green" => ansi_term::Color::Green.normal(),
        "gb" | "green_bold" => ansi_term::Color::Green.bold(),
        "gu" | "green_underline" => ansi_term::Color::Green.underline(),
        "gi" | "green_italic" => ansi_term::Color::Green.italic(),
        "gd" | "green_dimmed" => ansi_term::Color::Green.dimmed(),
        "gr" | "green_reverse" => ansi_term::Color::Green.reverse(),
        "r" | "red" => ansi_term::Color::Red.normal(),
        "rb" | "red_bold" => ansi_term::Color::Red.bold(),
        "ru" | "red_underline" => ansi_term::Color::Red.underline(),
        "ri" | "red_italic" => ansi_term::Color::Red.italic(),
        "rd" | "red_dimmed" => ansi_term::Color::Red.dimmed(),
        "rr" | "red_reverse" => ansi_term::Color::Red.reverse(),
        "u" | "blue" => ansi_term::Color::Blue.normal(),
        "ub" | "blue_bold" => ansi_term::Color::Blue.bold(),
        "uu" | "blue_underline" => ansi_term::Color::Blue.underline(),
        "ui" | "blue_italic" => ansi_term::Color::Blue.italic(),
        "ud" | "blue_dimmed" => ansi_term::Color::Blue.dimmed(),
        "ur" | "blue_reverse" => ansi_term::Color::Blue.reverse(),
        "b" | "black" => ansi_term::Color::Black.normal(),
        "bb" | "black_bold" => ansi_term::Color::Black.bold(),
        "bu" | "black_underline" => ansi_term::Color::Black.underline(),
        "bi" | "black_italic" => ansi_term::Color::Black.italic(),
        "bd" | "black_dimmed" => ansi_term::Color::Black.dimmed(),
        "br" | "black_reverse" => ansi_term::Color::Black.reverse(),
        "y" | "yellow" => ansi_term::Color::Yellow.normal(),
        "yb" | "yellow_bold" => ansi_term::Color::Yellow.bold(),
        "yu" | "yellow_underline" => ansi_term::Color::Yellow.underline(),
        "yi" | "yellow_italic" => ansi_term::Color::Yellow.italic(),
        "yd" | "yellow_dimmed" => ansi_term::Color::Yellow.dimmed(),
        "yr" | "yellow_reverse" => ansi_term::Color::Yellow.reverse(),
        "p" | "purple" => ansi_term::Color::Purple.normal(),
        "pb" | "purple_bold" => ansi_term::Color::Purple.bold(),
        "pu" | "purple_underline" => ansi_term::Color::Purple.underline(),
        "pi" | "purple_italic" => ansi_term::Color::Purple.italic(),
        "pd" | "purple_dimmed" => ansi_term::Color::Purple.dimmed(),
        "pr" | "purple_reverse" => ansi_term::Color::Purple.reverse(),
        "c" | "cyan" => ansi_term::Color::Cyan.normal(),
        "cb" | "cyan_bold" => ansi_term::Color::Cyan.bold(),
        "cu" | "cyan_underline" => ansi_term::Color::Cyan.underline(),
        "ci" | "cyan_italic" => ansi_term::Color::Cyan.italic(),
        "cd" | "cyan_dimmed" => ansi_term::Color::Cyan.dimmed(),
        "cr" | "cyan_reverse" => ansi_term::Color::Cyan.reverse(),
        "w" | "white" => ansi_term::Color::White.normal(),
        "wb" | "white_bold" => ansi_term::Color::White.bold(),
        "wu" | "white_underline" => ansi_term::Color::White.underline(),
        "wi" | "white_italic" => ansi_term::Color::White.italic(),
        "wd" | "white_dimmed" => ansi_term::Color::White.dimmed(),
        "wr" | "white_reverse" => ansi_term::Color::White.reverse(),
        // "reset" => "\x1b[0m".to_owned(),
        _ => ansi_term::Color::White.normal(),
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
        "primitive_pattern" => "Primitive::Pattern".to_string(),
        "primitive_boolean" => "Primitive::Boolean".to_string(),
        "primitive_date" => "Primitive::Date".to_string(),
        "primitive_duration" => "Primitive::Duration".to_string(),
        "primitive_range" => "Primitive::Range".to_string(),
        "primitive_path" => "Primitive::Path".to_string(),
        "primitive_binary" => "Primitive::Binary".to_string(),
        "separator" => "separator".to_string(),
        "header_align" => "header_align".to_string(),
        "header_color" => "header_color".to_string(),
        "header_bold" => "header_bold".to_string(),
        "header_style" => "header_style".to_string(),
        _ => "Primitive::Nothing".to_string(),
    }
}

fn update_hashmap(key: &String, val: &Value, hm: &mut HashMap<String, ansi_term::Style>) {
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

pub fn get_color_config() -> HashMap<String, ansi_term::Style> {
    // create the hashmap
    let mut hm: HashMap<String, ansi_term::Style> = HashMap::new();
    // set some defaults
    hm.insert("primitive_int".to_string(), ansi_term::Color::White.normal());
    hm.insert("primitive_decimal".to_string(), ansi_term::Color::White.normal());
    hm.insert("primitive_filesize".to_string(), ansi_term::Color::White.normal());
    hm.insert("primitive_string".to_string(), ansi_term::Color::White.normal());
    hm.insert("primitive_line".to_string(), ansi_term::Color::White.normal());
    hm.insert("primitive_columnpath".to_string(), ansi_term::Color::White.normal());
    hm.insert("primitive_pattern".to_string(), ansi_term::Color::White.normal());
    hm.insert("primitive_boolean".to_string(), ansi_term::Color::White.normal());
    hm.insert("primitive_date".to_string(), ansi_term::Color::White.normal());
    hm.insert("primitive_duration".to_string(), ansi_term::Color::White.normal());
    hm.insert("primitive_range".to_string(), ansi_term::Color::White.normal());
    hm.insert("primitive_path".to_string(), ansi_term::Color::White.normal());
    hm.insert("primitive_binary".to_string(), ansi_term::Color::White.normal());
    hm.insert("separator".to_string(), ansi_term::Color::White.normal());
    hm.insert("header_align".to_string(), ansi_term::Color::White.normal());
    hm.insert("header_color".to_string(), ansi_term::Color::White.normal());
    hm.insert("header_bold".to_string(), ansi_term::Color::White.normal());
    hm.insert("header_style".to_string(), ansi_term::Style::default());

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
                    "separator" => {
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
                    _ => (),
                }
            }
        }
    }

    hm
}

// pub fn style_primitive(
//     primitive: &Primitive,
//     color_hm: &HashMap<String, ansi_term::Style>,
// ) -> TextStyle {
//     match primitive {
//         Primitive::Int(_) => {
//             let style = color_hm.get("Primitive::Int");
//             match style {
//                 Some(s) => TextStyle::with_style(Alignment::Right, *s),
//                 None => TextStyle::basic_right(),
//             }
//         }
//         Primitive::Decimal(_) => {
//             let style = color_hm.get("Primitive::Decimal");
//             match style {
//                 Some(s) => TextStyle::with_style(Alignment::Right, *s),
//                 None => TextStyle::basic_right(),
//             }
//         }
//         Primitive::Filesize(_) => {
//             let style = color_hm.get("Primitive::Filesize");
//             match style {
//                 Some(s) => TextStyle::with_style(Alignment::Right, *s),
//                 None => TextStyle::basic_right(),
//             }
//         }
//         Primitive::String(_) => {
//             let style = color_hm.get("Primitive::String");
//             match style {
//                 Some(s) => TextStyle::with_style(Alignment::Left, *s),
//                 None => TextStyle::basic_right(),
//             }
//         }
//         Primitive::Line(_) => {
//             let style = color_hm.get("Primitive::Line");
//             match style {
//                 Some(s) => TextStyle::with_style(Alignment::Left, *s),
//                 None => TextStyle::basic_right(),
//             }
//         }
//         Primitive::ColumnPath(_) => {
//             let style = color_hm.get("Primitive::ColumnPath");
//             match style {
//                 Some(s) => TextStyle::with_style(Alignment::Left, *s),
//                 None => TextStyle::basic_right(),
//             }
//         }
//         Primitive::Pattern(_) => {
//             let style = color_hm.get("Primitive::Pattern");
//             match style {
//                 Some(s) => TextStyle::with_style(Alignment::Left, *s),
//                 None => TextStyle::basic_right(),
//             }
//         }
//         Primitive::Boolean(_) => {
//             let style = color_hm.get("Primitive::Boolean");
//             match style {
//                 Some(s) => TextStyle::with_style(Alignment::Left, *s),
//                 None => TextStyle::basic_right(),
//             }
//         }
//         Primitive::Date(_) => {
//             let style = color_hm.get("Primitive::Date");
//             match style {
//                 Some(s) => TextStyle::with_style(Alignment::Left, *s),
//                 None => TextStyle::basic_right(),
//             }
//         }
//         Primitive::Duration(_) => {
//             let style = color_hm.get("Primitive::Duration");
//             match style {
//                 Some(s) => TextStyle::with_style(Alignment::Left, *s),
//                 None => TextStyle::basic_right(),
//             }
//         }
//         Primitive::Range(_) => {
//             let style = color_hm.get("Primitive::Range");
//             match style {
//                 Some(s) => TextStyle::with_style(Alignment::Left, *s),
//                 None => TextStyle::basic_right(),
//             }
//         }
//         Primitive::Path(_) => {
//             let style = color_hm.get("Primitive::Path");
//             match style {
//                 Some(s) => TextStyle::with_style(Alignment::Left, *s),
//                 None => TextStyle::basic_right(),
//             }
//         }
//         Primitive::Binary(_) => {
//             let style = color_hm.get("Primitive::Binary");
//             match style {
//                 Some(s) => TextStyle::with_style(Alignment::Left, *s),
//                 None => TextStyle::basic_right(),
//             }
//         }
//         Primitive::BeginningOfStream => {
//             let style = color_hm.get("Primitive::BeginningOfStream");
//             match style {
//                 Some(s) => TextStyle::with_style(Alignment::Left, *s),
//                 None => TextStyle::basic_right(),
//             }
//         }
//         Primitive::EndOfStream => {
//             let style = color_hm.get("Primitive::EndOfStream");
//             match style {
//                 Some(s) => TextStyle::with_style(Alignment::Left, *s),
//                 None => TextStyle::basic_right(),
//             }
//         }
//         Primitive::Nothing => {
//             let style = color_hm.get("Primitive::Nothing");
//             match style {
//                 Some(s) => TextStyle::with_style(Alignment::Left, *s),
//                 None => TextStyle::basic_right(),
//             }
//         }
//         // Primitive::Nothing => {
//         //     let style = color_hm.get("Primitive::Nothing");
//         //     match style {
//         //         Some(s) => TextStyle::with_style(Alignment::Left, *s),
//         //         None => TextStyle::basic_right(),
//         //     }
//         // }
//         // Primitive::Nothing => {
//         //     let style = color_hm.get("Primitive::Nothing");
//         //     match style {
//         //         Some(s) => TextStyle::with_style(Alignment::Left, *s),
//         //         None => TextStyle::basic_right(),
//         //     }
//         // }
//         // Primitive::Nothing => {
//         //     let style = color_hm.get("Primitive::Nothing");
//         //     match style {
//         //         Some(s) => TextStyle::with_style(Alignment::Left, *s),
//         //         None => TextStyle::basic_right(),
//         //     }
//         // }
//         // Primitive::Nothing => {
//         //     let style = color_hm.get("Primitive::Nothing");
//         //     match style {
//         //         Some(s) => TextStyle::with_style(Alignment::Left, *s),
//         //         None => TextStyle::basic_right(),
//         //     }
//         // }
//         // _ => {
//         //     println!("Prim=[{:?}]", &primitive);
//         //     TextStyle::basic()
//         // }
//     }
// }

pub fn style_primitive(
    primitive: &String,
    color_hm: &HashMap<String, ansi_term::Style>,
) -> TextStyle {
    // println!("{}", &primitive);
    match primitive.as_ref() {
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
                None => TextStyle::basic_right(),
            }
        }
        "Line" => {
            let style = color_hm.get("Primitive::Line");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_right(),
            }
        }
        "ColumnPath" => {
            let style = color_hm.get("Primitive::ColumnPath");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_right(),
            }
        }
        "Pattern" => {
            let style = color_hm.get("Primitive::Pattern");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_right(),
            }
        }
        "Boolean" => {
            let style = color_hm.get("Primitive::Boolean");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_right(),
            }
        }
        "Date" => {
            let style = color_hm.get("Primitive::Date");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_right(),
            }
        }
        "Duration" => {
            let style = color_hm.get("Primitive::Duration");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_right(),
            }
        }
        "Range" => {
            let style = color_hm.get("Primitive::Range");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_right(),
            }
        }
        "Path" => {
            let style = color_hm.get("Primitive::Path");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_right(),
            }
        }
        "Binary" => {
            let style = color_hm.get("Primitive::Binary");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_right(),
            }
        }
        "BeginningOfStream" => {
            let style = color_hm.get("Primitive::BeginningOfStream");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_right(),
            }
        }
        "EndOfStream" => {
            let style = color_hm.get("Primitive::EndOfStream");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_right(),
            }
        }
        "Nothing" => {
            let style = color_hm.get("Primitive::Nothing");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_right(),
            }
        }
        "separator" => {
            let style = color_hm.get("separator");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_right(),
            }
        }
        "header_align" => {
            let style = color_hm.get("header_align");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_right(),
            }
        }
        "header_color" => {
            let style = color_hm.get("header_color");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_right(),
            }
        }
        "header_bold" => {
            let style = color_hm.get("header_bold");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_right(),
            }
        }
        "header_style" => {
            let style = color_hm.get("header_style");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_right(),
            }
        }
        _ => {
            TextStyle::basic()
        }
    }
}
