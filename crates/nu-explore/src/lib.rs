mod commands;
mod default_context;
mod explore;
mod nu_common;
mod pager;
mod registry;
mod views;

use anyhow::Result;
use commands::{ExpandCmd, HelpCmd, NuCmd, QuitCmd, TableCmd, TryCmd};
pub use default_context::add_explore_context;
pub use explore::Explore;
use nu_common::{collect_pipeline, has_simple_value, CtrlC};
use nu_protocol::{
    engine::{EngineState, Stack},
    PipelineData, Value,
};
use pager::{Page, Pager, PagerConfig, StyleConfig};
use registry::CommandRegistry;
use terminal_size::{Height, Width};
use views::{BinaryView, Orientation, Preview, RecordView};

mod util {
    pub use super::nu_common::{create_lscolors, create_map, map_into_value};
}

fn run_pager(
    engine_state: &EngineState,
    stack: &mut Stack,
    ctrlc: CtrlC,
    input: PipelineData,
    config: PagerConfig,
) -> Result<Option<Value>> {
    let mut p = Pager::new(config.clone());
    let commands = create_command_registry();

    let is_record = matches!(input, PipelineData::Value(Value::Record { .. }, ..));
    let is_binary = matches!(input, PipelineData::Value(Value::Binary { .. }, ..));

    if is_binary {
        p.show_message("For help type :help");

        let view = binary_view(input);
        return p.run(engine_state, stack, ctrlc, view, commands);
    }

    let (columns, data) = collect_pipeline(input)?;

    let has_no_input = columns.is_empty() && data.is_empty();
    if has_no_input {
        return p.run(engine_state, stack, ctrlc, help_view(), commands);
    }

    p.show_message("For help type :help");

    if let Some(value) = has_simple_value(&data) {
        let text = value.to_abbreviated_string(config.nu_config);
        let view = Some(Page::new(Preview::new(&text), false));
        return p.run(engine_state, stack, ctrlc, view, commands);
    }

    let view = create_record_view(columns, data, is_record, config);
    p.run(engine_state, stack, ctrlc, view, commands)
}

fn create_record_view(
    columns: Vec<String>,
    data: Vec<Vec<Value>>,
    // wait, why would we use RecordView for something that isn't a record?
    is_record: bool,
    config: PagerConfig,
) -> Option<Page> {
    let mut view = RecordView::new(columns, data);
    if is_record {
        view.set_orientation_current(Orientation::Left);
    }

    if config.tail {
        if let Some((Width(w), Height(h))) = terminal_size::terminal_size() {
            view.tail(w, h);
        }
    }

    Some(Page::new(view, true))
}

fn help_view() -> Option<Page> {
    Some(Page::new(HelpCmd::view(), false))
}

fn binary_view(input: PipelineData) -> Option<Page> {
    let data = match input {
        PipelineData::Value(Value::Binary { val, .. }, _) => val,
        _ => unreachable!("checked beforehand"),
    };

    let view = BinaryView::new(data);

    Some(Page::new(view, true))
}

fn create_command_registry() -> CommandRegistry {
    let mut registry = CommandRegistry::new();
    create_commands(&mut registry);
    create_aliases(&mut registry);

    registry
}

fn create_commands(registry: &mut CommandRegistry) {
    registry.register_command_view(NuCmd::new(), true);
    registry.register_command_view(TableCmd::new(), true);

    registry.register_command_view(ExpandCmd::new(), false);
    registry.register_command_view(TryCmd::new(), false);
    registry.register_command_view(HelpCmd::default(), false);

    registry.register_command_reactive(QuitCmd);
}

fn create_aliases(registry: &mut CommandRegistry) {
    registry.create_aliases("h", HelpCmd::NAME);
    registry.create_aliases("e", ExpandCmd::NAME);
    registry.create_aliases("q", QuitCmd::NAME);
    registry.create_aliases("q!", QuitCmd::NAME);
}
