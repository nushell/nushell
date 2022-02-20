use crate::color_config::lookup_ansi_color_style;
use nu_ansi_term::{Color, Style};
use nu_protocol::Config;

pub fn get_shape_color(shape: String, conf: &Config) -> Style {
    match conf.color_config.get(shape.as_str()) {
        Some(int_color) => match int_color.as_string() {
            Ok(int_color) => lookup_ansi_color_style(&int_color),
            Err(_) => Style::default(),
        },
        None => match shape.as_ref() {
            "flatshape_garbage" => Style::new().fg(Color::White).on(Color::Red).bold(),
            "flatshape_bool" => Style::new().fg(Color::LightCyan),
            "flatshape_int" => Style::new().fg(Color::Purple).bold(),
            "flatshape_float" => Style::new().fg(Color::Purple).bold(),
            "flatshape_range" => Style::new().fg(Color::Yellow).bold(),
            "flatshape_internalcall" => Style::new().fg(Color::Cyan).bold(),
            "flatshape_external" => Style::new().fg(Color::Cyan),
            "flatshape_externalarg" => Style::new().fg(Color::Green).bold(),
            "flatshape_literal" => Style::new().fg(Color::Blue),
            "flatshape_operator" => Style::new().fg(Color::Yellow),
            "flatshape_signature" => Style::new().fg(Color::Green).bold(),
            "flatshape_string" => Style::new().fg(Color::Green),
            "flatshape_string_interpolation" => Style::new().fg(Color::Cyan).bold(),
            "flatshape_list" => Style::new().fg(Color::Cyan).bold(),
            "flatshape_table" => Style::new().fg(Color::Blue).bold(),
            "flatshape_record" => Style::new().fg(Color::Cyan).bold(),
            "flatshape_block" => Style::new().fg(Color::Blue).bold(),
            "flatshape_filepath" => Style::new().fg(Color::Cyan),
            "flatshape_globpattern" => Style::new().fg(Color::Cyan).bold(),
            "flatshape_variable" => Style::new().fg(Color::Purple),
            "flatshape_flag" => Style::new().fg(Color::Blue).bold(),
            "flatshape_custom" => Style::new().fg(Color::Green),
            "flatshape_nothing" => Style::new().fg(Color::LightCyan),
            _ => Style::default(),
        },
    }
}
