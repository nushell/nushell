use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};
use std::io::Result;

use crate::{
    nu_common::{self, collect_input},
    views::Preview,
};

use super::{HelpManual, ViewCommand};

#[derive(Default, Clone)]
pub struct PreviewCmd;

impl PreviewCmd {
    pub fn new() -> Self {
        Self
    }
}

impl PreviewCmd {
    pub const NAME: &'static str = "preview";
}

impl ViewCommand for PreviewCmd {
    type View = Preview;

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn usage(&self) -> &'static str {
        ""
    }

    fn help(&self) -> Option<HelpManual> {
        Some(HelpManual {
            name: "preview",
            description:
                "View the currently selected cell's data using the `table` Nushell command",
            arguments: vec![],
            examples: vec![],
        })
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
        let value = match value {
            Some(value) => {
                let (cols, vals) = collect_input(value.clone());

                let has_no_head = cols.is_empty() || (cols.len() == 1 && cols[0].is_empty());
                let has_single_value = vals.len() == 1 && vals[0].len() == 1;
                if !has_no_head && has_single_value {
                    let config = engine_state.get_config();
                    vals[0][0].into_abbreviated_string(config)
                } else {
                    let ctrlc = engine_state.ctrlc.clone();
                    let config = engine_state.get_config();

                    nu_common::try_build_table(engine_state, stack, ctrlc, config, value)
                }
            }
            None => String::new(),
        };

        Ok(Preview::new(&value))
    }
}
