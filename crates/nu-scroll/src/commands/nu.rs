use std::io::{self, Result};

use nu_protocol::{
    engine::{EngineState, Stack},
    PipelineData, Value,
};

use crate::{
    nu_common::{collect_pipeline, run_nu_command},
    pager::TableConfig,
    views::RecordView,
};

use super::{HelpExample, HelpManual, ViewCommand};

#[derive(Debug, Default, Clone)]
pub struct NuCmd {
    command: String,
    table_cfg: TableConfig,
}

impl NuCmd {
    pub fn new(table_cfg: TableConfig) -> Self {
        Self {
            command: String::new(),
            table_cfg,
        }
    }

    pub const NAME: &'static str = "nu";
}

impl ViewCommand for NuCmd {
    type View = RecordView<'static>;

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn usage(&self) -> &'static str {
        ""
    }

    fn help(&self) -> Option<HelpManual> {
        Some(HelpManual {
            name: "nu",
            description: "Run a nu command. You can use a presented table as an input",
            arguments: vec![],
            examples: vec![
                HelpExample {
                    example: "where type == 'file'",
                    description: "Filter data to get only entries with a type being a 'file'",
                },
                HelpExample {
                    example: "get scope | get examples",
                    description: "Get a inner values",
                },
                HelpExample {
                    example: "open Cargo.toml",
                    description: "Open a Cargo.toml file",
                },
            ],
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
        engine_state: &EngineState,
        stack: &mut Stack,
        value: Option<Value>,
    ) -> Result<Self::View> {
        let value = value.unwrap_or_default();

        let pipeline = PipelineData::Value(value, None);
        let pipeline = run_nu_command(engine_state, stack, &self.command, pipeline)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let (columns, values) = collect_pipeline(pipeline);

        let view = RecordView::new(columns, values, self.table_cfg.clone());

        Ok(view)
    }
}
