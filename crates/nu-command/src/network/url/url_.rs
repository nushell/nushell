use nu_engine::get_full_help;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, IntoPipelineData, PipelineData, Signature, Type, Value,
};

#[derive(Clone)]
pub struct Url;

impl Command for Url {
    fn name(&self) -> &str {
        "url"
    }

    fn signature(&self) -> Signature {
        Signature::build("url")
            .input_output_types(vec![(Type::String, Type::String)])
            .category(Category::Network)
    }

    fn usage(&self) -> &str {
        "Apply url function."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["network", "parse"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        Ok(Value::String {
            val: get_full_help(&Url.signature(), &Url.examples(), engine_state, stack),
            span: call.head,
        }
        .into_pipeline_data())
    }
}
