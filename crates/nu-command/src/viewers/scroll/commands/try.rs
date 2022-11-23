use std::io::{self, Result};

use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};

use crate::viewers::scroll::{
    pager::NuSpan,
    pager::{Pager, TableConfig, Transition},
    views::{RecordView, InteractiveView},
};

use super::{HelpManual, SimpleCommand, ViewCommand, HelpExample};


#[derive(Debug, Default)]
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
        Some(HelpManual {
            name: "try",
            description: "Opens a dynamic REPL to run nu commands",
            arguments: vec![],
            examples: vec![HelpExample {
                example: "try open Cargo.toml",
                description: "Optionally you can provide a command which will be run right away",
            }],
        })
    }

    fn parse(&mut self, args: &str) -> Result<()> {
        let cmd = args
            .strip_prefix(Self::NAME)
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "failed to parse"))?;

        let cmd = cmd.trim();

        self.command = cmd.to_owned();

        Ok(())
    }

    fn spawn(
        &mut self,
        _: &EngineState,
        _: &mut Stack,
        value: Option<Value>,
    ) -> Result<Self::View> {
        let value = value.unwrap_or_default();
        let mut view = InteractiveView::new(value, self.table_cfg.clone());
        view.init(self.command.clone());

        Ok(view)
    }
}