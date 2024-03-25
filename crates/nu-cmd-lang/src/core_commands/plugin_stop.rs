use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape, Type,
};

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
                "The name of the plugin to stop.",
            )
            .category(Category::Core)
    }

    fn usage(&self) -> &str {
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

        let mut found = false;
        for plugin in engine_state.plugins() {
            if plugin.identity().name() == name.item {
                plugin.stop()?;
                found = true;
            }
        }

        if found {
            Ok(PipelineData::Empty)
        } else {
            Err(ShellError::GenericError {
                error: format!("Failed to stop the `{}` plugin", name.item),
                msg: "couldn't find a plugin with this name".into(),
                span: Some(name.span),
                help: Some("you may need to `register` the plugin first".into()),
                inner: vec![],
            })
        }
    }
}
