mod command;
mod commands;
mod events;
mod nu_common;
mod pager;
mod views;

use std::io;

use nu_common::{collect_pipeline, CtrlC};
use nu_protocol::{
    engine::{EngineState, Stack},
    PipelineData, Value,
};
use pager::{Page, Pager};
use views::{InformationView, RecordView};

pub use pager::{StyleConfig, TableConfig, TableSplitLines, ViewConfig};

pub fn run_pager(
    engine_state: &EngineState,
    stack: &mut Stack,
    ctrlc: CtrlC,
    table_cfg: TableConfig,
    view_cfg: ViewConfig,
    input: PipelineData,
) -> io::Result<Option<Value>> {
    let (columns, data) = collect_pipeline(input);

    let commands = command::CommandList::new(&table_cfg);

    let mut p = Pager::new(table_cfg.clone(), view_cfg.clone());

    if columns.is_empty() && data.is_empty() {
        return p.run(
            engine_state,
            stack,
            ctrlc,
            Some(Page::new(InformationView, true)),
            commands,
        );
    }

    let mut view = RecordView::new(columns, data, table_cfg.clone());

    if table_cfg.reverse {
        if let Some((terminal_size::Width(w), terminal_size::Height(h))) =
            terminal_size::terminal_size()
        {
            view.reverse(w, h);
        }
    }

    p.run(
        engine_state,
        stack,
        ctrlc,
        Some(Page::new(view, false)),
        commands,
    )
}
