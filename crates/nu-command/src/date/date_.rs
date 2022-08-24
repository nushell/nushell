use nu_engine::get_full_help;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, IntoPipelineData, PipelineData, ShellError, Signature, Value,
};

#[derive(Clone)]
pub struct Date;

impl Command for Date {
    fn name(&self) -> &str {
        "date"
    }

    fn signature(&self) -> Signature {
        Signature::build("date").category(Category::Date)
    }

    fn usage(&self) -> &str {
        "Date-related commands"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec![
            "time",
            "now",
            "today",
            "tomorrow",
            "yesterday",
            "weekday",
            "weekday_name",
            "timezone",
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        date(engine_state, stack, call)
    }
}

fn date(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    let head = call.head;

    Ok(Value::String {
        val: get_full_help(&Date.signature(), &Date.examples(), engine_state, stack),
        span: head,
    }
    .into_pipeline_data())
}
