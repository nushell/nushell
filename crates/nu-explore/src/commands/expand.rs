use super::ViewCommand;
use crate::views::{Preview, ViewConfig};
use anyhow::Result;
use nu_protocol::{
    Value,
    engine::{EngineState, Stack},
};

#[derive(Default, Clone)]
pub struct ExpandCmd;

impl ExpandCmd {
    pub fn new() -> Self {
        Self
    }
}

impl ExpandCmd {
    pub const NAME: &'static str = "expand";
}

impl ViewCommand for ExpandCmd {
    type View = Preview;

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn description(&self) -> &'static str {
        ""
    }

    fn parse(&mut self, _: &str) -> Result<()> {
        Ok(())
    }

    fn spawn(
        &mut self,
        _: &EngineState,
        _: &mut Stack,
        value: Option<Value>,
        _: &ViewConfig,
    ) -> Result<Self::View> {
        if let Some(value) = value {
            Ok(Preview::new(value))
        } else {
            Ok(Preview::empty())
        }
    }
}
