use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape,
    Type, Value,
};
use std::{
    thread,
    time::{Duration, Instant},
};

const CTRL_C_CHECK_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Clone)]
pub struct Sleep;

impl Command for Sleep {
    fn name(&self) -> &str {
        "sleep"
    }

    fn usage(&self) -> &str {
        "Delay for a specified amount of time."
    }

    fn signature(&self) -> Signature {
        Signature::build("sleep")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required(
                "duration",
                SyntaxShape::OneOf(vec![SyntaxShape::Duration, SyntaxShape::Number]),
                "time to sleep",
            )
            .rest(
                "rest",
                SyntaxShape::OneOf(vec![SyntaxShape::Duration, SyntaxShape::Number]),
                "additional time",
            )
            .category(Category::Platform)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["delay", "wait", "timer"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let args: Vec<Value> = call.rest(engine_state, stack, 0)?;

        let total_duration = args.into_iter().map(duration_from_value).sum::<Duration>();

        let ctrlc_ref = &engine_state.ctrlc.clone();
        let start = Instant::now();
        loop {
            thread::sleep(CTRL_C_CHECK_INTERVAL);
            if start.elapsed() >= total_duration {
                break;
            }

            if nu_utils::ctrl_c::was_pressed(ctrlc_ref) {
                break;
            }
        }

        Ok(Value::Nothing { span: call.head }.into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Sleep for 3 seconds",
                example: "sleep 3sec",
                result: None,
            },
            Example {
                description: "Sleep for 3.5 seconds with multiple arguments",
                example: "sleep 1sec 1 1.5",
                result: Some(Value::Nothing {
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Send output after 1sec",
                example: "sleep 1sec; echo done",
                result: None,
            },
        ]
    }
}

fn duration_from_value(value: Value) -> Duration {
    match value {
        Value::Int { val, span: _ } => Duration::from_secs(if val < 0 { 0 } else { val as u64 }),
        Value::Float { val, span: _ } => {
            // A user can do `sleep 1.2`, but unlikely `sleep 1.2345`, so millisecond precision is enough
            Duration::from_millis(if val < 0.0 { 0 } else { (val * 1000.0) as u64 })
        }
        Value::Duration { val, span: _ } => {
            Duration::from_nanos(if val < 0 { 0 } else { val as u64 })
        }
        _ => panic!("Unknown type"), // this should never happen, it's covered by the SyntaxShape::OneOf above
    }
}

#[cfg(test)]
mod tests {
    use super::Sleep;

    #[test]
    fn examples_work_as_expected() {
        use crate::test_examples;
        use std::time::Instant;

        let start = Instant::now();
        test_examples(Sleep {});

        let elapsed = start.elapsed();

        // only examples with actual output are run
        assert!(elapsed >= std::time::Duration::from_secs(3));
        assert!(elapsed < std::time::Duration::from_secs(4));
    }
}
