use crate::HttpPlugin;
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, IntoPipelineData, LabeledError, PipelineData, Signature, Type, Value};

#[derive(Clone)]
pub struct Http;

impl PluginCommand for Http {
    type Plugin = HttpPlugin;

    fn name(&self) -> &str {
        "http"
    }

    fn signature(&self) -> Signature {
        Signature::build("http")
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .category(Category::Network)
    }

    fn description(&self) -> &str {
        "Various commands for working with http methods."
    }

    fn extra_description(&self) -> &str {
        "You must use one of the following subcommands. Using this command as-is will only produce this help message."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec![
            "network", "fetch", "pull", "request", "download", "curl", "wget",
        ]
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        Ok(Value::string(engine.get_help()?, call.head).into_pipeline_data())
    }
}
