use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};

use super::{
    pager::{Pager, Transition},
    views::View,
};

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

pub struct HelpManual {
    name: &'static str,
    description: &'static str,
    arguments: Vec<HelpExample>,
    examples: Vec<HelpExample>,
}

pub struct HelpExample {
    example: &'static str,
    description: &'static str,
}

pub enum Command {
    Reactive(Box<dyn SimpleCommand>),
    View(Box<dyn ViewCommand<View = Box<dyn View>>>),
}
