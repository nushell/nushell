use crate::color_config::lookup_ansi_color_style;
use nu_ansi_term::{Color, Style};
use nu_protocol::Config;

pub fn get_shape_color(shape: String, conf: &Config) -> Style {
    match shape.as_ref() {
        "flatshape_garbage" => {
            if conf.color_config.contains_key("flatshape_garbage") {
                let int_color = &conf.color_config["flatshape_garbage"];
                lookup_ansi_color_style(int_color.to_string())
            } else {
                Style::new().fg(Color::White).on(Color::Red).bold()
            }
        }
        "flatshape_bool" => {
            if conf.color_config.contains_key("flatshape_bool") {
                let int_color = &conf.color_config["flatshape_bool"];
                lookup_ansi_color_style(int_color.to_string())
            } else {
                Style::new().fg(Color::LightCyan)
            }
        }
        "flatshape_int" => {
            if conf.color_config.contains_key("flatshape_int") {
                let int_color = &conf.color_config["flatshape_int"];
                lookup_ansi_color_style(int_color.to_string())
            } else {
                Style::new().fg(Color::Purple).bold()
            }
        }
        "flatshape_float" => {
            if conf.color_config.contains_key("flatshape_float") {
                let int_color = &conf.color_config["flatshape_float"];
                lookup_ansi_color_style(int_color.to_string())
            } else {
                Style::new().fg(Color::Purple).bold()
            }
        }
        "flatshape_range" => {
            if conf.color_config.contains_key("flatshape_range") {
                let int_color = &conf.color_config["flatshape_range"];
                lookup_ansi_color_style(int_color.to_string())
            } else {
                Style::new().fg(Color::Yellow).bold()
            }
        }
        "flatshape_internalcall" => {
            if conf.color_config.contains_key("flatshape_internalcall") {
                let int_color = &conf.color_config["flatshape_internalcall"];
                lookup_ansi_color_style(int_color.to_string())
            } else {
                Style::new().fg(Color::Cyan).bold()
            }
        }
        "flatshape_external" => {
            if conf.color_config.contains_key("flatshape_external") {
                let int_color = &conf.color_config["flatshape_external"];
                lookup_ansi_color_style(int_color.to_string())
            } else {
                Style::new().fg(Color::Cyan)
            }
        }
        "flatshape_externalarg" => {
            if conf.color_config.contains_key("flatshape_externalarg") {
                let int_color = &conf.color_config["flatshape_externalarg"];
                lookup_ansi_color_style(int_color.to_string())
            } else {
                Style::new().fg(Color::Green).bold()
            }
        }
        "flatshape_literal" => {
            if conf.color_config.contains_key("flatshape_literal") {
                let int_color = &conf.color_config["flatshape_literal"];
                lookup_ansi_color_style(int_color.to_string())
            } else {
                Style::new().fg(Color::Blue)
            }
        }
        "flatshape_operator" => {
            if conf.color_config.contains_key("flatshape_operator") {
                let int_color = &conf.color_config["flatshape_operator"];
                lookup_ansi_color_style(int_color.to_string())
            } else {
                Style::new().fg(Color::Yellow)
            }
        }
        "flatshape_signature" => {
            if conf.color_config.contains_key("flatshape_signature") {
                let int_color = &conf.color_config["flatshape_signature"];
                lookup_ansi_color_style(int_color.to_string())
            } else {
                Style::new().fg(Color::Green).bold()
            }
        }
        "flatshape_string" => {
            if conf.color_config.contains_key("flatshape_string") {
                let int_color = &conf.color_config["flatshape_string"];
                lookup_ansi_color_style(int_color.to_string())
            } else {
                Style::new().fg(Color::Green)
            }
        }
        "flatshape_filepath" => {
            if conf.color_config.contains_key("flatshape_filepath") {
                let int_color = &conf.color_config["flatshape_filepath"];
                lookup_ansi_color_style(int_color.to_string())
            } else {
                Style::new().fg(Color::Cyan)
            }
        }
        "flatshape_globpattern" => {
            if conf.color_config.contains_key("flatshape_globpattern") {
                let int_color = &conf.color_config["flatshape_globpattern"];
                lookup_ansi_color_style(int_color.to_string())
            } else {
                Style::new().fg(Color::Cyan).bold()
            }
        }
        "flatshape_variable" => {
            if conf.color_config.contains_key("flatshape_variable") {
                let int_color = &conf.color_config["flatshape_variable"];
                lookup_ansi_color_style(int_color.to_string())
            } else {
                Style::new().fg(Color::Purple)
            }
        }
        "flatshape_flag" => {
            if conf.color_config.contains_key("flatshape_flag") {
                let int_color = &conf.color_config["flatshape_flag"];
                lookup_ansi_color_style(int_color.to_string())
            } else {
                Style::new().fg(Color::Blue).bold()
            }
        }
        "flatshape_custom" => {
            if conf.color_config.contains_key("flatshape_custom") {
                let int_color = &conf.color_config["flatshape_custom"];
                lookup_ansi_color_style(int_color.to_string())
            } else {
                Style::new().bold()
            }
        }
        _ => Style::default(),
    }
}
