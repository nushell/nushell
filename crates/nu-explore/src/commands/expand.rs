use super::{HelpManual, Shortcode, ViewCommand};
use crate::{
    nu_common::{self, collect_input},
    views::Preview,
};
use nu_color_config::StyleComputer;
use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};
use std::{io::Result, vec};

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

    fn help(&self) -> Option<HelpManual> {
        #[rustfmt::skip]
        let shortcodes = vec![
            Shortcode::new("Up",        "",     "Moves the viewport one row up"),
            Shortcode::new("Down",      "",     "Moves the viewport one row down"),
            Shortcode::new("Left",      "",     "Moves the viewport one column left"),
            Shortcode::new("Right",     "",     "Moves the viewport one column right"),
            Shortcode::new("PgDown",    "",     "Moves the viewport one page of rows down"),
            Shortcode::new("PgUp",      "",     "Moves the cursor or viewport one page of rows up"),
            Shortcode::new("Esc",       "",     "Exits cursor mode. Exits the currently explored data."),
        ];

        Some(HelpManual {
            name: "expand",
            description:
                "View the currently selected cell's data using the `table` Nushell command",
            arguments: vec![],
            examples: vec![],
            config_options: vec![],
            input: shortcodes,
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
        let value = value
            .map(|v| convert_value_to_string(v, engine_state, stack))
            .unwrap_or_default();

        Ok(Preview::new(&value))
    }
}

fn convert_value_to_string(value: Value, engine_state: &EngineState, stack: &mut Stack) -> String {
    let (cols, vals) = collect_input(value.clone());

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
