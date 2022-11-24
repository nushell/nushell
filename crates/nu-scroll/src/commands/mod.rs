use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};

use super::pager::{Pager, Transition};

use std::io::Result;

mod help;
mod nu;
mod quit;
mod r#try;

pub use help::HelpCmd;
pub use nu::NuCmd;
pub use quit::QuitCmd;
pub use r#try::TryCmd;

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
}

#[derive(Debug, Default, Clone)]
pub struct HelpExample {
    pub example: &'static str,
    pub description: &'static str,
}
