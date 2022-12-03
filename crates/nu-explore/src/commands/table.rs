use std::io::Result;

use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};

use crate::{nu_common::collect_input, pager::TableConfig, views::RecordView};

use super::{HelpManual, ViewCommand};

#[derive(Debug, Default, Clone)]
pub struct TableCmd {
    table_cfg: TableConfig,
}

impl TableCmd {
    pub fn new(table_cfg: TableConfig) -> Self {
        Self { table_cfg }
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

    fn help(&self) -> Option<HelpManual> {
        Some(HelpManual {
            name: "table",
            description: "Display a table view",
            arguments: vec![],
            examples: vec![],
        })
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

        let (columns, data) = collect_input(value);

        let mut view = RecordView::new(columns, data, self.table_cfg);

        if is_record {
            view.transpose();
            view.show_head(false);
        }

        Ok(view)
    }
}
