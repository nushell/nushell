use crate::views::ViewConfig;

use super::pager::{Pager, Transition};
use anyhow::Result;
use nu_protocol::{
    Value,
    engine::{EngineState, Stack},
};

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
pub use table::TableCmd;
pub use r#try::TryCmd;

pub trait SimpleCommand {
    fn name(&self) -> &'static str;

    fn description(&self) -> &'static str;

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

    fn description(&self) -> &'static str;

    fn parse(&mut self, args: &str) -> Result<()>;

    fn spawn(
        &mut self,
        engine_state: &EngineState,
        stack: &mut Stack,
        value: Option<Value>,
        config: &ViewConfig,
    ) -> Result<Self::View>;
}
