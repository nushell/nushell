use nu_protocol::{hir::Number, Primitive};
use nu_source::Tag;
use nu_table::{Alignment, TextStyle};
use std::collections::HashMap;
// use nu_errors::{ProximateShellError, ShellDiagnostic, ShellError};
// use indexmap::IndexMap;

pub fn number(number: impl Into<Number>) -> Primitive {
    let number = number.into();

    match number {
        Number::Int(int) => Primitive::Int(int),
        Number::Decimal(decimal) => Primitive::Decimal(decimal),
    }
}

pub fn lookup_ansi_color(s: String) -> ansi_term::Style {
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

pub fn text_primitive_to_primitive(str_prim: &str) -> String {
    match str_prim {
        "prim_int" => "Primitive::Int".to_string(),
        "prim_decimal" => "Primitive::Decimal".to_string(),
        "prim_filesize" => "Primitive::Filesize".to_string(),
        "prim_string" => "Primitive::String".to_string(),
        "prim_line" => "Primitive::Line".to_string(),
        "prim_columnpath" => "Primitive::ColumnPath".to_string(),
        "prim_pattern" => "Primitive::Pattern".to_string(),
        "prim_boolean" => "Primitive::Boolean".to_string(),
        "prim_date" => "Primitive::Date".to_string(),
        "prim_duration" => "Primitive::Duration".to_string(),
        "prim_range" => "Primitive::Range".to_string(),
        "prim_path" => "Primitive::Path".to_string(),
        "prim_binary" => "Primitive::Binary".to_string(),
        _ => "Primitive::Nothing".to_string(),
    }
}

pub fn get_primitive_color_config() -> HashMap<String, ansi_term::Style> {
    let mut hm: HashMap<String, ansi_term::Style> = HashMap::new();
    // let mut hm: IndexMap<String, ansi_term::Style> = IndexMap::new();
    // let config = match crate::config::config(Tag::unknown()) {
    //     Ok(config) => config,
    //     Err(e) => {
    //         eprintln!("Config could not be loaded.");
    //         if let ShellError {
    //             error: ProximateShellError::Diagnostic(ShellDiagnostic { diagnostic }),
    //             ..
    //         } = e
    //         {
    //             eprintln!("{}", diagnostic.message);
    //         }
    //         IndexMap::new()
    //     }
    // };

    // Set default colors
    if let Ok(config) = crate::config::config(Tag::unknown()) {
        if let Some(primitive_color_vars) = config.get("primitive_colors") {
            for (idx, value) in primitive_color_vars.row_entries() {
                match idx.as_ref() {
                    "prim_int" => {
                        if let Ok(var) = value.as_string() {
                            let color = lookup_ansi_color(var);
                            let prim = text_primitive_to_primitive(&idx);
                            hm.insert(prim, color);
                        }
                    }
                    "prim_decimal" => {
                        if let Ok(var) = value.as_string() {
                            let color = lookup_ansi_color(var);
                            let prim = text_primitive_to_primitive(&idx);
                            hm.insert(prim, color);
                        }
                    }
                    "prim_filesize" => {
                        if let Ok(var) = value.as_string() {
                            let color = lookup_ansi_color(var);
                            let prim = text_primitive_to_primitive(&idx);
                            hm.insert(prim, color);
                        }
                    }
                    "prim_string" => {
                        if let Ok(var) = value.as_string() {
                            let color = lookup_ansi_color(var);
                            let prim = text_primitive_to_primitive(&idx);
                            hm.insert(prim, color);
                        }
                    }
                    "prim_line" => {
                        if let Ok(var) = value.as_string() {
                            let color = lookup_ansi_color(var);
                            let prim = text_primitive_to_primitive(&idx);
                            hm.insert(prim, color);
                        }
                    }
                    "prim_columnpath" => {
                        if let Ok(var) = value.as_string() {
                            let color = lookup_ansi_color(var);
                            let prim = text_primitive_to_primitive(&idx);
                            hm.insert(prim, color);
                        }
                    }
                    "prim_pattern" => {
                        if let Ok(var) = value.as_string() {
                            let color = lookup_ansi_color(var);
                            let prim = text_primitive_to_primitive(&idx);
                            hm.insert(prim, color);
                        }
                    }
                    "prim_boolean" => {
                        if let Ok(var) = value.as_string() {
                            let color = lookup_ansi_color(var);
                            let prim = text_primitive_to_primitive(&idx);
                            hm.insert(prim, color);
                        }
                    }
                    "prim_date" => {
                        if let Ok(var) = value.as_string() {
                            let color = lookup_ansi_color(var);
                            let prim = text_primitive_to_primitive(&idx);
                            hm.insert(prim, color);
                        }
                    }
                    "prim_duration" => {
                        if let Ok(var) = value.as_string() {
                            let color = lookup_ansi_color(var);
                            let prim = text_primitive_to_primitive(&idx);
                            hm.insert(prim, color);
                        }
                    }
                    "prim_range" => {
                        if let Ok(var) = value.as_string() {
                            let color = lookup_ansi_color(var);
                            let prim = text_primitive_to_primitive(&idx);
                            hm.insert(prim, color);
                        }
                    }
                    "prim_path" => {
                        if let Ok(var) = value.as_string() {
                            let color = lookup_ansi_color(var);
                            let prim = text_primitive_to_primitive(&idx);
                            hm.insert(prim, color);
                        }
                    }
                    "prim_binary" => {
                        if let Ok(var) = value.as_string() {
                            let color = lookup_ansi_color(var);
                            let prim = text_primitive_to_primitive(&idx);
                            hm.insert(prim, color);
                        }
                    }
                    _ => (),
                }
            }
        }
    }

    hm
}

pub fn style_primitive(
    primitive: &Primitive,
    color_hm: &HashMap<String, ansi_term::Style>,
) -> TextStyle {
    // let hm = get_primitive_color_config();
    // println!("HashMap=[{:?}", hm);
    // match primitive {
    //     Primitive::Int(_) | Primitive::Filesize(_) | Primitive::Decimal(_) => {
    //         //TextStyle::basic_right()
    //         TextStyle::with_attributes(false, Alignment::Right, ansi_term::Color::Yellow)
    //     }
    //     Primitive::Date(_) => {
    //         TextStyle::with_attributes(false, Alignment::Left, ansi_term::Color::RGB(255,165,0))
    //     }
    //     //_ => TextStyle::basic(),
    //     _ => TextStyle::with_attributes(false, Alignment::Left, ansi_term::Color::Purple)
    // }

    match primitive {
        Primitive::Int(_) => {
            let style = color_hm.get("Primitive::Int");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Right, *s),
                None => TextStyle::basic_right(),
            }
        }
        Primitive::Decimal(_) => {
            let style = color_hm.get("Primitive::Decimal");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Right, *s),
                None => TextStyle::basic_right(),
            }
        }
        Primitive::Filesize(_) => {
            let style = color_hm.get("Primitive::Filesize");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Right, *s),
                None => TextStyle::basic_right(),
            }
        }
        Primitive::String(_) => {
            let style = color_hm.get("Primitive::String");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_right(),
            }
        }
        Primitive::Line(_) => {
            let style = color_hm.get("Primitive::Line");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_right(),
            }
        }
        Primitive::ColumnPath(_) => {
            let style = color_hm.get("Primitive::ColumnPath");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_right(),
            }
        }
        Primitive::Pattern(_) => {
            let style = color_hm.get("Primitive::Pattern");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_right(),
            }
        }
        Primitive::Boolean(_) => {
            let style = color_hm.get("Primitive::Boolean");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_right(),
            }
        }
        Primitive::Date(_) => {
            let style = color_hm.get("Primitive::Date");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_right(),
            }
        }
        Primitive::Duration(_) => {
            let style = color_hm.get("Primitive::Duration");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_right(),
            }
        }
        Primitive::Range(_) => {
            let style = color_hm.get("Primitive::Range");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_right(),
            }
        }
        Primitive::Path(_) => {
            let style = color_hm.get("Primitive::Path");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_right(),
            }
        }
        Primitive::Binary(_) => {
            let style = color_hm.get("Primitive::Binary");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_right(),
            }
        }
        Primitive::BeginningOfStream => {
            let style = color_hm.get("Primitive::BeginningOfStream");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_right(),
            }
        }
        Primitive::EndOfStream => {
            let style = color_hm.get("Primitive::EndOfStream");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_right(),
            }
        }
        Primitive::Nothing => {
            let style = color_hm.get("Primitive::Nothing");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_right(),
            }
        }
    }
}
