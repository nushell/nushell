use crate::{color_config::lookup_ansi_color_style, color_record_to_nustyle};
use nu_ansi_term::{Color, Style};
use nu_protocol::{Config, Value};

// The default colors for shapes, used when there is no config for them.
pub fn default_shape_color(shape: &str) -> Style {
    match shape {
        "shape_binary" => Style::new().fg(Color::Purple).bold(),
        "shape_block" => Style::new().fg(Color::Blue).bold(),
        "shape_bool" => Style::new().fg(Color::LightCyan),
        "shape_closure" => Style::new().fg(Color::Green).bold(),
        "shape_custom" => Style::new().fg(Color::Green),
        "shape_datetime" => Style::new().fg(Color::Cyan).bold(),
        "shape_directory" => Style::new().fg(Color::Cyan),
        "shape_external" => Style::new().fg(Color::Cyan),
        "shape_externalarg" => Style::new().fg(Color::Green).bold(),
        "shape_external_resolved" => Style::new().fg(Color::LightYellow).bold(),
        "shape_filepath" => Style::new().fg(Color::Cyan),
        "shape_flag" => Style::new().fg(Color::Blue).bold(),
        "shape_float" => Style::new().fg(Color::Purple).bold(),
        "shape_garbage" => Style::new().fg(Color::Default).on(Color::Red).bold(),
        "shape_glob_interpolation" => Style::new().fg(Color::Cyan).bold(),
        "shape_globpattern" => Style::new().fg(Color::Cyan).bold(),
        "shape_int" => Style::new().fg(Color::Purple).bold(),
        "shape_internalcall" => Style::new().fg(Color::Cyan).bold(),
        "shape_keyword" => Style::new().fg(Color::Cyan).bold(),
        "shape_list" => Style::new().fg(Color::Cyan).bold(),
        "shape_literal" => Style::new().fg(Color::Blue),
        "shape_match_pattern" => Style::new().fg(Color::Green),
        "shape_nothing" => Style::new().fg(Color::LightCyan),
        "shape_operator" => Style::new().fg(Color::Yellow),
        "shape_pipe" => Style::new().fg(Color::Purple).bold(),
        "shape_range" => Style::new().fg(Color::Yellow).bold(),
        "shape_raw_string" => Style::new().fg(Color::LightMagenta).bold(),
        "shape_record" => Style::new().fg(Color::Cyan).bold(),
        "shape_redirection" => Style::new().fg(Color::Purple).bold(),
        "shape_signature" => Style::new().fg(Color::Green).bold(),
        "shape_string" => Style::new().fg(Color::Green),
        "shape_string_interpolation" => Style::new().fg(Color::Cyan).bold(),
        "shape_table" => Style::new().fg(Color::Blue).bold(),
        "shape_variable" => Style::new().fg(Color::Purple),
        "shape_vardecl" => Style::new().fg(Color::Purple),
        _ => Style::default(),
    }
}

pub fn get_shape_color(shape: &str, conf: &Config) -> Style {
    match conf.color_config.get(shape) {
        Some(int_color) => {
            // Shapes do not use color_config closures, currently.
            match int_color {
                Value::Record { .. } => color_record_to_nustyle(int_color),
                Value::String { val, .. } => lookup_ansi_color_style(val),
                // Defer to the default in the event of incorrect types being given
                // (i.e. treat null, etc. as the value being unset)
                _ => default_shape_color(shape),
            }
        }
        None => default_shape_color(shape),
    }
}
