use nu_engine::{command_prelude::*, get_eval_block_with_early_return};
use nu_protocol::{
    engine::Closure,
    io::{copy_with_interrupt, ReadIterator},
    process::ChildPipe,
    ByteStream, ByteStreamSource, OutDest, PipelineMetadata,
};
use std::{
    io::{self, Read, Write},
    sync::{
        atomic::AtomicBool,
        mpsc::{self, Sender},
        Arc,
    },
    thread::{self, JoinHandle},
};

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
        let head = call.head;
        let use_stderr = call.has_flag(engine_state, stack, "stderr")?;

        let closure: Spanned<Closure> = call.req(engine_state, stack, 0)?;
        let closure_span = closure.span;
        let closure = closure.item;

        let mut eval_block = {
            let closure_engine_state = engine_state.clone();
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
                result.and_then(|data| data.drain())
            }
        };

        if let PipelineData::ByteStream(stream, metadata) = input {
            let span = stream.span();
            let ctrlc = engine_state.ctrlc.clone();

            match stream.into_source() {
                ByteStreamSource::Read(read) => {
                    if use_stderr {
                        return stderr_misuse(span, head);
                    }

                    let (sender, thread) = spawn_tee(&metadata, eval_block, span)?;
                    let tee = IoTee::new(read, sender, thread);

                    Ok(PipelineData::ByteStream(
                        ByteStream::read(tee, span, ctrlc),
                        metadata,
                    ))
                }
                ByteStreamSource::File(file) => {
                    if use_stderr {
                        return stderr_misuse(span, head);
                    }

                    let (sender, thread) = spawn_tee(&metadata, eval_block, span)?;
                    let tee = IoTee::new(file, sender, thread);

                    Ok(PipelineData::ByteStream(
                        ByteStream::read(tee, span, ctrlc),
                        metadata,
                    ))
                }
                ByteStreamSource::Child(mut child) => {
                    let stderr_thread = if use_stderr {
                        let stderr_thread = if let Some(stderr) = child.stderr.take() {
                            match stack.stderr() {
                                OutDest::Pipe | OutDest::Capture => {
                                    let (sender, thread) = spawn_tee(&metadata, eval_block, span)?;
                                    let tee = IoTee::new(stderr, sender, thread);
                                    child.stderr = Some(ChildPipe::Tee(Box::new(tee)));
                                    Ok(None)
                                }
                                OutDest::Null => tee_and_drain_on_thread(
                                    stderr,
                                    io::sink(),
                                    span,
                                    ctrlc.as_ref(),
                                    &metadata,
                                    eval_block,
                                )
                                .map(Some),
                                OutDest::Inherit => tee_and_drain_on_thread(
                                    stderr,
                                    io::stderr(),
                                    span,
                                    ctrlc.as_ref(),
                                    &metadata,
                                    eval_block,
                                )
                                .map(Some),
                                OutDest::File(file) => tee_and_drain_on_thread(
                                    stderr,
                                    file.clone(),
                                    span,
                                    ctrlc.as_ref(),
                                    &metadata,
                                    eval_block,
                                )
                                .map(Some),
                            }?
                        } else {
                            None
                        };

                        if let Some(stdout) = child.stdout.take() {
                            match stack.stdout() {
                                OutDest::Pipe | OutDest::Capture => {
                                    child.stdout = Some(stdout);
                                    Ok(())
                                }
                                OutDest::Null => {
                                    drain_pipe(stdout, io::sink(), span, ctrlc.as_deref())
                                }
                                OutDest::Inherit => {
                                    drain_pipe(stdout, io::stdout(), span, ctrlc.as_deref())
                                }
                                OutDest::File(file) => {
                                    drain_pipe(stdout, file.as_ref(), span, ctrlc.as_deref())
                                }
                            }?;
                        }

                        stderr_thread
                    } else {
                        let stderr_thread = if let Some(stderr) = child.stderr.take() {
                            match stack.stderr() {
                                OutDest::Pipe | OutDest::Capture => {
                                    child.stderr = Some(stderr);
                                    Ok(None)
                                }
                                OutDest::Null => {
                                    copy_on_thread(stderr, io::sink(), span, ctrlc.as_ref())
                                        .map(Some)
                                }
                                OutDest::Inherit => {
                                    copy_on_thread(stderr, io::stderr(), span, ctrlc.as_ref())
                                        .map(Some)
                                }
                                OutDest::File(file) => {
                                    copy_on_thread(stderr, file.clone(), span, ctrlc.as_ref())
                                        .map(Some)
                                }
                            }?
                        } else {
                            None
                        };

                        if let Some(stdout) = child.stdout.take() {
                            match stack.stdout() {
                                OutDest::Pipe | OutDest::Capture => {
                                    let (sender, thread) = spawn_tee(&metadata, eval_block, span)?;
                                    let tee = IoTee::new(stdout, sender, thread);
                                    child.stdout = Some(ChildPipe::Tee(Box::new(tee)));
                                    Ok(())
                                }
                                OutDest::Null => tee_and_drain(
                                    stdout,
                                    io::sink(),
                                    span,
                                    ctrlc,
                                    &metadata,
                                    eval_block,
                                ),
                                OutDest::Inherit => tee_and_drain(
                                    stdout,
                                    io::stdout(),
                                    span,
                                    ctrlc,
                                    &metadata,
                                    eval_block,
                                ),
                                OutDest::File(file) => tee_and_drain(
                                    stdout,
                                    file.as_ref(),
                                    span,
                                    ctrlc,
                                    &metadata,
                                    eval_block,
                                ),
                            }?;
                        }

                        stderr_thread
                    };

                    if child.stdout.is_some() || child.stderr.is_some() {
                        Ok(PipelineData::ByteStream(
                            ByteStream::child(*child, span),
                            metadata,
                        ))
                    } else {
                        if let Some(thread) = stderr_thread {
                            thread.join().unwrap_or_else(|_| Err(panic_error()))?;
                        }
                        child.wait()?.check_ok(span)?;
                        Ok(PipelineData::Empty)
                    }
                }
            }
        } else {
            if use_stderr {
                return stderr_misuse(input.span().unwrap_or(head), head);
            }

            let span = input.span().unwrap_or(head);
            let ctrlc = engine_state.ctrlc.clone();
            let metadata = input.metadata();
            let metadata_clone = metadata.clone();

            Ok(tee(input.into_iter(), move |rx| {
                let input = rx.into_pipeline_data_with_metadata(span, ctrlc, metadata_clone);
                eval_block(input)
            })
            .err_span(call.head)?
            .map(move |result| result.unwrap_or_else(|err| Value::error(err, closure_span)))
            .into_pipeline_data_with_metadata(
                span,
                engine_state.ctrlc.clone(),
                metadata,
            ))
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

fn stderr_misuse<T>(span: Span, head: Span) -> Result<T, ShellError> {
    Err(ShellError::UnsupportedInput {
        msg: "--stderr can only be used on external commands".into(),
        input: "the input to `tee` is not an external commands".into(),
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
    fn new(reader: R, sender: Sender<Vec<u8>>, thread: JoinHandle<Result<(), ShellError>>) -> Self {
        Self {
            reader,
            sender: Some(sender),
            thread: Some(thread),
        }
    }
}

impl<R: Read> Read for IoTee<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if let Some(thread) = self.thread.take() {
            if thread.is_finished() {
                if let Err(err) = thread.join().unwrap_or_else(|_| Err(panic_error())) {
                    return Err(io::Error::new(io::ErrorKind::Other, err));
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
                    return Err(io::Error::new(io::ErrorKind::Other, err));
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

#[allow(clippy::type_complexity)]
fn spawn_tee(
    metadata: &Option<PipelineMetadata>,
    mut eval_block: impl FnMut(PipelineData) -> Result<(), ShellError> + Send + 'static,
    span: Span,
) -> Result<(Sender<Vec<u8>>, JoinHandle<Result<(), ShellError>>), ShellError> {
    let (sender, receiver) = mpsc::channel();

    let meta = metadata.clone();
    let thread = thread::Builder::new()
        .name("tee".into())
        .spawn(move || {
            let stream = ByteStream::read(ReadIterator::new(receiver.into_iter()), span, None);
            eval_block(PipelineData::ByteStream(stream, meta))
        })
        .err_span(span)?;

    Ok((sender, thread))
}

fn drain_teed_pipe(
    child_pipe: ChildPipe,
    mut dest: impl Write,
    sender: Sender<Vec<u8>>,
    thread: JoinHandle<Result<(), ShellError>>,
    span: Span,
    ctrlc: Option<&AtomicBool>,
) -> Result<(), ShellError> {
    match child_pipe {
        ChildPipe::Pipe(pipe) => {
            let mut tee = IoTee::new(pipe, sender, thread);
            copy_with_interrupt(&mut tee, &mut dest, span, ctrlc)?;
        }
        ChildPipe::Tee(tee) => {
            let mut tee = IoTee::new(tee, sender, thread);
            copy_with_interrupt(&mut tee, &mut dest, span, ctrlc)?;
        }
    }
    Ok(())
}

fn tee_and_drain_on_thread(
    pipe: ChildPipe,
    dest: impl Write + Send + 'static,
    span: Span,
    ctrlc: Option<&Arc<AtomicBool>>,
    metadata: &Option<PipelineMetadata>,
    eval_block: impl FnMut(PipelineData) -> Result<(), ShellError> + Send + 'static,
) -> Result<JoinHandle<Result<(), ShellError>>, ShellError> {
    let ctrlc = ctrlc.cloned();
    let (sender, thread) = spawn_tee(metadata, eval_block, span)?;
    thread::Builder::new()
        .name("stderr tee".into())
        .spawn(move || drain_teed_pipe(pipe, dest, sender, thread, span, ctrlc.as_deref()))
        .map_err(|e| e.into_spanned(span).into())
}

fn tee_and_drain(
    pipe: ChildPipe,
    dest: impl Write,
    span: Span,
    ctrlc: Option<Arc<AtomicBool>>,
    metadata: &Option<PipelineMetadata>,
    eval_block: impl FnMut(PipelineData) -> Result<(), ShellError> + Send + 'static,
) -> Result<(), ShellError> {
    let (sender, thread) = spawn_tee(metadata, eval_block, span)?;
    drain_teed_pipe(pipe, dest, sender, thread, span, ctrlc.as_deref())
}

fn drain_pipe(
    child_pipe: ChildPipe,
    mut dest: impl Write,
    span: Span,
    ctrlc: Option<&AtomicBool>,
) -> Result<(), ShellError> {
    match child_pipe {
        ChildPipe::Pipe(mut pipe) => {
            copy_with_interrupt(&mut pipe, &mut dest, span, ctrlc)?;
        }
        ChildPipe::Tee(mut tee) => {
            copy_with_interrupt(&mut tee, &mut dest, span, ctrlc)?;
        }
    }
    Ok(())
}

fn copy_on_thread(
    read: ChildPipe,
    mut write: impl Write + Send + 'static,
    span: Span,
    ctrlc: Option<&Arc<AtomicBool>>,
) -> Result<JoinHandle<Result<(), ShellError>>, ShellError> {
    let ctrlc = ctrlc.cloned();
    thread::Builder::new()
        .name("stderr consumer".into())
        .spawn(move || {
            match read {
                ChildPipe::Pipe(mut pipe) => {
                    copy_with_interrupt(&mut pipe, &mut write, span, ctrlc.as_deref())
                }
                ChildPipe::Tee(mut tee) => {
                    copy_with_interrupt(&mut tee, &mut write, span, ctrlc.as_deref())
                }
            }?;

            Ok(())
        })
        .map_err(|e| e.into_spanned(span).into())
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
