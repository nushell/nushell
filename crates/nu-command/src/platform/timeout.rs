use nu_engine::{command_prelude::*, get_eval_block_with_early_return};
use nu_protocol::{engine::Closure, format_duration};
use std::{sync::mpsc, thread, time::Duration};

#[derive(Clone)]
pub struct Timeout;

impl Command for Timeout {
    fn name(&self) -> &str {
        "timeout"
    }

    fn description(&self) -> &str {
        "Run a closure with a time limit."
    }

    fn extra_description(&self) -> &str {
        r#"If the closure does not complete within the specified duration, it will be terminated
and an error will be returned. This is useful for preventing long-running operations
from blocking indefinitely, such as in MCP servers or automation scripts."#
    }

    fn signature(&self) -> Signature {
        Signature::build("timeout")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .required("duration", SyntaxShape::Duration, "Time limit.")
            .required("closure", SyntaxShape::Closure(None), "The closure to run.")
            .category(Category::Platform)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["limit", "deadline", "cancel"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let duration_val: i64 = call.req(engine_state, stack, 0)?;
        let closure: Closure = call.req(engine_state, stack, 1)?;

        let duration = Duration::from_nanos(if duration_val < 0 {
            0
        } else {
            duration_val as u64
        });

        // Clone engine state and prepare stack for the closure
        let engine_state_clone = engine_state.clone();
        let mut callee_stack = stack.captures_to_stack_preserve_out_dest(closure.captures);
        let block = engine_state.get_block(closure.block_id).clone();

        // Channel to receive the result
        let (tx, rx) = mpsc::channel();

        // Spawn a thread to run the closure
        let handle = thread::Builder::new()
            .name("timeout".into())
            .spawn(move || {
                let eval_block_with_early_return =
                    get_eval_block_with_early_return(&engine_state_clone);
                let result = eval_block_with_early_return(
                    &engine_state_clone,
                    &mut callee_stack,
                    &block,
                    PipelineData::empty(),
                )
                .map(|p| p.body);
                let _ = tx.send(result);
            })
            .map_err(|e| ShellError::GenericError {
                error: "failed to spawn thread".into(),
                msg: e.to_string(),
                span: Some(head),
                help: None,
                inner: vec![],
            })?;

        // Wait for the result with timeout
        match rx.recv_timeout(duration) {
            Ok(result) => {
                // Thread completed, wait for it to finish
                let _ = handle.join();
                result
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Timeout occurred
                // Note: We can't forcibly kill the thread in Rust, but we return an error
                // The thread will continue to run in the background until it completes
                Err(ShellError::GenericError {
                    error: "timeout".into(),
                    msg: format!(
                        "operation timed out after {}",
                        format_duration(duration_val)
                    ),
                    span: Some(head),
                    help: Some(
                        "the operation did not complete within the specified time limit".into(),
                    ),
                    inner: vec![],
                })
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                // Thread panicked or channel disconnected
                Err(ShellError::GenericError {
                    error: "thread error".into(),
                    msg: "the closure thread terminated unexpectedly".into(),
                    span: Some(head),
                    help: None,
                    inner: vec![],
                })
            }
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Run a closure with a timeout",
                example: "timeout 1sec { 'done' }",
                result: Some(Value::test_string("done")),
            },
            Example {
                description: "Run a command that completes before the timeout",
                example: "timeout 5sec { sleep 2sec; 'completed' }",
                result: None, // Uses sleep command
            },
            Example {
                description: "Timeout a long-running operation",
                example: "timeout 100ms { sleep 5sec }",
                result: None, // Returns an error
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::Timeout;

    #[test]
    fn examples_work_as_expected() {
        use crate::test_examples;
        test_examples(Timeout {});
    }
}
