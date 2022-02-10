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
            "gb" | "green-bold" => Color::Green.bold(),
            "gu" | "green-underline" => Color::Green.underline(),
            "gi" | "green-italic" => Color::Green.italic(),
            "gd" | "green-dimmed" => Color::Green.dimmed(),
            "gr" | "green-reverse" => Color::Green.reverse(),
            "gbl" | "green-blink" => Color::Green.blink(),
            "gst" | "green-strike" => Color::Green.strikethrough(),

            "lg" | "light-green" => Color::LightGreen.normal(),
            "lgb" | "light-green-bold" => Color::LightGreen.bold(),
            "lgu" | "light-green-underline" => Color::LightGreen.underline(),
            "lgi" | "light-green-italic" => Color::LightGreen.italic(),
            "lgd" | "light-green-dimmed" => Color::LightGreen.dimmed(),
            "lgr" | "light-green-reverse" => Color::LightGreen.reverse(),
            "lgbl" | "light-green-blink" => Color::LightGreen.blink(),
            "lgst" | "light-green-strike" => Color::LightGreen.strikethrough(),

            "r" | "red" => Color::Red.normal(),
            "rb" | "red-bold" => Color::Red.bold(),
            "ru" | "red-underline" => Color::Red.underline(),
            "ri" | "red-italic" => Color::Red.italic(),
            "rd" | "red-dimmed" => Color::Red.dimmed(),
            "rr" | "red-reverse" => Color::Red.reverse(),
            "rbl" | "red-blink" => Color::Red.blink(),
            "rst" | "red-strike" => Color::Red.strikethrough(),

            "lr" | "light-red" => Color::LightRed.normal(),
            "lrb" | "light-red-bold" => Color::LightRed.bold(),
            "lru" | "light-red-underline" => Color::LightRed.underline(),
            "lri" | "light-red-italic" => Color::LightRed.italic(),
            "lrd" | "light-red-dimmed" => Color::LightRed.dimmed(),
            "lrr" | "light-red-reverse" => Color::LightRed.reverse(),
            "lrbl" | "light-red-blink" => Color::LightRed.blink(),
            "lrst" | "light-red-strike" => Color::LightRed.strikethrough(),

            "u" | "blue" => Color::Blue.normal(),
            "ub" | "blue-bold" => Color::Blue.bold(),
            "uu" | "blue-underline" => Color::Blue.underline(),
            "ui" | "blue-italic" => Color::Blue.italic(),
            "ud" | "blue-dimmed" => Color::Blue.dimmed(),
            "ur" | "blue-reverse" => Color::Blue.reverse(),
            "ubl" | "blue-blink" => Color::Blue.blink(),
            "ust" | "blue-strike" => Color::Blue.strikethrough(),

            "lu" | "light-blue" => Color::LightBlue.normal(),
            "lub" | "light-blue-bold" => Color::LightBlue.bold(),
            "luu" | "light-blue-underline" => Color::LightBlue.underline(),
            "lui" | "light-blue-italic" => Color::LightBlue.italic(),
            "lud" | "light-blue-dimmed" => Color::LightBlue.dimmed(),
            "lur" | "light-blue-reverse" => Color::LightBlue.reverse(),
            "lubl" | "light-blue-blink" => Color::LightBlue.blink(),
            "lust" | "light-blue-strike" => Color::LightBlue.strikethrough(),

            "b" | "black" => Color::Black.normal(),
            "bb" | "black-bold" => Color::Black.bold(),
            "bu" | "black-underline" => Color::Black.underline(),
            "bi" | "black-italic" => Color::Black.italic(),
            "bd" | "black-dimmed" => Color::Black.dimmed(),
            "br" | "black-reverse" => Color::Black.reverse(),
            "bbl" | "black-blink" => Color::Black.blink(),
            "bst" | "black-strike" => Color::Black.strikethrough(),

            "ligr" | "light-gray" => Color::LightGray.normal(),
            "ligrb" | "light-gray-bold" => Color::LightGray.bold(),
            "ligru" | "light-gray-underline" => Color::LightGray.underline(),
            "ligri" | "light-gray-italic" => Color::LightGray.italic(),
            "ligrd" | "light-gray-dimmed" => Color::LightGray.dimmed(),
            "ligrr" | "light-gray-reverse" => Color::LightGray.reverse(),
            "ligrbl" | "light-gray-blink" => Color::LightGray.blink(),
            "ligrst" | "light-gray-strike" => Color::LightGray.strikethrough(),

            "y" | "yellow" => Color::Yellow.normal(),
            "yb" | "yellow-bold" => Color::Yellow.bold(),
            "yu" | "yellow-underline" => Color::Yellow.underline(),
            "yi" | "yellow-italic" => Color::Yellow.italic(),
            "yd" | "yellow-dimmed" => Color::Yellow.dimmed(),
            "yr" | "yellow-reverse" => Color::Yellow.reverse(),
            "ybl" | "yellow-blink" => Color::Yellow.blink(),
            "yst" | "yellow-strike" => Color::Yellow.strikethrough(),

            "ly" | "light-yellow" => Color::LightYellow.normal(),
            "lyb" | "light-yellow-bold" => Color::LightYellow.bold(),
            "lyu" | "light-yellow-underline" => Color::LightYellow.underline(),
            "lyi" | "light-yellow-italic" => Color::LightYellow.italic(),
            "lyd" | "light-yellow-dimmed" => Color::LightYellow.dimmed(),
            "lyr" | "light-yellow-reverse" => Color::LightYellow.reverse(),
            "lybl" | "light-yellow-blink" => Color::LightYellow.blink(),
            "lyst" | "light-yellow-strike" => Color::LightYellow.strikethrough(),

            "p" | "purple" => Color::Purple.normal(),
            "pb" | "purple-bold" => Color::Purple.bold(),
            "pu" | "purple-underline" => Color::Purple.underline(),
            "pi" | "purple-italic" => Color::Purple.italic(),
            "pd" | "purple-dimmed" => Color::Purple.dimmed(),
            "pr" | "purple-reverse" => Color::Purple.reverse(),
            "pbl" | "purple-blink" => Color::Purple.blink(),
            "pst" | "purple-strike" => Color::Purple.strikethrough(),

            "lp" | "light-purple" => Color::LightPurple.normal(),
            "lpb" | "light-purple-bold" => Color::LightPurple.bold(),
            "lpu" | "light-purple-underline" => Color::LightPurple.underline(),
            "lpi" | "light-purple-italic" => Color::LightPurple.italic(),
            "lpd" | "light-purple-dimmed" => Color::LightPurple.dimmed(),
            "lpr" | "light-purple-reverse" => Color::LightPurple.reverse(),
            "lpbl" | "light-purple-blink" => Color::LightPurple.blink(),
            "lpst" | "light-purple-strike" => Color::LightPurple.strikethrough(),

            "c" | "cyan" => Color::Cyan.normal(),
            "cb" | "cyan-bold" => Color::Cyan.bold(),
            "cu" | "cyan-underline" => Color::Cyan.underline(),
            "ci" | "cyan-italic" => Color::Cyan.italic(),
            "cd" | "cyan-dimmed" => Color::Cyan.dimmed(),
            "cr" | "cyan-reverse" => Color::Cyan.reverse(),
            "cbl" | "cyan-blink" => Color::Cyan.blink(),
            "cst" | "cyan-strike" => Color::Cyan.strikethrough(),

            "lc" | "light-cyan" => Color::LightCyan.normal(),
            "lcb" | "light-cyan-bold" => Color::LightCyan.bold(),
            "lcu" | "light-cyan-underline" => Color::LightCyan.underline(),
            "lci" | "light-cyan-italic" => Color::LightCyan.italic(),
            "lcd" | "light-cyan-dimmed" => Color::LightCyan.dimmed(),
            "lcr" | "light-cyan-reverse" => Color::LightCyan.reverse(),
            "lcbl" | "light-cyan-blink" => Color::LightCyan.blink(),
            "lcst" | "light-cyan-strike" => Color::LightCyan.strikethrough(),

            "w" | "white" => Color::White.normal(),
            "wb" | "white-bold" => Color::White.bold(),
            "wu" | "white-underline" => Color::White.underline(),
            "wi" | "white-italic" => Color::White.italic(),
            "wd" | "white-dimmed" => Color::White.dimmed(),
            "wr" | "white-reverse" => Color::White.reverse(),
            "wbl" | "white-blink" => Color::White.blink(),
            "wst" | "white-strike" => Color::White.strikethrough(),

            "dgr" | "dark-gray" => Color::DarkGray.normal(),
            "dgrb" | "dark-gray-bold" => Color::DarkGray.bold(),
            "dgru" | "dark-gray-underline" => Color::DarkGray.underline(),
            "dgri" | "dark-gray-italic" => Color::DarkGray.italic(),
            "dgrd" | "dark-gray-dimmed" => Color::DarkGray.dimmed(),
            "dgrr" | "dark-gray-reverse" => Color::DarkGray.reverse(),
            "dgrbl" | "dark-gray-blink" => Color::DarkGray.blink(),
            "dgrst" | "dark-gray-strike" => Color::DarkGray.strikethrough(),

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
    // hm.insert("primitive-line".to_string(), Color::White.normal());
    // hm.insert("primitive-pattern".to_string(), Color::White.normal());
    // hm.insert("primitive-path".to_string(), Color::White.normal());
    // hm.insert("separator-color".to_string(), Color::White.normal());
    hm.insert(
        "leading-trailing-space-bg".to_string(),
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
    hm.insert("row-index".to_string(), Color::Green.bold());
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

        "row-index" => {
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
        _ => TextStyle::basic_left(),
    }
}

#[test]
fn test_hm() {
    use nu_ansi_term::{Color, Style};

    let mut hm: HashMap<String, Style> = HashMap::new();
    hm.insert("primitive-int".to_string(), Color::White.normal());
    hm.insert("primitive-decimal".to_string(), Color::White.normal());
    hm.insert("primitive-filesize".to_string(), Color::White.normal());
    hm.insert("primitive-string".to_string(), Color::White.normal());
    hm.insert("primitive-line".to_string(), Color::White.normal());
    hm.insert("primitive-columnpath".to_string(), Color::White.normal());
    hm.insert("primitive-pattern".to_string(), Color::White.normal());
    hm.insert("primitive-boolean".to_string(), Color::White.normal());
    hm.insert("primitive-date".to_string(), Color::White.normal());
    hm.insert("primitive-duration".to_string(), Color::White.normal());
    hm.insert("primitive-range".to_string(), Color::White.normal());
    hm.insert("primitive-path".to_string(), Color::White.normal());
    hm.insert("primitive-binary".to_string(), Color::White.normal());
    hm.insert("separator".to_string(), Color::White.normal());
    hm.insert("header-align".to_string(), Color::Green.bold());
    hm.insert("header".to_string(), Color::Green.bold());
    hm.insert("header-style".to_string(), Style::default());
    hm.insert("row-index".to_string(), Color::Green.bold());
    hm.insert(
        "leading-trailing-space-bg".to_string(),
        Style::default().on(Color::Rgb(128, 128, 128)),
    );

    update_hashmap("primitive-int", "green", &mut hm);

    assert_eq!(hm["primitive-int"], Color::Green.normal());
}
