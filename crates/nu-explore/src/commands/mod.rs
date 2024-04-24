use super::pager::{Pager, Transition};
use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};
use std::{borrow::Cow, io::Result};

mod expand;
mod help;
mod nu;
mod quit;
mod table;
mod r#try;

pub use expand::ExpandCmd;
pub use help::HelpCmd;
pub use nu::NuCmd;
pub use quit::QuitCmd;
pub use r#try::TryCmd;
pub use table::TableCmd;

pub trait SimpleCommand {
    fn name(&self) -> &'static str;

    fn usage(&self) -> &'static str;

    fn help(&self) -> Option<HelpManual>;

    fn parse(&mut self, args: &str) -> Result<()>;

    fn react(
        &mut self,
        engine_state: &EngineState,
        stack: &mut Stack,
        pager: &mut Pager<'_>,
        value: Option<Value>,
    ) -> Result<Transition>;
}

pub trait ViewCommand {
    type View;

    fn name(&self) -> &'static str;

    fn usage(&self) -> &'static str;

    fn help(&self) -> Option<HelpManual>;

    fn parse(&mut self, args: &str) -> Result<()>;

    fn spawn(
        &mut self,
        engine_state: &EngineState,
        stack: &mut Stack,
        value: Option<Value>,
    ) -> Result<Self::View>;
}

#[derive(Debug, Default, Clone)]
pub struct HelpManual {
    pub name: &'static str,
    pub description: &'static str,
    pub arguments: Vec<HelpExample>,
    pub examples: Vec<HelpExample>,
    pub config_options: Vec<ConfigOption>,
    pub input: Vec<Shortcode>,
}

#[derive(Debug, Default, Clone)]
pub struct HelpExample {
    pub example: Cow<'static, str>,
    pub description: Cow<'static, str>,
}

impl HelpExample {
    pub fn new(
        example: impl Into<Cow<'static, str>>,
        description: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            example: example.into(),
            description: description.into(),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Shortcode {
    pub code: &'static str,
    pub context: &'static str,
    pub description: &'static str,
}

impl Shortcode {
    pub fn new(code: &'static str, context: &'static str, description: &'static str) -> Self {
        Self {
            code,
            context,
            description,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct ConfigOption {
    pub group: String,
    pub description: String,
    pub key: String,
    pub values: Vec<HelpExample>,
}

impl ConfigOption {
    pub fn new<N, D, K>(group: N, description: D, key: K, values: Vec<HelpExample>) -> Self
    where
        N: Into<String>,
        D: Into<String>,
        K: Into<String>,
    {
        Self {
            group: group.into(),
            description: description.into(),
            key: key.into(),
            values,
        }
    }

    pub fn boolean<N, D, K>(group: N, description: D, key: K) -> Self
    where
        N: Into<String>,
        D: Into<String>,
        K: Into<String>,
    {
        Self {
            group: group.into(),
            description: description.into(),
            key: key.into(),
            values: vec![
                HelpExample::new("true", "Turn the flag on"),
                HelpExample::new("false", "Turn the flag on"),
            ],
        }
    }
}

#[rustfmt::skip]
pub fn default_color_list() -> Vec<HelpExample> {
    vec![
        HelpExample::new("red",                   "Red foreground"),
        HelpExample::new("blue",                  "Blue foreground"),
        HelpExample::new("green",                 "Green foreground"),
        HelpExample::new("yellow",                "Yellow foreground"),
        HelpExample::new("magenta",               "Magenta foreground"),
        HelpExample::new("black",                 "Black foreground"),
        HelpExample::new("white",                 "White foreground"),
        HelpExample::new("#AA4433",               "#AA4433 HEX foreground"),
        HelpExample::new(r#"{bg: "red"}"#,        "Red background"),
        HelpExample::new(r#"{bg: "blue"}"#,       "Blue background"),
        HelpExample::new(r#"{bg: "green"}"#,      "Green background"),
        HelpExample::new(r#"{bg: "yellow"}"#,     "Yellow background"),
        HelpExample::new(r#"{bg: "magenta"}"#,    "Magenta background"),
        HelpExample::new(r#"{bg: "black"}"#,      "Black background"),
        HelpExample::new(r#"{bg: "white"}"#,      "White background"),
        HelpExample::new(r##"{bg: "#AA4433"}"##,  "#AA4433 HEX background"),
    ]
}

pub fn default_int_list() -> Vec<HelpExample> {
    (0..20)
        .map(|i| HelpExample::new(i.to_string(), format!("A value equal to {i}")))
        .collect()
}
