use nu_engine::get_full_help;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, IntoPipelineData, PipelineData, ShellError, Signature, Value,
};

#[derive(Clone)]
pub struct LazyDataframe;

impl Command for LazyDataframe {
    fn name(&self) -> &str {
        "dfl"
    }

    fn usage(&self) -> &str {
        "Lazy dataframe commands"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).category(Category::Custom("dataframe".into()))
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(Value::String {
            val: get_full_help(
                &LazyDataframe.signature(),
                &LazyDataframe.examples(),
                engine_state,
                stack,
            ),
            span: call.head,
        }
        .into_pipeline_data())
    }
}
