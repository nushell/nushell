use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, NuDuration, PipelineData, ShellError, Signature, Span, SyntaxShape,
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
        fn ns_from_arg(arg: Value) -> Result<i64, ShellError> {
            if let Value::Duration{val, span} = arg {
            val.to_ns_or_err(span)
            } else {
                Err(ShellError::TypeMismatch { err_message: "Expected duration value".into(), span: arg.span()? })
            }
        }

        let duration: Value = call.req(engine_state, stack, 0)?;
        let rest: Vec<Value> = call.rest(engine_state, stack, 1)?;

        let total_dur = ns_from_arg(duration)? + rest.into_iter().map(|d| ns_from_arg(d)?).sum::<i64>();

        let ctrlc_ref = &engine_state.ctrlc.clone();
        let start = Instant::now();
        loop {
            thread::sleep(CTRL_C_CHECK_INTERVAL);
            if start.elapsed() >= total_dur {
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
                description: "Sleep for 1sec",
                example: "sleep 1sec",
                result: Some(Value::Nothing {
                    span: Span::test_data(),
                }),
            },
            // Example {
            //     description: "Sleep for 3sec",
            //     example: "sleep 1sec 1sec 1sec",
            //     result: None,
            // },
            // Example {
            //     description: "Send output after 1sec",
            //     example: "sleep 1sec; echo done",
            //     result: None,
            // },
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
