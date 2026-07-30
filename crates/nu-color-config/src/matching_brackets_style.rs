use crate::{color_config::lookup_ansi_color_style, color_record_to_nustyle};
use nu_ansi_term::Style;
use nu_protocol::{Config, Value};

pub fn get_matching_brackets_style(default_style: Style, conf: &Config) -> Style {
    const MATCHING_BRACKETS_CONFIG_KEY: &str = "shape_matching_brackets";

    match conf.color_config.get(MATCHING_BRACKETS_CONFIG_KEY) {
        Some(Value::String { val, .. }) => {
            merge_styles(default_style, lookup_ansi_color_style(val))
        }
        Some(value @ Value::Record { .. }) => {
            merge_styles(default_style, color_record_to_nustyle(value))
        }
        // Missing or unusable types: keep the base shape style only
        _ => default_style,
    }
}

fn merge_styles(base: Style, extra: Style) -> Style {
    // `default_*` color names (e.g. `default_underline`) set `Color::Default`.
    // For matching brackets we merge onto the shape style, so treat Default as
    // "keep the base color" and only layer attributes / explicit colors.
    Style {
        foreground: merge_color(base.foreground, extra.foreground),
        background: merge_color(base.background, extra.background),
        is_bold: extra.is_bold || base.is_bold,
        is_dimmed: extra.is_dimmed || base.is_dimmed,
        is_italic: extra.is_italic || base.is_italic,
        is_underline: extra.is_underline || base.is_underline,
        is_blink: extra.is_blink || base.is_blink,
        is_reverse: extra.is_reverse || base.is_reverse,
        is_hidden: extra.is_hidden || base.is_hidden,
        is_strikethrough: extra.is_strikethrough || base.is_strikethrough,
        prefix_with_reset: false,
    }
}

fn merge_color(
    base: Option<nu_ansi_term::Color>,
    extra: Option<nu_ansi_term::Color>,
) -> Option<nu_ansi_term::Color> {
    match extra {
        None | Some(nu_ansi_term::Color::Default) => base,
        other => other,
    }
}
