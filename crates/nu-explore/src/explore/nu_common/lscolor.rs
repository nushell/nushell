use std::path::Path;

use super::NuText;
use lscolors::LsColors;
use nu_ansi_term::{Color, Style};
use nu_engine::env_to_string;
use nu_path::{expand_path_with, expand_to_real_path};
use nu_protocol::engine::{EngineState, Stack};
use nu_utils::get_ls_colors;

pub fn create_lscolors(engine_state: &EngineState, stack: &Stack) -> LsColors {
    let colors = stack
        .get_env_var(engine_state, "LS_COLORS")
        .and_then(|v| env_to_string("LS_COLORS", v, engine_state, stack).ok());

    get_ls_colors(colors)
}

/// Colorizes any columns named "name" in the table using LS_COLORS
pub fn lscolorize(header: &[String], data: &mut [Vec<NuText>], cwd: &str, lscolors: &LsColors) {
    for (col, col_name) in header.iter().enumerate() {
        if col_name != "name" {
            continue;
        }

        for row in data.iter_mut() {
            let (path, text_style) = &mut row[col];

            let style = get_path_style(path, cwd, lscolors);
            if let Some(style) = style {
                *text_style = text_style.style(style);
            }
        }
    }
}

fn get_path_style(path: &str, cwd: &str, ls_colors: &LsColors) -> Option<Style> {
    let stripped_path = nu_utils::strip_ansi_unlikely(path);
    let mut style = ls_colors.style_for_str(stripped_path.as_ref());

    let is_likely_dir = style.is_none();
    if is_likely_dir {
        let mut meta = std::fs::symlink_metadata(path).ok();

        if meta.is_none() {
            let mut expanded_path = expand_to_real_path(path);
            let try_cwd = expanded_path.as_path() == Path::new(path);
            if try_cwd {
                let cwd_path = format!("./{path}");
                expanded_path = expand_path_with(cwd_path, cwd, false);
            }

            meta = std::fs::symlink_metadata(expanded_path.as_path()).ok();
            style = ls_colors.style_for_path_with_metadata(expanded_path.as_path(), meta.as_ref());
        } else {
            style = ls_colors.style_for_path_with_metadata(path, meta.as_ref());
        }
    }

    style.map(lsstyle_to_nu_style)
}

fn lsstyle_to_nu_style(s: &lscolors::Style) -> Style {
    let mut out = Style::default();
    if let Some(clr) = &s.background {
        out.background = lscolor_to_nu_color(clr);
    }

    if let Some(clr) = &s.foreground {
        out.foreground = lscolor_to_nu_color(clr);
    }

    if s.font_style.slow_blink | s.font_style.rapid_blink {
        out.is_blink = true;
    }

    if s.font_style.bold {
        out.is_bold = true;
    }

    if s.font_style.dimmed {
        out.is_dimmed = true;
    }

    if s.font_style.hidden {
        out.is_hidden = true;
    }

    if s.font_style.reverse {
        out.is_reverse = true;
    }

    if s.font_style.italic {
        out.is_italic = true;
    }

    if s.font_style.underline {
        out.is_underline = true;
    }

    out
}

fn lscolor_to_nu_color(clr: &lscolors::Color) -> Option<Color> {
    use lscolors::Color::*;

    let clr = match clr {
        Black => Color::Black,
        BrightBlack => Color::DarkGray,
        Red => Color::Red,
        BrightRed => Color::LightRed,
        Green => Color::Green,
        BrightGreen => Color::LightGreen,
        Yellow => Color::Yellow,
        BrightYellow => Color::LightYellow,
        Blue => Color::Blue,
        BrightBlue => Color::LightBlue,
        Magenta => Color::Magenta,
        BrightMagenta => Color::LightMagenta,
        Cyan => Color::Cyan,
        BrightCyan => Color::LightCyan,
        White => Color::White,
        BrightWhite => Color::LightGray,
        &Fixed(i) => Color::Fixed(i),
        &RGB(r, g, b) => Color::Rgb(r, g, b),
    };

    Some(clr)
}
