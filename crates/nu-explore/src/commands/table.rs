use super::ViewCommand;
use crate::{
    nu_common::collect_input,
    views::{Orientation, RecordView},
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
    type View = RecordView<'static>;

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn usage(&self) -> &'static str {
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
    ) -> Result<Self::View> {
        let value = value.unwrap_or_default();
        let is_record = matches!(value, Value::Record { .. });

        let (columns, data) = collect_input(value)?;

        let mut view = RecordView::new(columns, data);

        // todo: use setup instead ????

        if is_record {
            view.set_orientation_current(Orientation::Left);
        }

        if let Some(o) = self.settings.orientation {
            view.set_orientation_current(o);
        }

        // if let Some(style) = self.settings.selected_cell_s {
        //     view.set_style_selected_cell(style);
        // }

        // if let Some(style) = self.settings.selected_column_s {
        //     view.set_style_selected_column(style);
        // }

        // if let Some(style) = self.settings.selected_row_s {
        //     view.set_style_selected_row(style);
        // }

        // if let Some(style) = self.settings.split_line_s {
        //     view.set_style_separator(style);
        // }

        // if let Some(p) = self.settings.padding_column_left {
        //     let c = view.get_padding_column();
        //     view.set_padding_column((p, c.1))
        // }

        // if let Some(p) = self.settings.padding_column_right {
        //     let c = view.get_padding_column();
        //     view.set_padding_column((c.0, p))
        // }

        if self.settings.turn_on_cursor_mode {
            view.set_cursor_mode();
        }

        Ok(view)
    }
}
