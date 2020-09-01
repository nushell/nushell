pub use nu_data::config::NuConfig;
use nu_data::primitive::lookup_ansi_color_style;
use nu_protocol::{UntaggedValue, Value};
use nu_source::Tag;
use nu_table::TextStyle;
use std::fmt::Debug;

pub trait ConfigExtensions: Debug + Send {
    fn table_mode(&self) -> nu_table::Theme;
    fn disabled_indexes(&self) -> bool;
    fn header_style(&self) -> TextStyle;
}

pub fn header_alignment_from_value(align_value: Option<&Value>) -> nu_table::Alignment {
    match align_value {
        Some(v) => match v
            .as_string()
            .unwrap_or_else(|_| "none".to_string())
            .as_ref()
        {
            "l" | "left" => nu_table::Alignment::Left,
            "c" | "center" => nu_table::Alignment::Center,
            "r" | "right" => nu_table::Alignment::Right,
            _ => nu_table::Alignment::Center,
        },
        _ => nu_table::Alignment::Center,
    }
}

pub fn get_color_from_key_and_subkey(config: &NuConfig, key: &str, subkey: &str) -> Value {
    let vars = config.vars.lock();

    let mut v: Value =
        UntaggedValue::Primitive(nu_protocol::Primitive::String("nocolor".to_string()))
            .into_value(Tag::unknown());
    if let Some(config_vars) = vars.get(key) {
        for (kee, value) in config_vars.row_entries() {
            if kee == subkey {
                v = value.to_owned();
            }
        }
    }

    v
}

pub fn header_bold_from_value(bold_value: Option<&Value>) -> bool {
    bold_value
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
    fn header_style(&self) -> TextStyle {
        // FIXME: I agree, this is the long way around, please suggest and alternative.
        let head_color = get_color_from_key_and_subkey(self, "color_config", "header_color");
        let head_color_style = lookup_ansi_color_style(
            head_color
                .as_string()
                .unwrap_or_else(|_| "green".to_string()),
        );
        let head_bold = get_color_from_key_and_subkey(self, "color_config", "header_bold");
        let head_bold_bool = header_bold_from_value(Some(&head_bold));
        let head_align = get_color_from_key_and_subkey(self, "color_config", "header_align");
        let head_alignment = header_alignment_from_value(Some(&head_align));

        TextStyle::new()
            .alignment(head_alignment)
            .bold(Some(head_bold_bool))
            .fg(head_color_style
                .foreground
                .unwrap_or(ansi_term::Color::Green))
    }

    fn table_mode(&self) -> nu_table::Theme {
        table_mode(self)
    }

    fn disabled_indexes(&self) -> bool {
        disabled_indexes(self)
    }
}
