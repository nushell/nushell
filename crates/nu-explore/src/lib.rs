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
use views::{InformationView, Orientation, Preview, RecordView};

pub use pager::{PagerConfig, StyleConfig};

pub mod util {
    pub use super::nu_common::{create_map, map_into_value};
}

pub fn run_pager(
    engine_state: &EngineState,
    stack: &mut Stack,
    ctrlc: CtrlC,
    input: PipelineData,
    config: PagerConfig,
) -> io::Result<Option<Value>> {
    let mut p = Pager::new(config.clone());

    let is_record = matches!(input, PipelineData::Value(Value::Record { .. }, ..));
    let (columns, data) = collect_pipeline(input);

    let commands = command::CommandList::new();

    let has_no_input = columns.is_empty() && data.is_empty();
    if has_no_input {
        let view = Some(Page::new(InformationView, true));
        return p.run(engine_state, stack, ctrlc, view, commands);
    }

    if config.show_banner {
        p.show_message("For help type :help");
    }

    if has_simple_value(&data) {
        let text = data[0][0].into_abbreviated_string(config.nu_config);

        let view = Some(Page::new(Preview::new(&text), true));
        return p.run(engine_state, stack, ctrlc, view, commands);
    }

    let mut view = RecordView::new(columns, data);

    if is_record {
        view.set_orientation_current(Orientation::Left);
    }

    if config.reverse {
        if let Some((Width(w), Height(h))) = terminal_size::terminal_size() {
            view.reverse(w, h);
        }
    }

    let view = Some(Page::new(view, false));
    p.run(engine_state, stack, ctrlc, view, commands)
}
