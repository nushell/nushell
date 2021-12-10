use nu_engine::get_full_help;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, IntoPipelineData, PipelineData, ShellError, Signature, Value,
};

#[derive(Clone)]
pub struct Dataframe;

impl Command for Dataframe {
    fn name(&self) -> &str {
        "dataframe"
    }

    fn usage(&self) -> &str {
        "Dataframe commands"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).category(Category::Custom("dataframe".into()))
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(Value::String {
            val: get_full_help(&Dataframe.signature(), &Dataframe.examples(), engine_state),
            span: call.head,
        }
        .into_pipeline_data())
    }
}
