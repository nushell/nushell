use chrono_tz::TZ_VARIANTS;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "date list-timezone"
    }

    fn signature(&self) -> Signature {
        Signature::build("date list-timezone")
            .input_output_types(vec![(Type::Nothing, Type::table())])
            .category(Category::Date)
    }

    fn usage(&self) -> &str {
        "List supported time zones."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["UTC", "GMT", "tz"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;

        Ok(TZ_VARIANTS
            .iter()
            .map(move |x| {
                Value::record(
                    record! { "timezone" => Value::string(x.name(), span) },
                    span,
                )
            })
            .into_pipeline_data(engine_state.ctrlc.clone()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            example: "date list-timezone | where timezone =~ Shanghai",
            description: "Show time zone(s) that contains 'Shanghai'",
            result: Some(Value::test_list(vec![Value::test_record(record! {
                "timezone" => Value::test_string("Asia/Shanghai"),
            })])),
        }]
    }
}
