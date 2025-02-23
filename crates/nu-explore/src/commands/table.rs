use super::ViewCommand;
use crate::{
    nu_common::collect_input,
    views::{Orientation, RecordView, ViewConfig},
};
use anyhow::Result;
use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};

#[derive(Debug, Default, Clone)]
pub struct TableCmd {
    // todo: add arguments to override config right from CMD
    settings: TableSettings,
}

#[derive(Debug, Default, Clone)]
struct TableSettings {
    orientation: Option<Orientation>,
    turn_on_cursor_mode: bool,
}

impl TableCmd {
    pub fn new() -> Self {
        Self::default()
    }

    pub const NAME: &'static str = "table";
}

impl ViewCommand for TableCmd {
    type View = RecordView;

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn description(&self) -> &'static str {
        ""
    }

    fn parse(&mut self, _: &str) -> Result<()> {
        Ok(())
    }

    fn spawn(
        &mut self,
        _: &EngineState,
        _: &mut Stack,
        value: Option<Value>,
        config: &ViewConfig,
    ) -> Result<Self::View> {
        let value = value.unwrap_or_default();
        let is_record = matches!(value, Value::Record { .. });

        let (columns, data) = collect_input(value)?;

        let mut view = RecordView::new(columns, data, config.explore_config.clone());

        if is_record {
            view.set_top_layer_orientation(Orientation::Left);
        }

        if let Some(o) = self.settings.orientation {
            view.set_top_layer_orientation(o);
        }

        if self.settings.turn_on_cursor_mode {
            view.set_cursor_mode();
        }

        Ok(view)
    }
}
