use super::{HelpManual, SimpleCommand};
use crate::pager::{Pager, Transition};
use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};
use std::io::Result;

#[derive(Default, Clone)]
pub struct QuitCmd;

impl QuitCmd {
    pub const NAME: &'static str = "quit";
}

impl SimpleCommand for QuitCmd {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn usage(&self) -> &'static str {
        ""
    }

    fn help(&self) -> Option<HelpManual> {
        Some(HelpManual {
            name: "quit",
            description: "Quit and return to Nushell",
            arguments: vec![],
            examples: vec![],
            input: vec![],
            config_options: vec![],
        })
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
