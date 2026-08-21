use nu_engine::command_prelude::*;
use nu_protocol::ListStream;
use nu_protocol::OutDest;
use nu_protocol::Signals;
use nu_protocol::process::{ChildPipe, ChildProcess, ExitStatusGuard};
use nu_protocol::shell_error::generic::GenericError;
use nu_protocol::shell_error::io::IoError;
use std::io::{self, Read};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread::{self, JoinHandle};

const CHUNK_SIZE: usize = 8192;

#[derive(Clone)]
pub struct CompleteStream;

impl Command for CompleteStream {
    fn name(&self) -> &str {
        "complete stream"
    }

    fn signature(&self) -> Signature {
        Signature::build("complete stream")
            .category(Category::System)
            .input_output_types(vec![(
                Type::Any,
                Type::List(Box::new(Type::Record(
                    vec![("stream".into(), Type::String), ("chunk".into(), Type::Any)].into(),
                ))),
            )])
            .switch(
                "lines",
                "Split each stream on newlines instead of emitting raw read chunks.",
                Some('l'),
            )
            .allow_variants_without_examples(true)
    }

    fn description(&self) -> &str {
        "Stream tagged chunks from an external command's stdout and stderr as they arrive."
    }

    fn extra_description(&self) -> &str {
        r#"Only external commands can be piped into `complete stream`. Like `complete`, this command
requests both stdout and stderr as separate pipes.

Each output record has a `stream` column (`stdout` or `stderr`) and a `chunk` column. By default
`chunk` is whatever one `read` returned, at most 8192 bytes, typed as a string when the bytes are
valid UTF-8 and as binary otherwise.

`--lines` splits each pipe independently on `\n` (also stripping a preceding `\r`), keeps empty
lines, and emits a leftover fragment at EOF. A line longer than 8192 bytes is emitted in 8192-byte
fragments so a newline-free infinite producer cannot hang `| first`.

Order is read-arrival order of those chunks, not the child's original write order. Non-zero exit
status does not fail the pipeline. After the stream is fully consumed (or dropped),
`$env.LAST_EXIT_CODE` is set to the child's status. Dropping the stream early (for example
`| first`) closes the pipes so an infinite producer exits instead of hanging.

A wrapper command must take the external as pipeline input (`def cs [] { complete stream }`).
Writing `$in | complete stream` collects the child first and loses the separate pipes."#
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["stdout", "stderr", "chunk", "complete"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let split_lines = call.has_flag(engine_state, stack, "lines")?;

        match input {
            PipelineData::ByteStream(stream, metadata) => {
                let Ok(mut child) = stream.into_child() else {
                    return Err(not_external(head));
                };

                child.ignore_error(true);

                let exit = ExitStatusGuard::new(
                    child.clone_exit_status_future(),
                    child.clone_ignore_error(),
                )
                .with_span(head)
                .with_record_last_exit();

                let iter = spawn_readers(child, split_lines, head, engine_state.signals().clone())?;
                let stream = ListStream::new(iter, head, engine_state.signals().clone())
                    .with_exit_status(exit);
                Ok(PipelineData::list_stream(stream, metadata))
            }
            PipelineData::Value(Value::Error { error, .. }, _) => Err(*error),
            _ => Err(not_external(head)),
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Tag stdout and stderr chunks from an external command as they arrive",
                example: "nu -c 'print out; print -e err' | complete stream",
                result: None,
            },
            Example {
                description: "Split each stream on newlines",
                example: "nu -c 'print out; print -e err' | complete stream --lines",
                result: None,
            },
        ]
    }

    fn pipe_redirection(&self) -> (Option<OutDest>, Option<OutDest>) {
        (Some(OutDest::PipeSeparate), Some(OutDest::PipeSeparate))
    }
}

fn not_external(span: Span) -> ShellError {
    ShellError::Generic(GenericError::new(
        "complete stream only works with external commands",
        "complete stream only works on external commands",
        span,
    ))
}

enum StreamEvent {
    Chunk {
        stream: &'static str,
        bytes: Vec<u8>,
    },
    IoError(io::Error),
}

struct CompleteStreamIter {
    rx: Option<Receiver<StreamEvent>>,
    child: Option<ChildProcess>,
    threads: Vec<JoinHandle<()>>,
    span: Span,
    finished: bool,
}

impl Iterator for CompleteStreamIter {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        let Some(rx) = self.rx.as_ref() else {
            self.finish();
            return None;
        };

        match rx.recv() {
            Ok(StreamEvent::Chunk { stream, bytes }) => {
                Some(chunk_record(stream, bytes, self.span))
            }
            Ok(StreamEvent::IoError(err)) => Some(Value::error(
                IoError::new(err, self.span, None).into(),
                self.span,
            )),
            Err(_) => {
                self.finish();
                None
            }
        }
    }
}

impl Drop for CompleteStreamIter {
    fn drop(&mut self) {
        self.finish();
    }
}

impl CompleteStreamIter {
    fn finish(&mut self) {
        if self.finished {
            return;
        }
        self.finished = true;
        // Disconnect the consumer so reader threads unblock on `send`. They drop their
        // pipes instead of copying remaining bytes, so an infinite producer (`yes`, `iecho`)
        // gets EPIPE/SIGPIPE and exits instead of being drained forever.
        self.rx.take();
        for handle in self.threads.drain(..) {
            let _ = handle.join();
        }
        if let Some(child) = self.child.take() {
            let _ = child.wait_with_output();
        }
    }
}

fn spawn_readers(
    mut child: ChildProcess,
    split_lines: bool,
    span: Span,
    signals: Signals,
) -> Result<CompleteStreamIter, ShellError> {
    let (tx, rx) = mpsc::sync_channel(0);

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let mut threads = Vec::new();

    if let Some(stdout) = stdout {
        threads.push(spawn_reader(
            stdout,
            "stdout",
            tx.clone(),
            split_lines,
            signals.clone(),
            span,
        )?);
    }
    if let Some(stderr) = stderr {
        match spawn_reader(stderr, "stderr", tx.clone(), split_lines, signals, span) {
            Ok(handle) => threads.push(handle),
            Err(err) => {
                // Unblock a stdout reader that may already be waiting on `send`.
                drop(tx);
                drop(rx);
                for handle in threads {
                    let _ = handle.join();
                }
                let _ = child.wait_with_output();
                return Err(err);
            }
        }
    }
    // Drop the extra sender so `rx` disconnects when both reader threads finish.
    drop(tx);

    Ok(CompleteStreamIter {
        rx: Some(rx),
        child: Some(child),
        threads,
        span,
        finished: false,
    })
}

fn spawn_reader(
    pipe: ChildPipe,
    stream: &'static str,
    tx: SyncSender<StreamEvent>,
    split_lines: bool,
    signals: Signals,
    span: Span,
) -> Result<JoinHandle<()>, ShellError> {
    thread::Builder::new()
        .name(format!("complete stream {stream}"))
        .spawn(move || read_pipe(pipe, stream, tx, split_lines, signals))
        .map_err(|err| IoError::new(err, span, None).into())
}

fn read_pipe(
    mut pipe: ChildPipe,
    stream: &'static str,
    tx: SyncSender<StreamEvent>,
    split_lines: bool,
    signals: Signals,
) {
    let mut buf = [0_u8; CHUNK_SIZE];
    let mut leftover = Vec::new();

    loop {
        if signals.interrupted() {
            return;
        }

        match pipe.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                let bytes = &buf[..n];
                if split_lines {
                    leftover.extend_from_slice(bytes);
                    if !emit_lines(&mut leftover, stream, &tx)
                        || !emit_oversize_leftover(&mut leftover, stream, &tx)
                    {
                        return;
                    }
                } else if tx
                    .send(StreamEvent::Chunk {
                        stream,
                        bytes: bytes.to_vec(),
                    })
                    .is_err()
                {
                    return;
                }
            }
            Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
            Err(err) => {
                let _ = tx.send(StreamEvent::IoError(err));
                return;
            }
        }
    }

    if split_lines && !leftover.is_empty() {
        let _ = tx.send(StreamEvent::Chunk {
            stream,
            bytes: leftover,
        });
    }
}

/// Emit leftover bytes that never saw a newline so `| first` cannot hang on
/// a newline-free infinite producer.
fn emit_oversize_leftover(
    leftover: &mut Vec<u8>,
    stream: &'static str,
    tx: &SyncSender<StreamEvent>,
) -> bool {
    while leftover.len() >= CHUNK_SIZE {
        let chunk: Vec<u8> = leftover.drain(..CHUNK_SIZE).collect();
        if tx
            .send(StreamEvent::Chunk {
                stream,
                bytes: chunk,
            })
            .is_err()
        {
            leftover.clear();
            return false;
        }
    }
    true
}

/// Returns `false` if the consumer dropped the channel.
fn emit_lines(leftover: &mut Vec<u8>, stream: &'static str, tx: &SyncSender<StreamEvent>) -> bool {
    while let Some(idx) = leftover.iter().position(|&b| b == b'\n') {
        let mut line: Vec<u8> = leftover.drain(..=idx).collect();
        strip_newline_bytes(&mut line);
        if tx
            .send(StreamEvent::Chunk {
                stream,
                bytes: line,
            })
            .is_err()
        {
            leftover.clear();
            return false;
        }
    }
    true
}

fn strip_newline_bytes(line: &mut Vec<u8>) {
    if line.last() == Some(&b'\n') {
        line.pop();
        if line.last() == Some(&b'\r') {
            line.pop();
        }
    }
}

fn chunk_record(stream: &str, bytes: Vec<u8>, span: Span) -> Value {
    Value::record(
        record! {
            "stream" => Value::string(stream, span),
            "chunk" => bytes_to_chunk(bytes, span),
        },
        span,
    )
}

fn bytes_to_chunk(bytes: Vec<u8>, span: Span) -> Value {
    match String::from_utf8(bytes) {
        Ok(string) => Value::string(string, span),
        Err(err) => Value::binary(err.into_bytes(), span),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(CompleteStream)
    }
}
