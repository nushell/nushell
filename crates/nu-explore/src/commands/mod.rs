use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};

use super::pager::{Pager, Transition};

use std::io::Result;

mod expand;
mod help;
mod nu;
mod quit;
mod table;
mod r#try;
mod tweak;

pub mod config;
mod config_show;

pub use config_show::ConfigShowCmd;
pub use expand::ExpandCmd;
pub use help::HelpCmd;
pub use nu::NuCmd;
pub use quit::QuitCmd;
pub use r#try::TryCmd;
pub use table::TableCmd;
pub use tweak::TweakCmd;

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

    fn get_config_settings(&self) -> Vec<ConfigOption>;

    fn set_config_settings(&mut self, group: String, key: String, value: String);

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
    // todo: add config settings options
    // pub config_options: Vec<HelpExample>,
    pub input: Vec<Shortcode>,
}

#[derive(Debug, Default, Clone)]
pub struct HelpExample {
    pub example: &'static str,
    pub description: &'static str,
}

impl HelpExample {
    pub fn new(example: &'static str, description: &'static str) -> Self {
        Self {
            example,
            description,
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
            values: vec![HelpExample::new("true", ""), HelpExample::new("false", "")],
        }
    }
}
