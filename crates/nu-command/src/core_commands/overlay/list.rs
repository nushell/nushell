use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Value,
};

#[derive(Clone)]
pub struct OverlayList;

impl Command for OverlayList {
    fn name(&self) -> &str {
        "overlay list"
    }

    fn usage(&self) -> &str {
        "List all active overlays"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("overlay list").category(Category::Core)
    }

    fn extra_usage(&self) -> &str {
        "The overlays are listed in the order they were activated."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let active_overlays_engine: Vec<Value> = stack
            .active_overlays
            .iter()
            .map(|s| Value::string(s, call.head))
            .collect();

        Ok(Value::List {
            vals: active_overlays_engine,
            span: call.head,
        }
        .into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Get the last activated overlay",
            example: r#"module spam { export def foo [] { "foo" } }
    overlay use spam
    overlay list | last"#,
            result: Some(Value::String {
                val: "spam".to_string(),
                span: Span::test_data(),
            }),
        }]
    }
}
