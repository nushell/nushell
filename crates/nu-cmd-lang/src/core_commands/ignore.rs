use nu_engine::command_prelude::*;
#[cfg(feature = "os")]
use nu_protocol::{
    ByteStream, Signals,
    process::{ChildPipe, check_ok},
    shell_error::io::IoError,
    write_all_and_flush,
};
use nu_protocol::{OutDest, engine::StateWorkingSet};
#[cfg(feature = "os")]
use std::{
    io::{self, Cursor},
    thread,
};

#[derive(Clone)]
pub struct Ignore;

impl Command for Ignore {
    fn name(&self) -> &str {
        "ignore"
    }

    fn description(&self) -> &str {
        "Ignore selected output streams from the previous command in the pipeline."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("ignore")
            .input_output_types(vec![(Type::Any, Type::Any)])
            .switch(
                "stderr",
                "Consume all stderr output and allow stdout output through.",
                Some('e'),
            )
            .switch(
                "stdout",
                "Consume all stdout output and allow stderr output through.",
                Some('o'),
            )
            .switch(
                "show-errors",
                "Allow errors through and set $env.LAST_EXIT_CODE (internal failures use 1).",
                Some('x'),
            )
            .category(Category::Core)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["silent", "quiet", "out-null"]
    }

    fn is_const(&self) -> bool {
        true
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let consume_stderr = call.has_flag(engine_state, stack, "stderr")?;
        let consume_stdout = call.has_flag(engine_state, stack, "stdout")? || !consume_stderr;
        let show_errors = call.has_flag(engine_state, stack, "show-errors")?;
        let span = call.head;

        match input {
            PipelineData::ByteStream(stream, metadata) => {
                #[cfg(feature = "os")]
                {
                    let stream_type = stream.type_();
                    match stream.into_child() {
                        Ok(child) => handle_external_child(
                            engine_state,
                            stack,
                            span,
                            child,
                            stream_type,
                            metadata,
                            consume_stdout,
                            consume_stderr,
                            show_errors,
                        ),
                        Err(stream) => {
                            if !consume_stdout {
                                if show_errors {
                                    stack.set_last_exit_code(0, span);
                                }
                                return Ok(PipelineData::byte_stream(stream, metadata));
                            }

                            match stream.drain() {
                                Ok(()) => {
                                    if show_errors {
                                        stack.set_last_exit_code(0, span);
                                    }
                                    Ok(PipelineData::empty())
                                }
                                Err(err) => {
                                    if show_errors {
                                        stack.set_last_exit_code(1, span);
                                    }
                                    Err(err)
                                }
                            }
                        }
                    }
                }
                #[cfg(not(feature = "os"))]
                {
                    if !consume_stdout {
                        return Ok(PipelineData::byte_stream(stream, metadata));
                    }
                    match stream.drain() {
                        Ok(()) => Ok(PipelineData::empty()),
                        Err(err) => Err(err),
                    }
                }
            }
            PipelineData::Value(Value::Error { error, .. }, _) => {
                if consume_stderr {
                    if show_errors {
                        stack.set_last_exit_code(1, span);
                        Err(*error)
                    } else {
                        Ok(PipelineData::empty())
                    }
                } else {
                    if show_errors {
                        stack.set_last_exit_code(1, span);
                    }
                    Err(*error)
                }
            }
            input => {
                if !consume_stdout {
                    if show_errors {
                        stack.set_last_exit_code(0, span);
                    }
                    return Ok(input);
                }

                match input.drain() {
                    Ok(()) => {
                        if show_errors {
                            stack.set_last_exit_code(0, span);
                        }
                        Ok(PipelineData::empty())
                    }
                    Err(err) => {
                        if show_errors {
                            stack.set_last_exit_code(1, span);
                        }
                        Err(err)
                    }
                }
            }
        }
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let consume_stderr = call.has_flag_const(working_set, "stderr")?;
        let consume_stdout = call.has_flag_const(working_set, "stdout")? || !consume_stderr;

        if consume_stderr && matches!(&input, PipelineData::Value(Value::Error { .. }, _)) {
            return Ok(PipelineData::empty());
        }

        if consume_stdout {
            input.drain()?;
            Ok(PipelineData::empty())
        } else {
            Ok(input)
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Ignore all stdout output (default behavior).",
                example: "echo done | ignore",
                result: Some(Value::nothing(Span::test_data())),
            },
            Example {
                description: "Consume stderr and allow stdout through.",
                example: "echo done | ignore --stderr",
                result: Some(Value::test_string("done")),
            },
            Example {
                description: "Consume stdout while keeping stderr visible.",
                example: "$'done' | ignore --stdout",
                result: Some(Value::nothing(Span::test_data())),
            },
            Example {
                description: "Show internal errors and read the resulting exit code.",
                example: "try { error make {msg: 'boom'} | ignore --show-errors } catch { $env.LAST_EXIT_CODE }",
                result: Some(Value::test_int(1)),
            },
        ]
    }

    fn pipe_redirection(&self) -> (Option<OutDest>, Option<OutDest>) {
        (Some(OutDest::PipeSeparate), Some(OutDest::PipeSeparate))
    }
}

#[cfg(feature = "os")]
#[expect(clippy::too_many_arguments)]
/// Handle external child-process output/error behavior for `ignore`.
fn handle_external_child(
    engine_state: &EngineState,
    stack: &mut Stack,
    span: Span,
    mut child: nu_protocol::process::ChildProcess,
    stream_type: nu_protocol::ByteStreamType,
    metadata: Option<nu_protocol::PipelineMetadata>,
    consume_stdout: bool,
    consume_stderr: bool,
    show_errors: bool,
) -> Result<PipelineData, ShellError> {
    child.ignore_error(!show_errors);

    // Preserve streaming when stdout should pass through and errors remain suppressed.
    if !consume_stdout && !show_errors {
        if consume_stderr && let Some(stderr) = child.stderr.take() {
            consume_child_pipe_on_thread(stderr, span)?;
        }

        return Ok(PipelineData::byte_stream(
            ByteStream::child(child, span),
            metadata,
        ));
    }

    // Once we need exit status enforcement or selective stderr replay, we must collect the
    // process output before deciding what to forward.
    let output = match child.wait_with_output() {
        Ok(output) => output,
        Err(err) => {
            if show_errors {
                stack.set_last_exit_code(1, span);
            }
            return Err(err);
        }
    };

    if !consume_stderr && let Some(stderr) = output.stderr.as_deref() {
        write_to_out_dest(stderr, stack.stderr(), span, false, engine_state.signals())?;
    }

    let stdout = output.stdout.unwrap_or_default();
    let exit_status = output.exit_status;

    if show_errors {
        let exit_code = exit_status.code();
        stack.set_last_exit_code(exit_code, span);

        if let Err(err) = check_ok(exit_status, false, span) {
            if !consume_stdout && !stdout.is_empty() {
                write_to_out_dest(&stdout, stack.stdout(), span, true, engine_state.signals())?;
            }
            return Err(err);
        }
    }

    if !consume_stdout && !stdout.is_empty() {
        let stream = ByteStream::read(
            Cursor::new(stdout),
            span,
            engine_state.signals().clone(),
            stream_type,
        );
        return Ok(PipelineData::byte_stream(stream, metadata));
    }

    Ok(PipelineData::empty())
}

#[cfg(feature = "os")]
/// Write collected bytes to the requested output destination.
fn write_to_out_dest(
    bytes: &[u8],
    out_dest: &OutDest,
    span: Span,
    stdout_fallback: bool,
    signals: &Signals,
) -> Result<(), ShellError> {
    if bytes.is_empty() {
        return Ok(());
    }

    match out_dest {
        OutDest::Null => Ok(()),
        OutDest::File(file) => {
            let mut file = file.as_ref();
            write_all_and_flush(bytes, &mut file, "file", Some(span), signals)
        }
        OutDest::Print
        | OutDest::Inherit
        | OutDest::Pipe
        | OutDest::PipeSeparate
        | OutDest::Value => {
            if stdout_fallback {
                let mut stdout = io::stdout();
                write_all_and_flush(bytes, &mut stdout, "stdout", Some(span), signals)
            } else {
                let mut stderr = io::stderr();
                write_all_and_flush(bytes, &mut stderr, "stderr", Some(span), signals)
            }
        }
    }
}

#[cfg(feature = "os")]
/// Consume a child pipe on a detached thread so the child process cannot block on a full stderr
/// buffer when `ignore` is passing stdout through.
fn consume_child_pipe_on_thread(pipe: ChildPipe, span: Span) -> Result<(), ShellError> {
    thread::Builder::new()
        .name("ignore stderr consumer".into())
        .spawn(move || {
            let mut sink = io::sink();
            match pipe {
                ChildPipe::Pipe(mut pipe) => io::copy(&mut pipe, &mut sink),
                ChildPipe::Tee(mut tee) => io::copy(&mut tee, &mut sink),
            }?;
            Ok::<(), io::Error>(())
        })
        .map_err(|err| ShellError::Io(IoError::new(err, span, None)))?;
    Ok(())
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() -> nu_test_support::Result {
        use super::Ignore;
        nu_test_support::test().examples(Ignore)
    }
}
