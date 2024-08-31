use super::SimpleCommand;
use crate::pager::{Pager, Transition};
use anyhow::Result;
use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};

#[derive(Default, Clone)]
pub struct QuitCmd;

impl QuitCmd {
    pub const NAME: &'static str = "quit";
}

impl SimpleCommand for QuitCmd {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn description(&self) -> &'static str {
        ""
    }

    fn parse(&mut self, _: &str) -> Result<()> {
        Ok(())
    }

    fn react(
        &mut self,
        _: &EngineState,
        _: &mut Stack,
        _: &mut Pager<'_>,
        _: Option<Value>,
    ) -> Result<Transition> {
        Ok(Transition::Exit)
    }
}
