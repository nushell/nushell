use super::ViewCommand;
use crate::{
    nu_common::{collect_pipeline, has_simple_value, run_command_with_value},
    pager::Frame,
    views::{Layout, Orientation, Preview, RecordView, View, ViewConfig},
};
use anyhow::Result;
use nu_protocol::{
    engine::{EngineState, Stack},
    PipelineData, Value,
};
use ratatui::layout::Rect;

#[derive(Debug, Default, Clone)]
pub struct NuCmd {
    command: String,
}

impl NuCmd {
    pub fn new() -> Self {
        Self {
            command: String::new(),
        }
    }

    pub const NAME: &'static str = "nu";
}

impl ViewCommand for NuCmd {
    type View = NuView;

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn description(&self) -> &'static str {
        ""
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
        config: &ViewConfig,
    ) -> Result<Self::View> {
        let value = value.unwrap_or_default();

        let pipeline = run_command_with_value(&self.command, &value, engine_state, stack)?;

        let is_record = matches!(pipeline, PipelineData::Value(Value::Record { .. }, ..));

        let (columns, values) = collect_pipeline(pipeline)?;

        if let Some(value) = has_simple_value(&values) {
            let text = value.to_abbreviated_string(&engine_state.config);
            return Ok(NuView::Preview(Preview::new(&text)));
        }

        let mut view = RecordView::new(columns, values, config.explore_config.clone());

        if is_record {
            view.set_top_layer_orientation(Orientation::Left);
        }

        Ok(NuView::Records(view))
    }
}

pub enum NuView {
    Records(RecordView),
    Preview(Preview),
}

impl View for NuView {
    fn draw(&mut self, f: &mut Frame, area: Rect, cfg: ViewConfig<'_>, layout: &mut Layout) {
        match self {
            NuView::Records(v) => v.draw(f, area, cfg, layout),
            NuView::Preview(v) => v.draw(f, area, cfg, layout),
        }
    }

    fn handle_input(
        &mut self,
        engine_state: &EngineState,
        stack: &mut Stack,
        layout: &Layout,
        info: &mut crate::pager::ViewInfo,
        key: crossterm::event::KeyEvent,
    ) -> crate::pager::Transition {
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
