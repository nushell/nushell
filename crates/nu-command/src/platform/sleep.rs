use nu_engine::command_prelude::*;

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

    fn description(&self) -> &str {
        "Delay for a specified amount of time."
    }

    fn signature(&self) -> Signature {
        Signature::build("sleep")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required("duration", SyntaxShape::Duration, "Time to sleep.")
            .rest("rest", SyntaxShape::Duration, "Additional time.")
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
        let deadline = Instant::now() + total_dur;

        loop {
            // sleep for 100ms, or until the deadline
            let time_until_deadline = deadline.saturating_duration_since(Instant::now());
            if time_until_deadline.is_zero() {
                break;
            }
            thread::sleep(CTRL_C_CHECK_INTERVAL.min(time_until_deadline));
            engine_state.signals().check(&call.head)?;
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
                description: "Use multiple arguments to write a duration with multiple units, which is unsupported by duration literals",
                example: "sleep 1min 30sec",
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
        use crate::test_examples;
        use std::time::Instant;

        let start = Instant::now();
        test_examples(Sleep {});

        let elapsed = start.elapsed();

        // only examples with actual output are run
        assert!(elapsed >= std::time::Duration::from_secs(1));
        assert!(elapsed < std::time::Duration::from_secs(2));
    }
}
