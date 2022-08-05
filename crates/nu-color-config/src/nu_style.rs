use nu_ansi_term::{Color, Style};
use serde::Deserialize;

#[derive(Deserialize, PartialEq, Eq, Debug)]
pub struct NuStyle {
    pub fg: Option<String>,
    pub bg: Option<String>,
    pub attr: Option<String>,
}

pub fn parse_nustyle(nu_style: NuStyle) -> Style {
    // get the nu_ansi_term::Color foreground color
    let fg_color = match nu_style.fg {
        Some(fg) => color_from_hex(&fg).unwrap_or_default(),
        _ => None,
    };
    // get the nu_ansi_term::Color background color
    let bg_color = match nu_style.bg {
        Some(bg) => color_from_hex(&bg).unwrap_or_default(),
        _ => None,
    };
    // get the attributes
    let color_attr = match nu_style.attr {
        Some(attr) => attr,
        _ => "".to_string(),
    };

    // setup the attributes available in nu_ansi_term::Style
    let mut bold = false;
    let mut dimmed = false;
    let mut italic = false;
    let mut underline = false;
    let mut blink = false;
    let mut reverse = false;
    let mut hidden = false;
    let mut strikethrough = false;

    // since we can combine styles like bold-italic, iterate through the chars
    // and set the bools for later use in the nu_ansi_term::Style application
    for ch in color_attr.to_lowercase().chars() {
        match ch {
            'l' => blink = true,
            'b' => bold = true,
            'd' => dimmed = true,
            'h' => hidden = true,
            'i' => italic = true,
            'r' => reverse = true,
            's' => strikethrough = true,
            'u' => underline = true,
            'n' => (),
            _ => (),
        }
    }

    // here's where we build the nu_ansi_term::Style
    Style {
        foreground: fg_color,
        background: bg_color,
        is_blink: blink,
        is_bold: bold,
        is_dimmed: dimmed,
        is_hidden: hidden,
        is_italic: italic,
        is_reverse: reverse,
        is_strikethrough: strikethrough,
        is_underline: underline,
    }
}

pub fn color_string_to_nustyle(color_string: String) -> Style {
    // eprintln!("color_string: {}", &color_string);
    if color_string.chars().count() < 1 {
        Style::default()
    } else {
        let nu_style = match nu_json::from_str::<NuStyle>(&color_string) {
            Ok(s) => s,
            Err(_) => NuStyle {
                fg: None,
                bg: None,
                attr: None,
            },
        };

        parse_nustyle(nu_style)
    }
}

pub fn color_from_hex(
    hex_color: &str,
) -> std::result::Result<Option<Color>, std::num::ParseIntError> {
    // right now we only allow hex colors with hashtag and 6 characters
    let trimmed = hex_color.trim_matches('#');
    if trimmed.len() != 6 {
        Ok(None)
    } else {
        // make a nu_ansi_term::Color::Rgb color by converting hex to decimal
        Ok(Some(Color::Rgb(
            u8::from_str_radix(&trimmed[..2], 16)?,
            u8::from_str_radix(&trimmed[2..4], 16)?,
            u8::from_str_radix(&trimmed[4..6], 16)?,
        )))
    }
}
