use std::io::{Error, ErrorKind, Result};

use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};

use crate::views::InteractiveView;

use super::{HelpExample, HelpManual, Shortcode, ViewCommand};

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
        engine_state: &EngineState,
        stack: &mut Stack,
        value: Option<Value>,
    ) -> Result<Self::View> {
        let value = value.unwrap_or_default();
        let mut view = InteractiveView::new(value);
        view.init(self.command.clone());
        view.try_run(engine_state, stack)
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        Ok(view)
    }
}
