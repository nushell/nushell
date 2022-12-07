use nu_ansi_term::{Color, Style};
use serde::Deserialize;

#[derive(Deserialize, PartialEq, Eq, Debug)]
pub struct NuStyle {
    pub fg: Option<String>,
    pub bg: Option<String>,
    pub attr: Option<String>,
}

pub fn parse_nustyle(nu_style: NuStyle) -> Style {
    let mut style = Style {
        foreground: nu_style.fg.and_then(|fg| lookup_color_str(&fg)),
        background: nu_style.bg.and_then(|bg| lookup_color_str(&bg)),
        ..Default::default()
    };

    if let Some(attrs) = nu_style.attr {
        fill_modifiers(&attrs, &mut style)
    }

    style
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

pub fn lookup_style(s: &str) -> Style {
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

pub fn lookup_color(s: &str) -> Option<Color> {
    let color = match s {
        "g" | "green" => Color::Green,
        "lg" | "light_green" => Color::LightGreen,
        "r" | "red" => Color::Red,
        "lr" | "light_red" => Color::LightRed,
        "u" | "blue" => Color::Blue,
        "lu" | "light_blue" => Color::LightBlue,
        "b" | "black" => Color::Black,
        "ligr" | "light_gray" => Color::LightGray,
        "y" | "yellow" => Color::Yellow,
        "ly" | "light_yellow" => Color::LightYellow,
        "p" | "purple" => Color::Purple,
        "lp" | "light_purple" => Color::LightPurple,
        "c" | "cyan" => Color::Cyan,
        "lc" | "light_cyan" => Color::LightCyan,
        "w" | "white" => Color::White,
        "dgr" | "dark_gray" => Color::DarkGray,
        "def" | "default" => Color::Default,
        _ => return None,
    };

    Some(color)
}

fn fill_modifiers(attrs: &str, style: &mut Style) {
    // setup the attributes available in nu_ansi_term::Style
    //
    // since we can combine styles like bold-italic, iterate through the chars
    // and set the bools for later use in the nu_ansi_term::Style application
    for ch in attrs.to_lowercase().chars() {
        match ch {
            'l' => style.is_blink = true,
            'b' => style.is_bold = true,
            'd' => style.is_dimmed = true,
            'h' => style.is_hidden = true,
            'i' => style.is_italic = true,
            'r' => style.is_reverse = true,
            's' => style.is_strikethrough = true,
            'u' => style.is_underline = true,
            'n' => (),
            _ => (),
        }
    }
}

fn lookup_color_str(s: &str) -> Option<Color> {
    if s.starts_with('#') {
        color_from_hex(s).ok().and_then(|c| c)
    } else {
        lookup_color(s)
    }
}
