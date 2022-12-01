use std::io::{self, Result};

use nu_protocol::{
    engine::{EngineState, Stack},
    PipelineData, Value,
};

use crate::{
    nu_common::{collect_pipeline, has_simple_value, is_ignored_command, run_nu_command},
    pager::TableConfig,
    views::{Preview, RecordView, View},
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
    type View = NuView<'static>;

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
        self.command = args.trim().to_owned();

        Ok(())
    }

    fn spawn(
        &mut self,
        engine_state: &EngineState,
        stack: &mut Stack,
        value: Option<Value>,
    ) -> Result<Self::View> {
        if is_ignored_command(&self.command) {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "The command is ignored",
            ));
        }

        let value = value.unwrap_or_default();

        let pipeline = PipelineData::Value(value, None);
        let pipeline = run_nu_command(engine_state, stack, &self.command, pipeline)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let (columns, values) = collect_pipeline(pipeline);

        if has_simple_value(&values) {
            let config = &engine_state.config;
            let text = values[0][0].into_abbreviated_string(config);
            return Ok(NuView::Preview(Preview::new(&text)));
        }

        let view = RecordView::new(columns, values, self.table_cfg);

        Ok(NuView::Records(view))
    }
}

pub enum NuView<'a> {
    Records(RecordView<'a>),
    Preview(Preview),
}

impl View for NuView<'_> {
    fn draw(
        &mut self,
        f: &mut crate::pager::Frame,
        area: tui::layout::Rect,
        cfg: &crate::ViewConfig,
        layout: &mut crate::views::Layout,
    ) {
        match self {
            NuView::Records(v) => v.draw(f, area, cfg, layout),
            NuView::Preview(v) => v.draw(f, area, cfg, layout),
        }
    }

    fn handle_input(
        &mut self,
        engine_state: &EngineState,
        stack: &mut Stack,
        layout: &crate::views::Layout,
        info: &mut crate::pager::ViewInfo,
        key: crossterm::event::KeyEvent,
    ) -> Option<crate::pager::Transition> {
        match self {
            NuView::Records(v) => v.handle_input(engine_state, stack, layout, info, key),
            NuView::Preview(v) => v.handle_input(engine_state, stack, layout, info, key),
        }
    }

    fn show_data(&mut self, i: usize) -> bool {
        match self {
            NuView::Records(v) => v.show_data(i),
            NuView::Preview(v) => v.show_data(i),
        }
    }

    fn collect_data(&self) -> Vec<crate::nu_common::NuText> {
        match self {
            NuView::Records(v) => v.collect_data(),
            NuView::Preview(v) => v.collect_data(),
        }
    }

    fn exit(&mut self) -> Option<Value> {
        match self {
            NuView::Records(v) => v.exit(),
            NuView::Preview(v) => v.exit(),
        }
    }
}
