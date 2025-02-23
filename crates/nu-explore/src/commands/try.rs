use super::ViewCommand;
use crate::views::{TryView, ViewConfig};
use anyhow::Result;
use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};

#[derive(Debug, Default, Clone)]
pub struct TryCmd {
    command: String,
}

impl TryCmd {
    pub fn new() -> Self {
        Self {
            command: String::new(),
        }
    }

    pub const NAME: &'static str = "try";
}

impl ViewCommand for TryCmd {
    type View = TryView;

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn description(&self) -> &'static str {
        ""
    }

    fn parse(&mut self, args: &str) -> Result<()> {
        args.trim().clone_into(&mut self.command);

        Ok(())
    }

    fn spawn(
        &mut self,
        engine_state: &EngineState,
        stack: &mut Stack,
        value: Option<Value>,
        config: &ViewConfig,
    ) -> Result<Self::View> {
        let value = value.unwrap_or_default();
        let mut view = TryView::new(value, config.explore_config.clone());
        view.init(self.command.clone());
        view.try_run(engine_state, stack)?;

        Ok(view)
    }
}
