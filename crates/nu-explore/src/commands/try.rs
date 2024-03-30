use super::{default_color_list, ConfigOption, HelpExample, HelpManual, Shortcode, ViewCommand};
use crate::views::InteractiveView;
use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};
use std::io::{Error, ErrorKind, Result};

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

        #[rustfmt::skip]
        let config_options = vec![
            ConfigOption::boolean(":try options", "In the `:try` REPL, attempt to run the command on every keypress", "try.reactive"),
            ConfigOption::new(":try options", "Change a highlighted menu color", "try.highlighted_color", default_color_list()),
        ];

        #[rustfmt::skip]
        let examples = vec![
            HelpExample::new("try", "Open a interactive :try command"),
            HelpExample::new("try open Cargo.toml", "Optionally, you can provide a command which will be run immediately"),
        ];

        Some(HelpManual {
            name: "try",
            description: "Opens a panel in which to run Nushell commands and explore their output. The explorer acts like `:table`.",
            arguments: vec![],
            examples,
            input: shortcuts,
            config_options,
        })
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
    ) -> Result<Self::View> {
        let value = value.unwrap_or_default();
        let mut view = InteractiveView::new(value);
        view.init(self.command.clone());
        view.try_run(engine_state, stack)
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        Ok(view)
    }
}
