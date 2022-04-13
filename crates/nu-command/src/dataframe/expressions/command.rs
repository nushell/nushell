use nu_engine::get_full_help;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, IntoPipelineData, PipelineData, ShellError, Signature, Value,
};

#[derive(Clone)]
pub struct LazyExpression;

impl Command for LazyExpression {
    fn name(&self) -> &str {
        "expr"
    }

    fn usage(&self) -> &str {
        "Lazy dataframe expressions"
    }

    fn extra_usage(&self) -> &str {
        r#"Expressions are the backbone of lazy frames. They represent the chained 
operations that can be performed with a lazy dataframe"#
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
                &LazyExpression.signature(),
                &LazyExpression.examples(),
                engine_state,
                stack,
            ),
            span: call.head,
        }
        .into_pipeline_data())
    }
}
