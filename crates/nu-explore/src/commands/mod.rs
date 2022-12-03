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
