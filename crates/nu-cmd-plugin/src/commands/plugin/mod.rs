use nu_engine::{command_prelude::*, get_full_help};

mod add;
mod list;
mod rm;
mod stop;
mod use_;

pub use add::PluginAdd;
pub use list::PluginList;
pub use rm::PluginRm;
pub use stop::PluginStop;
pub use use_::PluginUse;

#[derive(Clone)]
pub struct PluginCommand;

impl Command for PluginCommand {
    fn name(&self) -> &str {
        "plugin"
    }

    fn signature(&self) -> Signature {
        Signature::build("plugin")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .category(Category::Plugin)
    }

    fn description(&self) -> &str {
        "Commands for managing plugins."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(Value::string(get_full_help(self, engine_state, stack), call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "plugin add nu_plugin_inc",
                description: "Run the `nu_plugin_inc` plugin from the current directory and install its signatures.",
                result: None,
            },
            Example {
                example: "plugin use inc",
                description: "
Load (or reload) the `inc` plugin from the plugin registry file and put its
commands in scope. The plugin must already be in the registry file at parse
time.
"
                .trim(),
                result: None,
            },
            Example {
                example: "plugin list",
                description: "List installed plugins",
                result: None,
            },
            Example {
                example: "plugin stop inc",
                description: "Stop the plugin named `inc`.",
                result: None,
            },
            Example {
                example: "plugin rm inc",
                description: "Remove the installed signatures for the `inc` plugin.",
                result: None,
            },
        ]
    }
}
