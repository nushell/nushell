use nu_engine::{command_prelude::*, get_full_help};

mod list;
mod stop;

pub use list::PluginList;
pub use stop::PluginStop;

#[derive(Clone)]
pub struct PluginCommand;

impl Command for PluginCommand {
    fn name(&self) -> &str {
        "plugin"
    }

    fn signature(&self) -> Signature {
        Signature::build("plugin")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .category(Category::Core)
    }

    fn usage(&self) -> &str {
        "Commands for managing plugins."
    }

    fn extra_usage(&self) -> &str {
        "To load a plugin, see `register`."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(Value::string(
            get_full_help(
                &PluginCommand.signature(),
                &PluginCommand.examples(),
                engine_state,
                stack,
                self.is_parser_keyword(),
            ),
            call.head,
        )
        .into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
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
        ]
    }
}
