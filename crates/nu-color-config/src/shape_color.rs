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
            "flatshape-garbage" => Style::new().fg(Color::White).on(Color::Red).bold(),
            "flatshape-bool" => Style::new().fg(Color::LightCyan),
            "flatshape-int" => Style::new().fg(Color::Purple).bold(),
            "flatshape-float" => Style::new().fg(Color::Purple).bold(),
            "flatshape-range" => Style::new().fg(Color::Yellow).bold(),
            "flatshape-internalcall" => Style::new().fg(Color::Cyan).bold(),
            "flatshape-external" => Style::new().fg(Color::Cyan),
            "flatshape-externalarg" => Style::new().fg(Color::Green).bold(),
            "flatshape-literal" => Style::new().fg(Color::Blue),
            "flatshape-operator" => Style::new().fg(Color::Yellow),
            "flatshape-signature" => Style::new().fg(Color::Green).bold(),
            "flatshape-string" => Style::new().fg(Color::Green),
            "flatshape-string-interpolation" => Style::new().fg(Color::Cyan).bold(),
            "flatshape-list" => Style::new().fg(Color::Cyan).bold(),
            "flatshape-table" => Style::new().fg(Color::Blue).bold(),
            "flatshape-record" => Style::new().fg(Color::Cyan).bold(),
            "flatshape-block" => Style::new().fg(Color::Blue).bold(),
            "flatshape-filepath" => Style::new().fg(Color::Cyan),
            "flatshape-globpattern" => Style::new().fg(Color::Cyan).bold(),
            "flatshape-variable" => Style::new().fg(Color::Purple),
            "flatshape-flag" => Style::new().fg(Color::Blue).bold(),
            "flatshape-custom" => Style::new().bold(),
            "flatshape-nothing" => Style::new().fg(Color::LightCyan),
            _ => Style::default(),
        },
    }
}
