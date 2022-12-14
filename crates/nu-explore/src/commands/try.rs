use std::io::{Error, ErrorKind, Result};

use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};

use crate::views::InteractiveView;

use super::{default_color_list, ConfigOption, HelpExample, HelpManual, Shortcode, ViewCommand};

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

        let config_options = vec![
            ConfigOption::boolean(
                "try",
                "Try makes running command on each input character",
                "try.reactive",
            ),
            ConfigOption::new(
                "try",
                "Change a border color of the menus",
                "try.border_color",
                default_color_list(),
            ),
            ConfigOption::new(
                "try",
                "Change a highlighed menu color",
                "try.highlighted_color",
                default_color_list(),
            ),
        ];

        Some(HelpManual {
            name: "try",
            description: "Opens a panel in which to run Nushell commands and explore their output. The exporer acts liek `:table`.",
            arguments: vec![],
            examples: vec![HelpExample::new("try open Cargo.toml", "Optionally, you can provide a command which will be run immediately")],
            input: shortcuts,
            config_options,
        })
    }

    fn display_config_option(&mut self, _: String, _: String, _: String) -> bool {
        false
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
