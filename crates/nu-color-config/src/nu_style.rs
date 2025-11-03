use nu_ansi_term::{Color, Style};
use nu_protocol::Value;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Eq, Debug)]
pub struct NuStyle {
    pub fg: Option<String>,
    pub bg: Option<String>,
    pub attr: Option<String>,
}

impl From<Style> for NuStyle {
    fn from(s: Style) -> Self {
        Self {
            bg: s.background.and_then(color_to_string),
            fg: s.foreground.and_then(color_to_string),
            attr: style_get_attr(s),
        }
    }
}

fn style_get_attr(s: Style) -> Option<String> {
    let mut attrs = String::new();

    if s.is_blink {
        attrs.push('l');
    };
    if s.is_bold {
        attrs.push('b');
    };
    if s.is_dimmed {
        attrs.push('d');
    };
    if s.is_hidden {
        attrs.push('h');
    };
    if s.is_italic {
        attrs.push('i');
    };
    if s.is_reverse {
        attrs.push('r');
    };
    if s.is_strikethrough {
        attrs.push('s');
    };
    if s.is_underline {
        attrs.push('u');
    };

    if attrs.is_empty() { None } else { Some(attrs) }
}

fn color_to_string(color: Color) -> Option<String> {
    match color {
        Color::Black => Some(String::from("black")),
        Color::DarkGray => Some(String::from("dark_gray")),
        Color::Red => Some(String::from("red")),
        Color::LightRed => Some(String::from("light_red")),
        Color::Green => Some(String::from("green")),
        Color::LightGreen => Some(String::from("light_green")),
        Color::Yellow => Some(String::from("yellow")),
        Color::LightYellow => Some(String::from("light_yellow")),
        Color::Blue => Some(String::from("blue")),
        Color::LightBlue => Some(String::from("light_blue")),
        Color::Purple => Some(String::from("purple")),
        Color::LightPurple => Some(String::from("light_purple")),
        Color::Magenta => Some(String::from("magenta")),
        Color::LightMagenta => Some(String::from("light_magenta")),
        Color::Cyan => Some(String::from("cyan")),
        Color::LightCyan => Some(String::from("light_cyan")),
        Color::White => Some(String::from("white")),
        Color::LightGray => Some(String::from("light_gray")),
        Color::Default => Some(String::from("default")),
        Color::Rgb(r, g, b) => Some(format!("#{r:X}{g:X}{b:X}")),
        Color::Fixed(_) => None,
    }
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

// Converts the color_config records, { fg, bg, attr }, into a Style.
pub fn color_record_to_nustyle(value: &Value) -> Style {
    let mut fg = None;
    let mut bg = None;
    let mut attr = None;
    let v = value.as_record();
    if let Ok(record) = v {
        for (k, v) in record {
            // Because config already type-checked the color_config records, this doesn't bother giving errors
            // if there are unrecognised keys or bad values.
            if let Ok(v) = v.coerce_string() {
                match k.as_str() {
                    "fg" => fg = Some(v),

                    "bg" => bg = Some(v),

                    "attr" => attr = Some(v),
                    _ => (),
                }
            }
        }
    }

    parse_nustyle(NuStyle { fg, bg, attr })
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

        "m" | "magenta" => Color::Magenta.normal(),
        "mb" | "magenta_bold" => Color::Magenta.bold(),
        "mu" | "magenta_underline" => Color::Magenta.underline(),
        "mi" | "magenta_italic" => Color::Magenta.italic(),
        "md" | "magenta_dimmed" => Color::Magenta.dimmed(),
        "mr" | "magenta_reverse" => Color::Magenta.reverse(),
        "mbl" | "magenta_blink" => Color::Magenta.blink(),
        "mst" | "magenta_strike" => Color::Magenta.strikethrough(),

        "lm" | "light_magenta" => Color::LightMagenta.normal(),
        "lmb" | "light_magenta_bold" => Color::LightMagenta.bold(),
        "lmu" | "light_magenta_underline" => Color::LightMagenta.underline(),
        "lmi" | "light_magenta_italic" => Color::LightMagenta.italic(),
        "lmd" | "light_magenta_dimmed" => Color::LightMagenta.dimmed(),
        "lmr" | "light_magenta_reverse" => Color::LightMagenta.reverse(),
        "lmbl" | "light_magenta_blink" => Color::LightMagenta.blink(),
        "lmst" | "light_magenta_strike" => Color::LightMagenta.strikethrough(),

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

        // Add xterm 256 colors adding an x prefix where the name conflicts
        "xblack" | "xterm_black" => Color::Fixed(0).normal(),
        "maroon" | "xterm_maroon" => Color::Fixed(1).normal(),
        "xgreen" | "xterm_green" => Color::Fixed(2).normal(),
        "olive" | "xterm_olive" => Color::Fixed(3).normal(),
        "navy" | "xterm_navy" => Color::Fixed(4).normal(),
        "xpurplea" | "xterm_purplea" => Color::Fixed(5).normal(),
        "teal" | "xterm_teal" => Color::Fixed(6).normal(),
        "silver" | "xterm_silver" => Color::Fixed(7).normal(),
        "grey" | "xterm_grey" => Color::Fixed(8).normal(),
        "xred" | "xterm_red" => Color::Fixed(9).normal(),
        "lime" | "xterm_lime" => Color::Fixed(10).normal(),
        "xyellow" | "xterm_yellow" => Color::Fixed(11).normal(),
        "xblue" | "xterm_blue" => Color::Fixed(12).normal(),
        "fuchsia" | "xterm_fuchsia" => Color::Fixed(13).normal(),
        "aqua" | "xterm_aqua" => Color::Fixed(14).normal(),
        "xwhite" | "xterm_white" => Color::Fixed(15).normal(),
        "grey0" | "xterm_grey0" => Color::Fixed(16).normal(),
        "navyblue" | "xterm_navyblue" => Color::Fixed(17).normal(),
        "darkblue" | "xterm_darkblue" => Color::Fixed(18).normal(),
        "blue3a" | "xterm_blue3a" => Color::Fixed(19).normal(),
        "blue3b" | "xterm_blue3b" => Color::Fixed(20).normal(),
        "blue1" | "xterm_blue1" => Color::Fixed(21).normal(),
        "darkgreen" | "xterm_darkgreen" => Color::Fixed(22).normal(),
        "deepskyblue4a" | "xterm_deepskyblue4a" => Color::Fixed(23).normal(),
        "deepskyblue4b" | "xterm_deepskyblue4b" => Color::Fixed(24).normal(),
        "deepskyblue4c" | "xterm_deepskyblue4c" => Color::Fixed(25).normal(),
        "dodgerblue3" | "xterm_dodgerblue3" => Color::Fixed(26).normal(),
        "dodgerblue2" | "xterm_dodgerblue2" => Color::Fixed(27).normal(),
        "green4" | "xterm_green4" => Color::Fixed(28).normal(),
        "springgreen4" | "xterm_springgreen4" => Color::Fixed(29).normal(),
        "turquoise4" | "xterm_turquoise4" => Color::Fixed(30).normal(),
        "deepskyblue3a" | "xterm_deepskyblue3a" => Color::Fixed(31).normal(),
        "deepskyblue3b" | "xterm_deepskyblue3b" => Color::Fixed(32).normal(),
        "dodgerblue1" | "xterm_dodgerblue1" => Color::Fixed(33).normal(),
        "green3a" | "xterm_green3a" => Color::Fixed(34).normal(),
        "springgreen3a" | "xterm_springgreen3a" => Color::Fixed(35).normal(),
        "darkcyan" | "xterm_darkcyan" => Color::Fixed(36).normal(),
        "lightseagreen" | "xterm_lightseagreen" => Color::Fixed(37).normal(),
        "deepskyblue2" | "xterm_deepskyblue2" => Color::Fixed(38).normal(),
        "deepskyblue1" | "xterm_deepskyblue1" => Color::Fixed(39).normal(),
        "green3b" | "xterm_green3b" => Color::Fixed(40).normal(),
        "springgreen3b" | "xterm_springgreen3b" => Color::Fixed(41).normal(),
        "springgreen2a" | "xterm_springgreen2a" => Color::Fixed(42).normal(),
        "cyan3" | "xterm_cyan3" => Color::Fixed(43).normal(),
        "darkturquoise" | "xterm_darkturquoise" => Color::Fixed(44).normal(),
        "turquoise2" | "xterm_turquoise2" => Color::Fixed(45).normal(),
        "green1" | "xterm_green1" => Color::Fixed(46).normal(),
        "springgreen2b" | "xterm_springgreen2b" => Color::Fixed(47).normal(),
        "springgreen1" | "xterm_springgreen1" => Color::Fixed(48).normal(),
        "mediumspringgreen" | "xterm_mediumspringgreen" => Color::Fixed(49).normal(),
        "cyan2" | "xterm_cyan2" => Color::Fixed(50).normal(),
        "cyan1" | "xterm_cyan1" => Color::Fixed(51).normal(),
        "darkreda" | "xterm_darkreda" => Color::Fixed(52).normal(),
        "deeppink4a" | "xterm_deeppink4a" => Color::Fixed(53).normal(),
        "purple4a" | "xterm_purple4a" => Color::Fixed(54).normal(),
        "purple4b" | "xterm_purple4b" => Color::Fixed(55).normal(),
        "purple3" | "xterm_purple3" => Color::Fixed(56).normal(),
        "blueviolet" | "xterm_blueviolet" => Color::Fixed(57).normal(),
        "orange4a" | "xterm_orange4a" => Color::Fixed(58).normal(),
        "grey37" | "xterm_grey37" => Color::Fixed(59).normal(),
        "mediumpurple4" | "xterm_mediumpurple4" => Color::Fixed(60).normal(),
        "slateblue3a" | "xterm_slateblue3a" => Color::Fixed(61).normal(),
        "slateblue3b" | "xterm_slateblue3b" => Color::Fixed(62).normal(),
        "royalblue1" | "xterm_royalblue1" => Color::Fixed(63).normal(),
        "chartreuse4" | "xterm_chartreuse4" => Color::Fixed(64).normal(),
        "darkseagreen4a" | "xterm_darkseagreen4a" => Color::Fixed(65).normal(),
        "paleturquoise4" | "xterm_paleturquoise4" => Color::Fixed(66).normal(),
        "steelblue" | "xterm_steelblue" => Color::Fixed(67).normal(),
        "steelblue3" | "xterm_steelblue3" => Color::Fixed(68).normal(),
        "cornflowerblue" | "xterm_cornflowerblue" => Color::Fixed(69).normal(),
        "chartreuse3a" | "xterm_chartreuse3a" => Color::Fixed(70).normal(),
        "darkseagreen4b" | "xterm_darkseagreen4b" => Color::Fixed(71).normal(),
        "cadetbluea" | "xterm_cadetbluea" => Color::Fixed(72).normal(),
        "cadetblueb" | "xterm_cadetblueb" => Color::Fixed(73).normal(),
        "skyblue3" | "xterm_skyblue3" => Color::Fixed(74).normal(),
        "steelblue1a" | "xterm_steelblue1a" => Color::Fixed(75).normal(),
        "chartreuse3b" | "xterm_chartreuse3b" => Color::Fixed(76).normal(),
        "palegreen3a" | "xterm_palegreen3a" => Color::Fixed(77).normal(),
        "seagreen3" | "xterm_seagreen3" => Color::Fixed(78).normal(),
        "aquamarine3" | "xterm_aquamarine3" => Color::Fixed(79).normal(),
        "mediumturquoise" | "xterm_mediumturquoise" => Color::Fixed(80).normal(),
        "steelblue1b" | "xterm_steelblue1b" => Color::Fixed(81).normal(),
        "chartreuse2a" | "xterm_chartreuse2a" => Color::Fixed(82).normal(),
        "seagreen2" | "xterm_seagreen2" => Color::Fixed(83).normal(),
        "seagreen1a" | "xterm_seagreen1a" => Color::Fixed(84).normal(),
        "seagreen1b" | "xterm_seagreen1b" => Color::Fixed(85).normal(),
        "aquamarine1a" | "xterm_aquamarine1a" => Color::Fixed(86).normal(),
        "darkslategray2" | "xterm_darkslategray2" => Color::Fixed(87).normal(),
        "darkredb" | "xterm_darkredb" => Color::Fixed(88).normal(),
        "deeppink4b" | "xterm_deeppink4b" => Color::Fixed(89).normal(),
        "darkmagentaa" | "xterm_darkmagentaa" => Color::Fixed(90).normal(),
        "darkmagentab" | "xterm_darkmagentab" => Color::Fixed(91).normal(),
        "darkvioleta" | "xterm_darkvioleta" => Color::Fixed(92).normal(),
        "xpurpleb" | "xterm_purpleb" => Color::Fixed(93).normal(),
        "orange4b" | "xterm_orange4b" => Color::Fixed(94).normal(),
        "lightpink4" | "xterm_lightpink4" => Color::Fixed(95).normal(),
        "plum4" | "xterm_plum4" => Color::Fixed(96).normal(),
        "mediumpurple3a" | "xterm_mediumpurple3a" => Color::Fixed(97).normal(),
        "mediumpurple3b" | "xterm_mediumpurple3b" => Color::Fixed(98).normal(),
        "slateblue1" | "xterm_slateblue1" => Color::Fixed(99).normal(),
        "yellow4a" | "xterm_yellow4a" => Color::Fixed(100).normal(),
        "wheat4" | "xterm_wheat4" => Color::Fixed(101).normal(),
        "grey53" | "xterm_grey53" => Color::Fixed(102).normal(),
        "lightslategrey" | "xterm_lightslategrey" => Color::Fixed(103).normal(),
        "mediumpurple" | "xterm_mediumpurple" => Color::Fixed(104).normal(),
        "lightslateblue" | "xterm_lightslateblue" => Color::Fixed(105).normal(),
        "yellow4b" | "xterm_yellow4b" => Color::Fixed(106).normal(),
        "darkolivegreen3a" | "xterm_darkolivegreen3a" => Color::Fixed(107).normal(),
        "darkseagreen" | "xterm_darkseagreen" => Color::Fixed(108).normal(),
        "lightskyblue3a" | "xterm_lightskyblue3a" => Color::Fixed(109).normal(),
        "lightskyblue3b" | "xterm_lightskyblue3b" => Color::Fixed(110).normal(),
        "skyblue2" | "xterm_skyblue2" => Color::Fixed(111).normal(),
        "chartreuse2b" | "xterm_chartreuse2b" => Color::Fixed(112).normal(),
        "darkolivegreen3b" | "xterm_darkolivegreen3b" => Color::Fixed(113).normal(),
        "palegreen3b" | "xterm_palegreen3b" => Color::Fixed(114).normal(),
        "darkseagreen3a" | "xterm_darkseagreen3a" => Color::Fixed(115).normal(),
        "darkslategray3" | "xterm_darkslategray3" => Color::Fixed(116).normal(),
        "skyblue1" | "xterm_skyblue1" => Color::Fixed(117).normal(),
        "chartreuse1" | "xterm_chartreuse1" => Color::Fixed(118).normal(),
        "lightgreena" | "xterm_lightgreena" => Color::Fixed(119).normal(),
        "lightgreenb" | "xterm_lightgreenb" => Color::Fixed(120).normal(),
        "palegreen1a" | "xterm_palegreen1a" => Color::Fixed(121).normal(),
        "aquamarine1b" | "xterm_aquamarine1b" => Color::Fixed(122).normal(),
        "darkslategray1" | "xterm_darkslategray1" => Color::Fixed(123).normal(),
        "red3a" | "xterm_red3a" => Color::Fixed(124).normal(),
        "deeppink4c" | "xterm_deeppink4c" => Color::Fixed(125).normal(),
        "mediumvioletred" | "xterm_mediumvioletred" => Color::Fixed(126).normal(),
        "magenta3" | "xterm_magenta3" => Color::Fixed(127).normal(),
        "darkvioletb" | "xterm_darkvioletb" => Color::Fixed(128).normal(),
        "purplec" | "xterm_purplec" => Color::Fixed(129).normal(),
        "darkorange3a" | "xterm_darkorange3a" => Color::Fixed(130).normal(),
        "indianreda" | "xterm_indianreda" => Color::Fixed(131).normal(),
        "hotpink3a" | "xterm_hotpink3a" => Color::Fixed(132).normal(),
        "mediumorchid3" | "xterm_mediumorchid3" => Color::Fixed(133).normal(),
        "mediumorchid" | "xterm_mediumorchid" => Color::Fixed(134).normal(),
        "mediumpurple2a" | "xterm_mediumpurple2a" => Color::Fixed(135).normal(),
        "darkgoldenrod" | "xterm_darkgoldenrod" => Color::Fixed(136).normal(),
        "lightsalmon3a" | "xterm_lightsalmon3a" => Color::Fixed(137).normal(),
        "rosybrown" | "xterm_rosybrown" => Color::Fixed(138).normal(),
        "grey63" | "xterm_grey63" => Color::Fixed(139).normal(),
        "mediumpurple2b" | "xterm_mediumpurple2b" => Color::Fixed(140).normal(),
        "mediumpurple1" | "xterm_mediumpurple1" => Color::Fixed(141).normal(),
        "gold3a" | "xterm_gold3a" => Color::Fixed(142).normal(),
        "darkkhaki" | "xterm_darkkhaki" => Color::Fixed(143).normal(),
        "navajowhite3" | "xterm_navajowhite3" => Color::Fixed(144).normal(),
        "grey69" | "xterm_grey69" => Color::Fixed(145).normal(),
        "lightsteelblue3" | "xterm_lightsteelblue3" => Color::Fixed(146).normal(),
        "lightsteelblue" | "xterm_lightsteelblue" => Color::Fixed(147).normal(),
        "yellow3a" | "xterm_yellow3a" => Color::Fixed(148).normal(),
        "darkolivegreen3c" | "xterm_darkolivegreen3c" => Color::Fixed(149).normal(),
        "darkseagreen3b" | "xterm_darkseagreen3b" => Color::Fixed(150).normal(),
        "darkseagreen2a" | "xterm_darkseagreen2a" => Color::Fixed(151).normal(),
        "lightcyan3" | "xterm_lightcyan3" => Color::Fixed(152).normal(),
        "lightskyblue1" | "xterm_lightskyblue1" => Color::Fixed(153).normal(),
        "greenyellow" | "xterm_greenyellow" => Color::Fixed(154).normal(),
        "darkolivegreen2" | "xterm_darkolivegreen2" => Color::Fixed(155).normal(),
        "palegreen1b" | "xterm_palegreen1b" => Color::Fixed(156).normal(),
        "darkseagreen2b" | "xterm_darkseagreen2b" => Color::Fixed(157).normal(),
        "darkseagreen1a" | "xterm_darkseagreen1a" => Color::Fixed(158).normal(),
        "paleturquoise1" | "xterm_paleturquoise1" => Color::Fixed(159).normal(),
        "red3b" | "xterm_red3b" => Color::Fixed(160).normal(),
        "deeppink3a" | "xterm_deeppink3a" => Color::Fixed(161).normal(),
        "deeppink3b" | "xterm_deeppink3b" => Color::Fixed(162).normal(),
        "magenta3a" | "xterm_magenta3a" => Color::Fixed(163).normal(),
        "magenta3b" | "xterm_magenta3b" => Color::Fixed(164).normal(),
        "magenta2a" | "xterm_magenta2a" => Color::Fixed(165).normal(),
        "darkorange3b" | "xterm_darkorange3b" => Color::Fixed(166).normal(),
        "indianredb" | "xterm_indianredb" => Color::Fixed(167).normal(),
        "hotpink3b" | "xterm_hotpink3b" => Color::Fixed(168).normal(),
        "hotpink2" | "xterm_hotpink2" => Color::Fixed(169).normal(),
        "orchid" | "xterm_orchid" => Color::Fixed(170).normal(),
        "mediumorchid1a" | "xterm_mediumorchid1a" => Color::Fixed(171).normal(),
        "orange3" | "xterm_orange3" => Color::Fixed(172).normal(),
        "lightsalmon3b" | "xterm_lightsalmon3b" => Color::Fixed(173).normal(),
        "lightpink3" | "xterm_lightpink3" => Color::Fixed(174).normal(),
        "pink3" | "xterm_pink3" => Color::Fixed(175).normal(),
        "plum3" | "xterm_plum3" => Color::Fixed(176).normal(),
        "violet" | "xterm_violet" => Color::Fixed(177).normal(),
        "gold3b" | "xterm_gold3b" => Color::Fixed(178).normal(),
        "lightgoldenrod3" | "xterm_lightgoldenrod3" => Color::Fixed(179).normal(),
        "tan" | "xterm_tan" => Color::Fixed(180).normal(),
        "mistyrose3" | "xterm_mistyrose3" => Color::Fixed(181).normal(),
        "thistle3" | "xterm_thistle3" => Color::Fixed(182).normal(),
        "plum2" | "xterm_plum2" => Color::Fixed(183).normal(),
        "yellow3b" | "xterm_yellow3b" => Color::Fixed(184).normal(),
        "khaki3" | "xterm_khaki3" => Color::Fixed(185).normal(),
        "lightgoldenrod2" | "xterm_lightgoldenrod2" => Color::Fixed(186).normal(),
        "lightyellow3" | "xterm_lightyellow3" => Color::Fixed(187).normal(),
        "grey84" | "xterm_grey84" => Color::Fixed(188).normal(),
        "lightsteelblue1" | "xterm_lightsteelblue1" => Color::Fixed(189).normal(),
        "yellow2" | "xterm_yellow2" => Color::Fixed(190).normal(),
        "darkolivegreen1a" | "xterm_darkolivegreen1a" => Color::Fixed(191).normal(),
        "darkolivegreen1b" | "xterm_darkolivegreen1b" => Color::Fixed(192).normal(),
        "darkseagreen1b" | "xterm_darkseagreen1b" => Color::Fixed(193).normal(),
        "honeydew2" | "xterm_honeydew2" => Color::Fixed(194).normal(),
        "lightcyan1" | "xterm_lightcyan1" => Color::Fixed(195).normal(),
        "red1" | "xterm_red1" => Color::Fixed(196).normal(),
        "deeppink2" | "xterm_deeppink2" => Color::Fixed(197).normal(),
        "deeppink1a" | "xterm_deeppink1a" => Color::Fixed(198).normal(),
        "deeppink1b" | "xterm_deeppink1b" => Color::Fixed(199).normal(),
        "magenta2b" | "xterm_magenta2b" => Color::Fixed(200).normal(),
        "magenta1" | "xterm_magenta1" => Color::Fixed(201).normal(),
        "orangered1" | "xterm_orangered1" => Color::Fixed(202).normal(),
        "indianred1a" | "xterm_indianred1a" => Color::Fixed(203).normal(),
        "indianred1b" | "xterm_indianred1b" => Color::Fixed(204).normal(),
        "hotpinka" | "xterm_hotpinka" => Color::Fixed(205).normal(),
        "hotpinkb" | "xterm_hotpinkb" => Color::Fixed(206).normal(),
        "mediumorchid1b" | "xterm_mediumorchid1b" => Color::Fixed(207).normal(),
        "darkorange" | "xterm_darkorange" => Color::Fixed(208).normal(),
        "salmon1" | "xterm_salmon1" => Color::Fixed(209).normal(),
        "lightcoral" | "xterm_lightcoral" => Color::Fixed(210).normal(),
        "palevioletred1" | "xterm_palevioletred1" => Color::Fixed(211).normal(),
        "orchid2" | "xterm_orchid2" => Color::Fixed(212).normal(),
        "orchid1" | "xterm_orchid1" => Color::Fixed(213).normal(),
        "orange1" | "xterm_orange1" => Color::Fixed(214).normal(),
        "sandybrown" | "xterm_sandybrown" => Color::Fixed(215).normal(),
        "lightsalmon1" | "xterm_lightsalmon1" => Color::Fixed(216).normal(),
        "lightpink1" | "xterm_lightpink1" => Color::Fixed(217).normal(),
        "pink1" | "xterm_pink1" => Color::Fixed(218).normal(),
        "plum1" | "xterm_plum1" => Color::Fixed(219).normal(),
        "gold1" | "xterm_gold1" => Color::Fixed(220).normal(),
        "lightgoldenrod2a" | "xterm_lightgoldenrod2a" => Color::Fixed(221).normal(),
        "lightgoldenrod2b" | "xterm_lightgoldenrod2b" => Color::Fixed(222).normal(),
        "navajowhite1" | "xterm_navajowhite1" => Color::Fixed(223).normal(),
        "mistyrose1" | "xterm_mistyrose1" => Color::Fixed(224).normal(),
        "thistle1" | "xterm_thistle1" => Color::Fixed(225).normal(),
        "yellow1" | "xterm_yellow1" => Color::Fixed(226).normal(),
        "lightgoldenrod1" | "xterm_lightgoldenrod1" => Color::Fixed(227).normal(),
        "khaki1" | "xterm_khaki1" => Color::Fixed(228).normal(),
        "wheat1" | "xterm_wheat1" => Color::Fixed(229).normal(),
        "cornsilk1" | "xterm_cornsilk1" => Color::Fixed(230).normal(),
        "grey100" | "xterm_grey100" => Color::Fixed(231).normal(),
        "grey3" | "xterm_grey3" => Color::Fixed(232).normal(),
        "grey7" | "xterm_grey7" => Color::Fixed(233).normal(),
        "grey11" | "xterm_grey11" => Color::Fixed(234).normal(),
        "grey15" | "xterm_grey15" => Color::Fixed(235).normal(),
        "grey19" | "xterm_grey19" => Color::Fixed(236).normal(),
        "grey23" | "xterm_grey23" => Color::Fixed(237).normal(),
        "grey27" | "xterm_grey27" => Color::Fixed(238).normal(),
        "grey30" | "xterm_grey30" => Color::Fixed(239).normal(),
        "grey35" | "xterm_grey35" => Color::Fixed(240).normal(),
        "grey39" | "xterm_grey39" => Color::Fixed(241).normal(),
        "grey42" | "xterm_grey42" => Color::Fixed(242).normal(),
        "grey46" | "xterm_grey46" => Color::Fixed(243).normal(),
        "grey50" | "xterm_grey50" => Color::Fixed(244).normal(),
        "grey54" | "xterm_grey54" => Color::Fixed(245).normal(),
        "grey58" | "xterm_grey58" => Color::Fixed(246).normal(),
        "grey62" | "xterm_grey62" => Color::Fixed(247).normal(),
        "grey66" | "xterm_grey66" => Color::Fixed(248).normal(),
        "grey70" | "xterm_grey70" => Color::Fixed(249).normal(),
        "grey74" | "xterm_grey74" => Color::Fixed(250).normal(),
        "grey78" | "xterm_grey78" => Color::Fixed(251).normal(),
        "grey82" | "xterm_grey82" => Color::Fixed(252).normal(),
        "grey85" | "xterm_grey85" => Color::Fixed(253).normal(),
        "grey89" | "xterm_grey89" => Color::Fixed(254).normal(),
        "grey93" | "xterm_grey93" => Color::Fixed(255).normal(),
        _ => Color::Default.normal(),
    }
}

pub fn lookup_color(s: &str) -> Option<Color> {
    lookup_style(s).foreground
}

fn fill_modifiers(attrs: &str, style: &mut Style) {
    // setup the attributes available in nu_ansi_term::Style
    //
    // since we can combine styles like bold-italic, iterate through the chars
    // and set the bools for later use in the nu_ansi_term::Style application
    for ch in attrs.chars().map(|c| c.to_ascii_lowercase()) {
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
