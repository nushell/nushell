mod command;
mod commands;
mod events;
mod nu_common;
mod pager;
mod views;

use std::io;

use nu_common::{collect_pipeline, has_simple_value, CtrlC};
use nu_protocol::{
    engine::{EngineState, Stack},
    PipelineData, Value,
};
use pager::{Page, Pager};
use terminal_size::{Height, Width};
use views::{InformationView, Preview, RecordView};

pub use pager::{StyleConfig, TableConfig, TableSplitLines, ViewConfig};

pub fn run_pager(
    engine_state: &EngineState,
    stack: &mut Stack,
    ctrlc: CtrlC,
    table_cfg: TableConfig,
    view_cfg: ViewConfig,
    input: PipelineData,
) -> io::Result<Option<Value>> {
    let commands = command::CommandList::new(table_cfg);

    let mut p = Pager::new(table_cfg, view_cfg.clone());

    let (columns, data) = collect_pipeline(input);

    let has_no_input = columns.is_empty() && data.is_empty();
    if has_no_input {
        let view = Some(Page::new(InformationView, true));
        return p.run(engine_state, stack, ctrlc, view, commands);
    }

    if has_simple_value(&data) {
        let text = data[0][0].into_abbreviated_string(view_cfg.config);

        let view = Some(Page::new(Preview::new(&text), true));
        return p.run(engine_state, stack, ctrlc, view, commands);
    }

    let mut view = RecordView::new(columns, data, table_cfg);

    if table_cfg.reverse {
        if let Some((Width(w), Height(h))) = terminal_size::terminal_size() {
            view.reverse(w, h);
        }
    }

    let view = Some(Page::new(view, false));
    p.run(engine_state, stack, ctrlc, view, commands)
}
