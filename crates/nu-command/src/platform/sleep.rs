use std::{
    thread,
    time::{Duration, Instant},
};

use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape,
    Type, Value,
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
            .required("duration", SyntaxShape::Duration, "time to sleep")
            .rest("rest", SyntaxShape::Duration, "additional time")
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
        fn duration_from_i64(val: i64) -> Duration {
            Duration::from_nanos(if val < 0 { 0 } else { val as u64 })
        }

        let duration: i64 = call.req(engine_state, stack, 0)?;
        let rest: Vec<i64> = call.rest(engine_state, stack, 1)?;

        let total_dur =
            duration_from_i64(duration) + rest.into_iter().map(duration_from_i64).sum::<Duration>();

        let ctrlc_ref = &engine_state.ctrlc.clone();
        let start = Instant::now();
        loop {
            thread::sleep(CTRL_C_CHECK_INTERVAL);
            if start.elapsed() >= total_dur {
                break;
            }

            if nu_utils::ctrl_c::was_pressed(ctrlc_ref) {
                return Err(ShellError::InterruptedByUser {
                    span: Some(call.head),
                });
            }
        }

        Ok(Value::nothing(call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Sleep for 1sec",
                example: "sleep 1sec",
                result: Some(Value::nothing(Span::test_data())),
            },
            Example {
                description: "Sleep for 3sec",
                example: "sleep 1sec 1sec 1sec",
                result: None,
            },
            Example {
                description: "Send output after 1sec",
                example: "sleep 1sec; echo done",
                result: None,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::Sleep;

    #[test]
    fn examples_work_as_expected() {
        use std::time::Instant;

        use crate::test_examples;

        let start = Instant::now();
        test_examples(Sleep {});

        let elapsed = start.elapsed();

        // only examples with actual output are run
        assert!(elapsed >= std::time::Duration::from_secs(1));
        assert!(elapsed < std::time::Duration::from_secs(2));
    }
}
