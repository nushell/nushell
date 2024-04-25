use nu_engine::{command_prelude::*, get_eval_block_with_early_return};
use nu_protocol::{engine::Closure, OutDest, RawStream};
use std::{sync::mpsc, thread};

#[derive(Clone)]
pub struct Tee;

impl Command for Tee {
    fn name(&self) -> &str {
        "tee"
    }

    fn usage(&self) -> &str {
        "Copy a stream to another command in parallel."
    }

    fn extra_usage(&self) -> &str {
        r#"This is useful for doing something else with a stream while still continuing to
use it in your pipeline."#
    }

    fn signature(&self) -> Signature {
        Signature::build("tee")
            .input_output_type(Type::Any, Type::Any)
            .switch(
                "stderr",
                "For external commands: copy the standard error stream instead.",
                Some('e'),
            )
            .required(
                "closure",
                SyntaxShape::Closure(None),
                "The other command to send the stream to.",
            )
            .category(Category::Filters)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "http get http://example.org/ | tee { save example.html }",
                description: "Save a webpage to a file while also printing it",
                result: None,
            },
            Example {
                example:
                    "nu -c 'print -e error; print ok' | tee --stderr { save error.log } | complete",
                description: "Save error messages from an external command to a file without \
                    redirecting them",
                result: None,
            },
            Example {
                example: "1..100 | tee { each { print } } | math sum | wrap sum",
                description: "Print numbers and their sum",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let use_stderr = call.has_flag(engine_state, stack, "stderr")?;

        let Spanned {
            item: Closure { block_id, captures },
            span: closure_span,
        } = call.req(engine_state, stack, 0)?;

        let closure_engine_state = engine_state.clone();
        let mut closure_stack = stack
            .captures_to_stack_preserve_out_dest(captures)
            .reset_pipes();

        let metadata = input.metadata();
        let metadata_clone = metadata.clone();

        let eval_block_with_early_return = get_eval_block_with_early_return(engine_state);

        match input {
            // Handle external streams specially, to make sure they pass through
            PipelineData::ExternalStream {
                stdout,
                stderr,
                exit_code,
                span,
                metadata,
                trim_end_newline,
            } => {
                let known_size = if use_stderr {
                    stderr.as_ref().and_then(|s| s.known_size)
                } else {
                    stdout.as_ref().and_then(|s| s.known_size)
                };

                let with_stream = move |rx: mpsc::Receiver<Result<Vec<u8>, ShellError>>| {
                    let iter = rx.into_iter();
                    let input_from_channel = PipelineData::ExternalStream {
                        stdout: Some(RawStream::new(
                            Box::new(iter),
                            closure_engine_state.ctrlc.clone(),
                            span,
                            known_size,
                        )),
                        stderr: None,
                        exit_code: None,
                        span,
                        metadata: metadata_clone,
                        trim_end_newline,
                    };
                    let result = eval_block_with_early_return(
                        &closure_engine_state,
                        &mut closure_stack,
                        closure_engine_state.get_block(block_id),
                        input_from_channel,
                    );
                    // Make sure to drain any iterator produced to avoid unexpected behavior
                    result.and_then(|data| data.drain())
                };

                if use_stderr {
                    let stderr = stderr
                        .map(|stderr| {
                            let iter = tee(stderr.stream, with_stream).err_span(call.head)?;
                            Ok::<_, ShellError>(RawStream::new(
                                Box::new(iter.map(flatten_result)),
                                stderr.ctrlc,
                                stderr.span,
                                stderr.known_size,
                            ))
                        })
                        .transpose()?;
                    Ok(PipelineData::ExternalStream {
                        stdout,
                        stderr,
                        exit_code,
                        span,
                        metadata,
                        trim_end_newline,
                    })
                } else {
                    let stdout = stdout
                        .map(|stdout| {
                            let iter = tee(stdout.stream, with_stream).err_span(call.head)?;
                            Ok::<_, ShellError>(RawStream::new(
                                Box::new(iter.map(flatten_result)),
                                stdout.ctrlc,
                                stdout.span,
                                stdout.known_size,
                            ))
                        })
                        .transpose()?;
                    Ok(PipelineData::ExternalStream {
                        stdout,
                        stderr,
                        exit_code,
                        span,
                        metadata,
                        trim_end_newline,
                    })
                }
            }
            // --stderr is not allowed if the input is not an external stream
            _ if use_stderr => Err(ShellError::UnsupportedInput {
                msg: "--stderr can only be used on external streams".into(),
                input: "the input to `tee` is not an external stream".into(),
                msg_span: call.head,
                input_span: input.span().unwrap_or(call.head),
            }),
            // Handle others with the plain iterator
            _ => {
                let teed = tee(input.into_iter(), move |rx| {
                    let input_from_channel = rx.into_pipeline_data_with_metadata(
                        metadata_clone,
                        closure_engine_state.ctrlc.clone(),
                    );
                    let result = eval_block_with_early_return(
                        &closure_engine_state,
                        &mut closure_stack,
                        closure_engine_state.get_block(block_id),
                        input_from_channel,
                    );
                    // Make sure to drain any iterator produced to avoid unexpected behavior
                    result.and_then(|data| data.drain())
                })
                .err_span(call.head)?
                .map(move |result| result.unwrap_or_else(|err| Value::error(err, closure_span)))
                .into_pipeline_data_with_metadata(metadata, engine_state.ctrlc.clone());

                Ok(teed)
            }
        }
    }

    fn pipe_redirection(&self) -> (Option<OutDest>, Option<OutDest>) {
        (Some(OutDest::Capture), Some(OutDest::Capture))
    }
}

fn panic_error() -> ShellError {
    ShellError::NushellFailed {
        msg: "A panic occurred on a thread spawned by `tee`".into(),
    }
}

fn flatten_result<T, E>(result: Result<Result<T, E>, E>) -> Result<T, E> {
    result.unwrap_or_else(Err)
}

/// Copies the iterator to a channel on another thread. If an error is produced on that thread,
/// it is embedded in the resulting iterator as an `Err` as soon as possible. When the iterator
/// finishes, it waits for the other thread to finish, also handling any error produced at that
/// point.
fn tee<T>(
    input: impl Iterator<Item = T>,
    with_cloned_stream: impl FnOnce(mpsc::Receiver<T>) -> Result<(), ShellError> + Send + 'static,
) -> Result<impl Iterator<Item = Result<T, ShellError>>, std::io::Error>
where
    T: Clone + Send + 'static,
{
    // For sending the values to the other thread
    let (tx, rx) = mpsc::channel();

    let mut thread = Some(
        thread::Builder::new()
            .name("stderr consumer".into())
            .spawn(move || with_cloned_stream(rx))?,
    );

    let mut iter = input.into_iter();
    let mut tx = Some(tx);

    Ok(std::iter::from_fn(move || {
        if thread.as_ref().is_some_and(|t| t.is_finished()) {
            // Check for an error from the other thread
            let result = thread
                .take()
                .expect("thread was taken early")
                .join()
                .unwrap_or_else(|_| Err(panic_error()));
            if let Err(err) = result {
                // Embed the error early
                return Some(Err(err));
            }
        }

        // Get a value from the iterator
        if let Some(value) = iter.next() {
            // Send a copy, ignoring any error if the channel is closed
            let _ = tx.as_ref().map(|tx| tx.send(value.clone()));
            Some(Ok(value))
        } else {
            // Close the channel so the stream ends for the other thread
            drop(tx.take());
            // Wait for the other thread, and embed any error produced
            thread.take().and_then(|t| {
                t.join()
                    .unwrap_or_else(|_| Err(panic_error()))
                    .err()
                    .map(Err)
            })
        }
    }))
}

#[test]
fn tee_copies_values_to_other_thread_and_passes_them_through() {
    let (tx, rx) = mpsc::channel();

    let expected_values = vec![1, 2, 3, 4];

    let my_result = tee(expected_values.clone().into_iter(), move |rx| {
        for val in rx {
            let _ = tx.send(val);
        }
        Ok(())
    })
    .expect("io error")
    .collect::<Result<Vec<i32>, ShellError>>()
    .expect("should not produce error");

    assert_eq!(expected_values, my_result);

    let other_threads_result = rx.into_iter().collect::<Vec<_>>();

    assert_eq!(expected_values, other_threads_result);
}

#[test]
fn tee_forwards_errors_back_immediately() {
    use std::time::Duration;
    let slow_input = (0..100).inspect(|_| std::thread::sleep(Duration::from_millis(1)));
    let iter = tee(slow_input, |_| {
        Err(ShellError::IOError { msg: "test".into() })
    })
    .expect("io error");
    for result in iter {
        if let Ok(val) = result {
            // should not make it to the end
            assert!(val < 99, "the error did not come early enough");
        } else {
            // got the error
            return;
        }
    }
    panic!("never received the error");
}

#[test]
fn tee_waits_for_the_other_thread() {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    use std::time::Duration;
    let waited = Arc::new(AtomicBool::new(false));
    let waited_clone = waited.clone();
    let iter = tee(0..100, move |_| {
        std::thread::sleep(Duration::from_millis(10));
        waited_clone.store(true, Ordering::Relaxed);
        Err(ShellError::IOError { msg: "test".into() })
    })
    .expect("io error");
    let last = iter.last();
    assert!(waited.load(Ordering::Relaxed), "failed to wait");
    assert!(
        last.is_some_and(|res| res.is_err()),
        "failed to return error from wait"
    );
}
