use crate::nu_style::{color_from_hex, color_string_to_nustyle};
use nu_ansi_term::{Color, Style};
use nu_protocol::Config;
use nu_table::{Alignment, TextStyle};
use std::collections::HashMap;

pub fn lookup_ansi_color_style(s: &str) -> Style {
    if s.starts_with('#') {
        match color_from_hex(s) {
            Ok(c) => match c {
                Some(c) => c.normal(),
                None => Style::default(),
            },
            Err(_) => Style::default(),
        }
    } else if s.starts_with('{') {
        color_string_to_nustyle(s.to_string())
    } else {
        match s {
            "g" | "green" => Color::Green.normal(),
            "gb" | "green_bold" => Color::Green.bold(),
            "gu" | "green_underline" => Color::Green.underline(),
            "gi" | "green_italic" => Color::Green.italic(),
            "gd" | "green_dimmed" => Color::Green.dimmed(),
            "gr" | "green_reverse" => Color::Green.reverse(),
            "gbl" | "green_blink" => Color::Green.blink(),
            "gst" | "green_strike" => Color::Green.strikethrough(),

            "lg" | "light_green" => Color::LightGreen.normal(),
            "lgb" | "light_green_bold" => Color::LightGreen.bold(),
            "lgu" | "light_green_underline" => Color::LightGreen.underline(),
            "lgi" | "light_green_italic" => Color::LightGreen.italic(),
            "lgd" | "light_green_dimmed" => Color::LightGreen.dimmed(),
            "lgr" | "light_green_reverse" => Color::LightGreen.reverse(),
            "lgbl" | "light_green_blink" => Color::LightGreen.blink(),
            "lgst" | "light_green_strike" => Color::LightGreen.strikethrough(),

            "r" | "red" => Color::Red.normal(),
            "rb" | "red_bold" => Color::Red.bold(),
            "ru" | "red_underline" => Color::Red.underline(),
            "ri" | "red_italic" => Color::Red.italic(),
            "rd" | "red_dimmed" => Color::Red.dimmed(),
            "rr" | "red_reverse" => Color::Red.reverse(),
            "rbl" | "red_blink" => Color::Red.blink(),
            "rst" | "red_strike" => Color::Red.strikethrough(),

            "lr" | "light_red" => Color::LightRed.normal(),
            "lrb" | "light_red_bold" => Color::LightRed.bold(),
            "lru" | "light_red_underline" => Color::LightRed.underline(),
            "lri" | "light_red_italic" => Color::LightRed.italic(),
            "lrd" | "light_red_dimmed" => Color::LightRed.dimmed(),
            "lrr" | "light_red_reverse" => Color::LightRed.reverse(),
            "lrbl" | "light_red_blink" => Color::LightRed.blink(),
            "lrst" | "light_red_strike" => Color::LightRed.strikethrough(),

            "u" | "blue" => Color::Blue.normal(),
            "ub" | "blue_bold" => Color::Blue.bold(),
            "uu" | "blue_underline" => Color::Blue.underline(),
            "ui" | "blue_italic" => Color::Blue.italic(),
            "ud" | "blue_dimmed" => Color::Blue.dimmed(),
            "ur" | "blue_reverse" => Color::Blue.reverse(),
            "ubl" | "blue_blink" => Color::Blue.blink(),
            "ust" | "blue_strike" => Color::Blue.strikethrough(),

            "lu" | "light_blue" => Color::LightBlue.normal(),
            "lub" | "light_blue_bold" => Color::LightBlue.bold(),
            "luu" | "light_blue_underline" => Color::LightBlue.underline(),
            "lui" | "light_blue_italic" => Color::LightBlue.italic(),
            "lud" | "light_blue_dimmed" => Color::LightBlue.dimmed(),
            "lur" | "light_blue_reverse" => Color::LightBlue.reverse(),
            "lubl" | "light_blue_blink" => Color::LightBlue.blink(),
            "lust" | "light_blue_strike" => Color::LightBlue.strikethrough(),

            "b" | "black" => Color::Black.normal(),
            "bb" | "black_bold" => Color::Black.bold(),
            "bu" | "black_underline" => Color::Black.underline(),
            "bi" | "black_italic" => Color::Black.italic(),
            "bd" | "black_dimmed" => Color::Black.dimmed(),
            "br" | "black_reverse" => Color::Black.reverse(),
            "bbl" | "black_blink" => Color::Black.blink(),
            "bst" | "black_strike" => Color::Black.strikethrough(),

            "ligr" | "light_gray" => Color::LightGray.normal(),
            "ligrb" | "light_gray_bold" => Color::LightGray.bold(),
            "ligru" | "light_gray_underline" => Color::LightGray.underline(),
            "ligri" | "light_gray_italic" => Color::LightGray.italic(),
            "ligrd" | "light_gray_dimmed" => Color::LightGray.dimmed(),
            "ligrr" | "light_gray_reverse" => Color::LightGray.reverse(),
            "ligrbl" | "light_gray_blink" => Color::LightGray.blink(),
            "ligrst" | "light_gray_strike" => Color::LightGray.strikethrough(),

            "y" | "yellow" => Color::Yellow.normal(),
            "yb" | "yellow_bold" => Color::Yellow.bold(),
            "yu" | "yellow_underline" => Color::Yellow.underline(),
            "yi" | "yellow_italic" => Color::Yellow.italic(),
            "yd" | "yellow_dimmed" => Color::Yellow.dimmed(),
            "yr" | "yellow_reverse" => Color::Yellow.reverse(),
            "ybl" | "yellow_blink" => Color::Yellow.blink(),
            "yst" | "yellow_strike" => Color::Yellow.strikethrough(),

            "ly" | "light_yellow" => Color::LightYellow.normal(),
            "lyb" | "light_yellow_bold" => Color::LightYellow.bold(),
            "lyu" | "light_yellow_underline" => Color::LightYellow.underline(),
            "lyi" | "light_yellow_italic" => Color::LightYellow.italic(),
            "lyd" | "light_yellow_dimmed" => Color::LightYellow.dimmed(),
            "lyr" | "light_yellow_reverse" => Color::LightYellow.reverse(),
            "lybl" | "light_yellow_blink" => Color::LightYellow.blink(),
            "lyst" | "light_yellow_strike" => Color::LightYellow.strikethrough(),

            "p" | "purple" => Color::Purple.normal(),
            "pb" | "purple_bold" => Color::Purple.bold(),
            "pu" | "purple_underline" => Color::Purple.underline(),
            "pi" | "purple_italic" => Color::Purple.italic(),
            "pd" | "purple_dimmed" => Color::Purple.dimmed(),
            "pr" | "purple_reverse" => Color::Purple.reverse(),
            "pbl" | "purple_blink" => Color::Purple.blink(),
            "pst" | "purple_strike" => Color::Purple.strikethrough(),

            "lp" | "light_purple" => Color::LightPurple.normal(),
            "lpb" | "light_purple_bold" => Color::LightPurple.bold(),
            "lpu" | "light_purple_underline" => Color::LightPurple.underline(),
            "lpi" | "light_purple_italic" => Color::LightPurple.italic(),
            "lpd" | "light_purple_dimmed" => Color::LightPurple.dimmed(),
            "lpr" | "light_purple_reverse" => Color::LightPurple.reverse(),
            "lpbl" | "light_purple_blink" => Color::LightPurple.blink(),
            "lpst" | "light_purple_strike" => Color::LightPurple.strikethrough(),

            "c" | "cyan" => Color::Cyan.normal(),
            "cb" | "cyan_bold" => Color::Cyan.bold(),
            "cu" | "cyan_underline" => Color::Cyan.underline(),
            "ci" | "cyan_italic" => Color::Cyan.italic(),
            "cd" | "cyan_dimmed" => Color::Cyan.dimmed(),
            "cr" | "cyan_reverse" => Color::Cyan.reverse(),
            "cbl" | "cyan_blink" => Color::Cyan.blink(),
            "cst" | "cyan_strike" => Color::Cyan.strikethrough(),

            "lc" | "light_cyan" => Color::LightCyan.normal(),
            "lcb" | "light_cyan_bold" => Color::LightCyan.bold(),
            "lcu" | "light_cyan_underline" => Color::LightCyan.underline(),
            "lci" | "light_cyan_italic" => Color::LightCyan.italic(),
            "lcd" | "light_cyan_dimmed" => Color::LightCyan.dimmed(),
            "lcr" | "light_cyan_reverse" => Color::LightCyan.reverse(),
            "lcbl" | "light_cyan_blink" => Color::LightCyan.blink(),
            "lcst" | "light_cyan_strike" => Color::LightCyan.strikethrough(),

            "w" | "white" => Color::White.normal(),
            "wb" | "white_bold" => Color::White.bold(),
            "wu" | "white_underline" => Color::White.underline(),
            "wi" | "white_italic" => Color::White.italic(),
            "wd" | "white_dimmed" => Color::White.dimmed(),
            "wr" | "white_reverse" => Color::White.reverse(),
            "wbl" | "white_blink" => Color::White.blink(),
            "wst" | "white_strike" => Color::White.strikethrough(),

            "dgr" | "dark_gray" => Color::DarkGray.normal(),
            "dgrb" | "dark_gray_bold" => Color::DarkGray.bold(),
            "dgru" | "dark_gray_underline" => Color::DarkGray.underline(),
            "dgri" | "dark_gray_italic" => Color::DarkGray.italic(),
            "dgrd" | "dark_gray_dimmed" => Color::DarkGray.dimmed(),
            "dgrr" | "dark_gray_reverse" => Color::DarkGray.reverse(),
            "dgrbl" | "dark_gray_blink" => Color::DarkGray.blink(),
            "dgrst" | "dark_gray_strike" => Color::DarkGray.strikethrough(),

            "def" | "default" => Color::Default.normal(),
            "defb" | "default_bold" => Color::Default.bold(),
            "defu" | "default_underline" => Color::Default.underline(),
            "defi" | "default_italic" => Color::Default.italic(),
            "defd" | "default_dimmed" => Color::Default.dimmed(),
            "defr" | "default_reverse" => Color::Default.reverse(),

            _ => Color::White.normal(),
        }
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
