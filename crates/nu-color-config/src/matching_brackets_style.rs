use crate::color_config::lookup_ansi_color_style;
use nu_ansi_term::Style;
use nu_protocol::Config;

pub fn get_matching_brackets_style(default_style: Style, conf: &Config) -> Style {
    const MATCHING_BRACKETS_CONFIG_KEY: &str = "shape_matching_brackets";

    match conf.color_config.get(MATCHING_BRACKETS_CONFIG_KEY) {
        Some(int_color) => match int_color.as_string() {
            Ok(int_color) => merge_styles(default_style, lookup_ansi_color_style(&int_color)),
            Err(_) => default_style,
        },
        None => default_style,
    }
}

fn merge_styles(base: Style, extra: Style) -> Style {
    Style {
        foreground: extra.foreground.or(base.foreground),
        background: extra.background.or(base.background),
        is_bold: extra.is_bold || base.is_bold,
        is_dimmed: extra.is_dimmed || base.is_dimmed,
        is_italic: extra.is_italic || base.is_italic,
        is_underline: extra.is_underline || base.is_underline,
        is_blink: extra.is_blink || base.is_blink,
        is_reverse: extra.is_reverse || base.is_reverse,
        is_hidden: extra.is_hidden || base.is_hidden,
        is_strikethrough: extra.is_strikethrough || base.is_strikethrough,
    }
}
