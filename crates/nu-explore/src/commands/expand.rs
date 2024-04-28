use super::ViewCommand;
use crate::{
    nu_common::{self, collect_input},
    views::Preview,
};
use anyhow::Result;
use nu_color_config::StyleComputer;
use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
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

    fn usage(&self) -> &'static str {
        ""
    }

    fn parse(&mut self, _: &str) -> Result<()> {
        Ok(())
    }

    fn spawn(
        &mut self,
        engine_state: &EngineState,
        stack: &mut Stack,
        value: Option<Value>,
    ) -> Result<Self::View> {
        let value = value
            .map(|v| convert_value_to_string(v, engine_state, stack))
            .unwrap_or_default();

        Ok(Preview::new(&value))
    }
}

fn convert_value_to_string(value: Value, engine_state: &EngineState, stack: &mut Stack) -> String {
    let (cols, vals) = collect_input(value.clone()).unwrap();

    let has_no_head = cols.is_empty() || (cols.len() == 1 && cols[0].is_empty());
    let has_single_value = vals.len() == 1 && vals[0].len() == 1;
    if !has_no_head && has_single_value {
        let config = engine_state.get_config();
        vals[0][0].to_abbreviated_string(config)
    } else {
        let ctrlc = engine_state.ctrlc.clone();
        let config = engine_state.get_config();
        let style_computer = StyleComputer::from_config(engine_state, stack);

        nu_common::try_build_table(ctrlc, config, &style_computer, value)
    }
}
