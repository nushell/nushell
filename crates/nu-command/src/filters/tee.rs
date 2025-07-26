use nu_engine::{command_prelude::*, get_eval_block_with_early_return};
#[cfg(feature = "os")]
use nu_protocol::process::ChildPipe;
#[cfg(test)]
use nu_protocol::shell_error;
use nu_protocol::{
    ByteStream, ByteStreamSource, OutDest, PipelineMetadata, Signals,
    byte_stream::copy_with_signals, engine::Closure, report_shell_error, shell_error::io::IoError,
};
use std::{
    io::{self, Read, Write},
    sync::{
        Arc,
        mpsc::{self, Sender},
    },
    thread::{self, JoinHandle},
};

#[derive(Clone)]
pub struct Tee;

impl Command for Tee {
    fn name(&self) -> &str {
        "tee"
    }

    fn description(&self) -> &str {
        "Copy a stream to another command in parallel."
    }

    fn extra_description(&self) -> &str {
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
                example: "nu -c 'print -e error; print ok' | tee --stderr { save error.log } | complete",
                description: "Save error messages from an external command to a file without \
                    redirecting them",
                result: None,
            },
            Example {
                example: "1..100 | tee { each { print } } | math sum | wrap sum",
                description: "Print numbers and their sum",
                result: None,
            },
            Example {
                example: "10000 | tee { 1..$in | print } | $in * 5",
                description: "Do something with a value on another thread, while also passing through the value",
                result: Some(Value::test_int(50000)),
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
        let head = call.head;
        let from_io_error = IoError::factory(head, None);
        let use_stderr = call.has_flag(engine_state, stack, "stderr")?;

        let closure: Spanned<Closure> = call.req(engine_state, stack, 0)?;
        let closure_span = closure.span;
        let closure = closure.item;

        let engine_state_arc = Arc::new(engine_state.clone());

        let mut eval_block = {
            let closure_engine_state = engine_state_arc.clone();
            let mut closure_stack = stack
                .captures_to_stack_preserve_out_dest(closure.captures)
                .reset_pipes();
            let eval_block_with_early_return = get_eval_block_with_early_return(engine_state);

            move |input| {
                let result = eval_block_with_early_return(
                    &closure_engine_state,
                    &mut closure_stack,
                    closure_engine_state.get_block(closure.block_id),
                    input,
                );
                // Make sure to drain any iterator produced to avoid unexpected behavior
                result.and_then(|data| data.drain().map(|_| ()))
            }
        };

        // Convert values that can be represented as streams into streams. Streams can pass errors
        // through later, so if we treat string/binary/list as a stream instead, it's likely that
        // we can get the error back to the original thread.
        let span = input.span().unwrap_or(head);
        let input = input
            .try_into_stream(engine_state)
            .unwrap_or_else(|original_input| original_input);

        if let PipelineData::ByteStream(stream, metadata) = input {
            let type_ = stream.type_();

            let info = StreamInfo {
                span,
                signals: engine_state.signals().clone(),
                type_,
                metadata: metadata.clone(),
            };

            match stream.into_source() {
                ByteStreamSource::Read(read) => {
                    if use_stderr {
                        return stderr_misuse(span, head);
                    }

                    let tee_thread = spawn_tee(info, eval_block)?;
                    let tee = IoTee::new(read, tee_thread);

                    Ok(PipelineData::byte_stream(
                        ByteStream::read(tee, span, engine_state.signals().clone(), type_),
                        metadata,
                    ))
                }
                ByteStreamSource::File(file) => {
                    if use_stderr {
                        return stderr_misuse(span, head);
                    }

                    let tee_thread = spawn_tee(info, eval_block)?;
                    let tee = IoTee::new(file, tee_thread);

                    Ok(PipelineData::byte_stream(
                        ByteStream::read(tee, span, engine_state.signals().clone(), type_),
                        metadata,
                    ))
                }
                #[cfg(feature = "os")]
                ByteStreamSource::Child(mut child) => {
                    let stderr_thread = if use_stderr {
                        let stderr_thread = if let Some(stderr) = child.stderr.take() {
                            let tee_thread = spawn_tee(info.clone(), eval_block)?;
                            let tee = IoTee::new(stderr, tee_thread);
                            match stack.stderr() {
                                OutDest::Pipe | OutDest::PipeSeparate | OutDest::Value => {
                                    child.stderr = Some(ChildPipe::Tee(Box::new(tee)));
                                    Ok(None)
                                }
                                OutDest::Null => copy_on_thread(tee, io::sink(), &info).map(Some),
                                OutDest::Print | OutDest::Inherit => {
                                    copy_on_thread(tee, io::stderr(), &info).map(Some)
                                }
                                OutDest::File(file) => {
                                    copy_on_thread(tee, file.clone(), &info).map(Some)
                                }
                            }?
                        } else {
                            None
                        };

                        if let Some(stdout) = child.stdout.take() {
                            match stack.stdout() {
                                OutDest::Pipe | OutDest::PipeSeparate | OutDest::Value => {
                                    child.stdout = Some(stdout);
                                    Ok(())
                                }
                                OutDest::Null => copy_pipe(stdout, io::sink(), &info),
                                OutDest::Print | OutDest::Inherit => {
                                    copy_pipe(stdout, io::stdout(), &info)
                                }
                                OutDest::File(file) => copy_pipe(stdout, file.as_ref(), &info),
                            }?;
                        }

                        stderr_thread
                    } else {
                        let stderr_thread = if let Some(stderr) = child.stderr.take() {
                            let info = info.clone();
                            match stack.stderr() {
                                OutDest::Pipe | OutDest::PipeSeparate | OutDest::Value => {
                                    child.stderr = Some(stderr);
                                    Ok(None)
                                }
                                OutDest::Null => {
                                    copy_pipe_on_thread(stderr, io::sink(), &info).map(Some)
                                }
                                OutDest::Print | OutDest::Inherit => {
                                    copy_pipe_on_thread(stderr, io::stderr(), &info).map(Some)
                                }
                                OutDest::File(file) => {
                                    copy_pipe_on_thread(stderr, file.clone(), &info).map(Some)
                                }
                            }?
                        } else {
                            None
                        };

                        if let Some(stdout) = child.stdout.take() {
                            let tee_thread = spawn_tee(info.clone(), eval_block)?;
                            let tee = IoTee::new(stdout, tee_thread);
                            match stack.stdout() {
                                OutDest::Pipe | OutDest::PipeSeparate | OutDest::Value => {
                                    child.stdout = Some(ChildPipe::Tee(Box::new(tee)));
                                    Ok(())
                                }
                                OutDest::Null => copy(tee, io::sink(), &info),
                                OutDest::Print | OutDest::Inherit => copy(tee, io::stdout(), &info),
                                OutDest::File(file) => copy(tee, file.as_ref(), &info),
                            }?;
                        }

                        stderr_thread
                    };

                    if child.stdout.is_some() || child.stderr.is_some() {
                        Ok(PipelineData::byte_stream(
                            ByteStream::child(*child, span),
                            metadata,
                        ))
                    } else {
                        if let Some(thread) = stderr_thread {
                            thread.join().unwrap_or_else(|_| Err(panic_error()))?;
                        }
                        child.wait()?;
                        Ok(PipelineData::empty())
                    }
                }
            }
        } else {
            if use_stderr {
                return stderr_misuse(input.span().unwrap_or(head), head);
            }

            let metadata = input.metadata();
            let metadata_clone = metadata.clone();

            if matches!(input, PipelineData::ListStream(..)) {
                // Only use the iterator implementation on lists / list streams. We want to be able
                // to preserve errors as much as possible, and only the stream implementations can
                // really do that
                let signals = engine_state.signals().clone();

                Ok(tee(input.into_iter(), move |rx| {
                    let input = rx.into_pipeline_data_with_metadata(span, signals, metadata_clone);
                    eval_block(input)
                })
                .map_err(&from_io_error)?
                .map(move |result| result.unwrap_or_else(|err| Value::error(err, closure_span)))
                .into_pipeline_data_with_metadata(
                    span,
                    engine_state.signals().clone(),
                    metadata,
                ))
            } else {
                // Otherwise, we can spawn a thread with the input value, but we have nowhere to
                // send an error to other than just trying to print it to stderr.
                let value = input.into_value(span)?;
                let value_clone = value.clone();
                tee_once(engine_state_arc, move || {
                    eval_block(value_clone.into_pipeline_data_with_metadata(metadata_clone))
                })
                .map_err(&from_io_error)?;
                Ok(value.into_pipeline_data_with_metadata(metadata))
            }
        }
    }

    fn pipe_redirection(&self) -> (Option<OutDest>, Option<OutDest>) {
        (Some(OutDest::PipeSeparate), Some(OutDest::PipeSeparate))
    }
}

fn panic_error() -> ShellError {
    ShellError::NushellFailed {
        msg: "A panic occurred on a thread spawned by `tee`".into(),
    }
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
            .name("tee".into())
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

/// "tee" for a single value. No stream handling, just spawns a thread, printing any resulting error
fn tee_once(
    engine_state: Arc<EngineState>,
    on_thread: impl FnOnce() -> Result<(), ShellError> + Send + 'static,
) -> Result<JoinHandle<()>, std::io::Error> {
    thread::Builder::new().name("tee".into()).spawn(move || {
        if let Err(err) = on_thread() {
            report_shell_error(&engine_state, &err);
        }
    })
}

fn stderr_misuse<T>(span: Span, head: Span) -> Result<T, ShellError> {
    Err(ShellError::UnsupportedInput {
        msg: "--stderr can only be used on external commands".into(),
        input: "the input to `tee` is not an external command".into(),
        msg_span: head,
        input_span: span,
    })
}

struct IoTee<R: Read> {
    reader: R,
    sender: Option<Sender<Vec<u8>>>,
    thread: Option<JoinHandle<Result<(), ShellError>>>,
}

impl<R: Read> IoTee<R> {
    fn new(reader: R, tee: TeeThread) -> Self {
        Self {
            reader,
            sender: Some(tee.sender),
            thread: Some(tee.thread),
        }
    }
}

impl<R: Read> Read for IoTee<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if let Some(thread) = self.thread.take() {
            if thread.is_finished() {
                if let Err(err) = thread.join().unwrap_or_else(|_| Err(panic_error())) {
                    return Err(io::Error::other(err));
                }
            } else {
                self.thread = Some(thread)
            }
        }
        let len = self.reader.read(buf)?;
        if len == 0 {
            self.sender = None;
            if let Some(thread) = self.thread.take() {
                if let Err(err) = thread.join().unwrap_or_else(|_| Err(panic_error())) {
                    return Err(io::Error::other(err));
                }
            }
        } else if let Some(sender) = self.sender.as_mut() {
            if sender.send(buf[..len].to_vec()).is_err() {
                self.sender = None;
            }
        }
        Ok(len)
    }
}

struct TeeThread {
    sender: Sender<Vec<u8>>,
    thread: JoinHandle<Result<(), ShellError>>,
}

fn spawn_tee(
    info: StreamInfo,
    mut eval_block: impl FnMut(PipelineData) -> Result<(), ShellError> + Send + 'static,
) -> Result<TeeThread, ShellError> {
    let (sender, receiver) = mpsc::channel();

    let thread = thread::Builder::new()
        .name("tee".into())
        .spawn(move || {
            // We use Signals::empty() here because we assume there already is a Signals on the other side
            let stream = ByteStream::from_iter(
                receiver.into_iter(),
                info.span,
                Signals::empty(),
                info.type_,
            );
            eval_block(PipelineData::byte_stream(stream, info.metadata))
        })
        .map_err(|err| {
            IoError::new_with_additional_context(err, info.span, None, "Could not spawn tee")
        })?;

    Ok(TeeThread { sender, thread })
}

#[derive(Clone)]
struct StreamInfo {
    span: Span,
    signals: Signals,
    type_: ByteStreamType,
    metadata: Option<PipelineMetadata>,
}

fn copy(src: impl Read, dest: impl Write, info: &StreamInfo) -> Result<(), ShellError> {
    copy_with_signals(src, dest, info.span, &info.signals)?;
    Ok(())
}

#[cfg(feature = "os")]
fn copy_pipe(pipe: ChildPipe, dest: impl Write, info: &StreamInfo) -> Result<(), ShellError> {
    match pipe {
        ChildPipe::Pipe(pipe) => copy(pipe, dest, info),
        ChildPipe::Tee(tee) => copy(tee, dest, info),
    }
}

fn copy_on_thread(
    src: impl Read + Send + 'static,
    dest: impl Write + Send + 'static,
    info: &StreamInfo,
) -> Result<JoinHandle<Result<(), ShellError>>, ShellError> {
    let span = info.span;
    let signals = info.signals.clone();
    thread::Builder::new()
        .name("stderr copier".into())
        .spawn(move || {
            copy_with_signals(src, dest, span, &signals)?;
            Ok(())
        })
        .map_err(|err| {
            IoError::new_with_additional_context(err, span, None, "Could not spawn stderr copier")
                .into()
        })
}

#[cfg(feature = "os")]
fn copy_pipe_on_thread(
    pipe: ChildPipe,
    dest: impl Write + Send + 'static,
    info: &StreamInfo,
) -> Result<JoinHandle<Result<(), ShellError>>, ShellError> {
    match pipe {
        ChildPipe::Pipe(pipe) => copy_on_thread(pipe, dest, info),
        ChildPipe::Tee(tee) => copy_on_thread(tee, dest, info),
    }
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
        Err(ShellError::Io(IoError::new_with_additional_context(
            shell_error::io::ErrorKind::from_std(std::io::ErrorKind::Other),
            Span::test_data(),
            None,
            "test",
        )))
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
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    use std::time::Duration;
    let waited = Arc::new(AtomicBool::new(false));
    let waited_clone = waited.clone();
    let iter = tee(0..100, move |_| {
        std::thread::sleep(Duration::from_millis(10));
        waited_clone.store(true, Ordering::Relaxed);
        Err(ShellError::Io(IoError::new_with_additional_context(
            shell_error::io::ErrorKind::from_std(std::io::ErrorKind::Other),
            Span::test_data(),
            None,
            "test",
        )))
    })
    .expect("io error");
    let last = iter.last();
    assert!(waited.load(Ordering::Relaxed), "failed to wait");
    assert!(
        last.is_some_and(|res| res.is_err()),
        "failed to return error from wait"
    );
}
