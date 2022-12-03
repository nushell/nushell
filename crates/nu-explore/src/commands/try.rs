use std::io::Result;

use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};

use crate::{pager::TableConfig, views::InteractiveView};

use super::{HelpExample, HelpManual, Shortcode, ViewCommand};

#[derive(Debug, Default, Clone)]
pub struct TryCmd {
    command: String,
    table_cfg: TableConfig,
}

impl TryCmd {
    pub fn new(table_cfg: TableConfig) -> Self {
        Self {
            command: String::new(),
            table_cfg,
        }
    }

    pub const NAME: &'static str = "try";
}

impl ViewCommand for TryCmd {
    type View = InteractiveView<'static>;

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn usage(&self) -> &'static str {
        ""
    }

    fn help(&self) -> Option<HelpManual> {
        #[rustfmt::skip]
        let shortcuts = vec![
            Shortcode::new("Up",     "",        "Switches between input and a output panes"),
            Shortcode::new("Down",   "",        "Switches between input and a output panes"),
            Shortcode::new("Esc",    "",        "Switches between input and a output panes"),
            Shortcode::new("Tab",    "",        "Switches between input and a output panes"),
        ];

        Some(HelpManual {
            name: "try",
            description: "Opens a panel in which to run Nushell commands and explore their output. The exporer acts liek `:table`.",
            arguments: vec![],
            examples: vec![HelpExample {
                example: "try open Cargo.toml",
                description: "Optionally, you can provide a command which will be run immediately",
            }],
            input: shortcuts,
        })
    }

    fn parse(&mut self, args: &str) -> Result<()> {
        self.command = args.trim().to_owned();

        Ok(())
    }

    fn spawn(
        &mut self,
        _: &EngineState,
        _: &mut Stack,
        value: Option<Value>,
    ) -> Result<Self::View> {
        let value = value.unwrap_or_default();
        let mut view = InteractiveView::new(value, self.table_cfg);
        view.init(self.command.clone());

        Ok(view)
    }
}
