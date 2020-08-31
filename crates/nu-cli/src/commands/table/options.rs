pub use nu_data::config::NuConfig;
use std::fmt::Debug;

pub trait ConfigExtensions: Debug + Send {
    fn header_alignment(&self) -> nu_table::Alignment;
    fn header_color(&self) -> Option<ansi_term::Color>;
    fn header_bold(&self) -> bool;
    fn table_mode(&self) -> nu_table::Theme;
    fn disabled_indexes(&self) -> bool;
    fn text_color(&self) -> Option<ansi_term::Color>;
    fn line_color(&self) -> Option<ansi_term::Color>;
}

pub fn header_alignment(config: &NuConfig) -> nu_table::Alignment {
    let vars = config.vars.lock();

    let alignment = vars.get("header_align");

    if alignment.is_none() {
        return nu_table::Alignment::Center;
    }

    alignment.map_or(nu_table::Alignment::Left, |a| {
        a.as_string().map_or(nu_table::Alignment::Center, |a| {
            match a.to_lowercase().as_str() {
                "center" | "c" => nu_table::Alignment::Center,
                "right" | "r" => nu_table::Alignment::Right,
                _ => nu_table::Alignment::Center,
            }
        })
    })
}

pub fn get_color_for_config_key(config: &NuConfig, key: &str) -> Option<ansi_term::Color> {
    let vars = config.vars.lock();

    Some(match vars.get(key) {
        Some(c) => match c.as_string() {
            Ok(color) => match color.to_lowercase().as_str() {
                "g" | "green" => ansi_term::Color::Green,
                "r" | "red" => ansi_term::Color::Red,
                "u" | "blue" => ansi_term::Color::Blue,
                "b" | "black" => ansi_term::Color::Black,
                "y" | "yellow" => ansi_term::Color::Yellow,
                "p" | "purple" => ansi_term::Color::Purple,
                "c" | "cyan" => ansi_term::Color::Cyan,
                "w" | "white" => ansi_term::Color::White,
                _ => ansi_term::Color::Green,
            },
            _ => ansi_term::Color::Green,
        },
        _ => ansi_term::Color::Green,
    })
}

pub fn header_bold(config: &NuConfig) -> bool {
    let vars = config.vars.lock();

    vars.get("header_bold")
        .map(|x| x.as_bool().unwrap_or(true))
        .unwrap_or(true)
}

pub fn table_mode(config: &NuConfig) -> nu_table::Theme {
    let vars = config.vars.lock();

    vars.get("table_mode")
        .map_or(nu_table::Theme::compact(), |mode| match mode.as_string() {
            Ok(m) if m == "basic" => nu_table::Theme::basic(),
            Ok(m) if m == "compact" => nu_table::Theme::compact(),
            Ok(m) if m == "light" => nu_table::Theme::light(),
            Ok(m) if m == "thin" => nu_table::Theme::thin(),
            Ok(m) if m == "with_love" => nu_table::Theme::with_love(),
            Ok(m) if m == "compact_double" => nu_table::Theme::compact_double(),
            _ => nu_table::Theme::compact(),
        })
}

pub fn disabled_indexes(config: &NuConfig) -> bool {
    let vars = config.vars.lock();

    vars.get("disable_table_indexes")
        .map_or(false, |x| x.as_bool().unwrap_or(false))
}

impl ConfigExtensions for NuConfig {
    fn header_alignment(&self) -> nu_table::Alignment {
        header_alignment(self)
    }

    fn header_color(&self) -> Option<ansi_term::Color> {
        get_color_for_config_key(self, "header_color")
    }

    fn text_color(&self) -> Option<ansi_term::Color> {
        get_color_for_config_key(self, "text_color")
    }

    fn line_color(&self) -> Option<ansi_term::Color> {
        get_color_for_config_key(self, "line_color")
    }

    fn header_bold(&self) -> bool {
        header_bold(self)
    }

    fn table_mode(&self) -> nu_table::Theme {
        table_mode(self)
    }

    fn disabled_indexes(&self) -> bool {
        disabled_indexes(self)
    }
}
