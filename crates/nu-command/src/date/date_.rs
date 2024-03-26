use nu_engine::{command_prelude::*, get_full_help};

#[derive(Clone)]
pub struct Date;

impl Command for Date {
    fn name(&self) -> &str {
        "date"
    }

    fn signature(&self) -> Signature {
        Signature::build("date")
            .category(Category::Date)
            .input_output_types(vec![(Type::Nothing, Type::String)])
    }

    fn usage(&self) -> &str {
        "Date-related commands."
    }

    fn extra_usage(&self) -> &str {
        "You must use one of the following subcommands. Using this command as-is will only produce this help message."
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

    Ok(Value::string(
        get_full_help(
            &Date.signature(),
            &Date.examples(),
            engine_state,
            stack,
            false,
        ),
        head,
    )
    .into_pipeline_data())
}
