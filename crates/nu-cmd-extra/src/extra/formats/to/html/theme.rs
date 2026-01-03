use std::borrow::Cow;

use nu_protocol::IntoValue;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, IntoValue)]
#[serde(rename_all = "camelCase")]
#[nu_value(rename_all = "camelCase")]
pub struct HtmlTheme {
    pub name: Cow<'static, str>,
    pub black: Cow<'static, str>,
    pub red: Cow<'static, str>,
    pub green: Cow<'static, str>,
    pub yellow: Cow<'static, str>,
    pub blue: Cow<'static, str>,
    pub purple: Cow<'static, str>,
    pub cyan: Cow<'static, str>,
    pub white: Cow<'static, str>,
    pub bright_black: Cow<'static, str>,
    pub bright_red: Cow<'static, str>,
    pub bright_green: Cow<'static, str>,
    pub bright_yellow: Cow<'static, str>,
    pub bright_blue: Cow<'static, str>,
    pub bright_purple: Cow<'static, str>,
    pub bright_cyan: Cow<'static, str>,
    pub bright_white: Cow<'static, str>,
    pub background: Cow<'static, str>,
    pub foreground: Cow<'static, str>,
}

impl Default for HtmlTheme {
    fn default() -> Self {
        HtmlTheme {
            name: "nu_default".into(),
            black: "black".into(),
            red: "red".into(),
            green: "green".into(),
            yellow: "#717100".into(),
            blue: "blue".into(),
            purple: "#c800c8".into(),
            cyan: "#037979".into(),
            white: "white".into(),
            bright_black: "black".into(),
            bright_red: "red".into(),
            bright_green: "green".into(),
            bright_yellow: "#717100".into(),
            bright_blue: "blue".into(),
            bright_purple: "#c800c8".into(),
            bright_cyan: "#037979".into(),
            bright_white: "white".into(),
            background: "white".into(),
            foreground: "black".into(),
        }
    }
}
