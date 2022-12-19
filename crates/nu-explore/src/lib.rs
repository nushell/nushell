mod commands;
mod nu_common;
mod pager;
mod registry;
mod views;

use std::io;

use commands::{
    config::ConfigCmd, default_color_list, ConfigOption, ConfigShowCmd, ExpandCmd, HelpCmd,
    HelpManual, NuCmd, QuitCmd, TableCmd, TryCmd, TweakCmd,
};
use nu_common::{collect_pipeline, has_simple_value, CtrlC};
use nu_protocol::{
    engine::{EngineState, Stack},
    PipelineData, Value,
};
use pager::{Page, Pager};
use registry::{Command, CommandRegistry};
use terminal_size::{Height, Width};
use views::{InformationView, Orientation, Preview, RecordView};

pub use pager::{PagerConfig, StyleConfig};

pub mod util {
    pub use super::nu_common::{create_lscolors, create_map, map_into_value};
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

    let commands = create_command_registry();

    let has_no_input = columns.is_empty() && data.is_empty();
    if has_no_input {
        return p.run(engine_state, stack, ctrlc, information_view(), commands);
    }

    if config.show_banner {
        p.show_message("For help type :help");
    }

    if let Some(value) = has_simple_value(&data) {
        let text = value.into_abbreviated_string(config.nu_config);
        let view = Some(Page::new(Preview::new(&text), true));
        return p.run(engine_state, stack, ctrlc, view, commands);
    }

    let view = create_record_view(columns, data, is_record, config);
    p.run(engine_state, stack, ctrlc, view, commands)
}

fn create_record_view(
    columns: Vec<String>,
    data: Vec<Vec<Value>>,
    is_record: bool,
    config: PagerConfig,
) -> Option<Page> {
    let mut view = RecordView::new(columns, data);
    if is_record {
        view.set_orientation_current(Orientation::Left);
    }

    if config.reverse {
        if let Some((Width(w), Height(h))) = terminal_size::terminal_size() {
            view.reverse(w, h);
        }
    }

    Some(Page::new(view, false))
}

fn information_view() -> Option<Page> {
    Some(Page::new(InformationView, true))
}

pub fn create_command_registry() -> CommandRegistry {
    let mut registry = CommandRegistry::new();
    create_commands(&mut registry);
    create_aliases(&mut registry);

    // reregister help && config commands
    let commands = registry.get_commands().cloned().collect::<Vec<_>>();
    let aliases = registry.get_aliases().collect::<Vec<_>>();

    let help_cmd = create_help_command(&commands, &aliases);
    let config_cmd = create_config_command(&commands);

    registry.register_command_view(help_cmd, true);
    registry.register_command_view(config_cmd, true);

    registry
}

pub fn create_commands(registry: &mut CommandRegistry) {
    registry.register_command_view(NuCmd::new(), false);
    registry.register_command_view(TableCmd::new(), false);

    registry.register_command_view(ExpandCmd::new(), true);
    registry.register_command_view(TryCmd::new(), true);
    registry.register_command_view(ConfigShowCmd::new(), true);
    registry.register_command_view(ConfigCmd::default(), true);
    registry.register_command_view(HelpCmd::default(), true);

    registry.register_command_reactive(QuitCmd::default());
    registry.register_command_reactive(TweakCmd::default());
}

pub fn create_aliases(regestry: &mut CommandRegistry) {
    regestry.create_aliase("h", HelpCmd::NAME);
    regestry.create_aliase("e", ExpandCmd::NAME);
    regestry.create_aliase("q", QuitCmd::NAME);
    regestry.create_aliase("q!", QuitCmd::NAME);
}

#[rustfmt::skip]
fn create_config_command(commands: &[Command]) -> ConfigCmd {
    const GROUP: &str = "Explore configuration";

    let mut config = ConfigCmd::from_commands(commands.to_vec());

    config.register_group(ConfigOption::new(GROUP, "Status bar information color", "status.info", default_color_list()));
    config.register_group(ConfigOption::new(GROUP, "Status bar warning color", "status.warn", default_color_list()));
    config.register_group(ConfigOption::new(GROUP, "Status bar error color", "status.error", default_color_list()));

    config.register_group(ConfigOption::new(GROUP, "Status bar default text color", "status_bar_text", default_color_list()));
    config.register_group(ConfigOption::new(GROUP, "Status bar background", "status_bar_background", default_color_list()));

    config.register_group(ConfigOption::new(GROUP, "Command bar text color", "command_bar_text", default_color_list()));
    config.register_group(ConfigOption::new(GROUP, "Command bar background", "command_bar_background", default_color_list()));

    config.register_group(ConfigOption::new(GROUP, "Highlight color in search", "highlight", default_color_list()));

    config.register_group(ConfigOption::boolean(GROUP, "Show help banner on open", "help_banner"));
    config.register_group(ConfigOption::boolean(GROUP, "Pressing ESC causes a program exit", "exit_esc"));

    config
}

fn create_help_command(commands: &[Command], aliases: &[(&str, &str)]) -> HelpCmd {
    let help_manuals = create_help_manuals(commands);

    HelpCmd::new(help_manuals, aliases)
}

fn create_help_manuals(cmd_list: &[Command]) -> Vec<HelpManual> {
    cmd_list.iter().map(create_help_manual).collect()
}

fn create_help_manual(cmd: &Command) -> HelpManual {
    let name = match cmd {
        Command::Reactive(cmd) => cmd.name(),
        Command::View { cmd, .. } => cmd.name(),
    };

    let manual = match cmd {
        Command::Reactive(cmd) => cmd.help(),
        Command::View { cmd, .. } => cmd.help(),
    };

    __create_help_manual(manual, name)
}

fn __create_help_manual(manual: Option<HelpManual>, name: &'static str) -> HelpManual {
    match manual {
        Some(manual) => manual,
        None => HelpManual {
            name,
            description: "",
            arguments: Vec::new(),
            examples: Vec::new(),
            input: Vec::new(),
            config_options: Vec::new(),
        },
    }
}
