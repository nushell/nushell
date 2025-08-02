use nu_engine::command_prelude::*;

use crate::util::canonicalize_possible_filename_arg;

#[derive(Clone)]
pub struct PluginStop;

impl Command for PluginStop {
    fn name(&self) -> &str {
        "plugin stop"
    }

    fn signature(&self) -> Signature {
        Signature::build("plugin stop")
            .input_output_type(Type::Nothing, Type::Nothing)
            .required(
                "name",
                SyntaxShape::String,
                "The name, or filename, of the plugin to stop.",
            )
            .category(Category::Plugin)
    }

    fn description(&self) -> &str {
        "Stop an installed plugin if it was running."
    }

    fn examples(&self) -> Vec<nu_protocol::Example> {
        vec![
            Example {
                example: "plugin stop inc",
                description: "Stop the plugin named `inc`.",
                result: None,
            },
            Example {
                example: "plugin stop ~/.cargo/bin/nu_plugin_inc",
                description: "Stop the plugin with the filename `~/.cargo/bin/nu_plugin_inc`.",
                result: None,
            },
            Example {
                example: "plugin list | each { |p| plugin stop $p.name }",
                description: "Stop all plugins.",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let name: Spanned<String> = call.req(engine_state, stack, 0)?;

        let filename = canonicalize_possible_filename_arg(engine_state, stack, &name.item);

        let mut found = false;
        for plugin in engine_state.plugins() {
            let id = &plugin.identity();
            if id.name() == name.item || id.filename() == filename {
                plugin.stop()?;
                found = true;
            }
        }

        if found {
            Ok(PipelineData::empty())
        } else {
            Err(ShellError::GenericError {
                error: format!("Failed to stop the `{}` plugin", name.item),
                msg: "couldn't find a plugin with this name".into(),
                span: Some(name.span),
                help: Some("you may need to `plugin add` the plugin first".into()),
                inner: vec![],
            })
        }
    }
}
