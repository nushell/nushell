//! Passes values between a `nu` and a `nu` it starts with `--structured-io`.
//!
//! The parent asks for it with [`STRUCTURED_IO_FLAG`] on the command line of the `nu` binary, or
//! in the shebang line of a script that `nu` runs. It then opens a local socket and tells the
//! child where to find it in [`STRUCTURED_IO_ENV`](nu_protocol::STRUCTURED_IO_ENV). The child
//! keeps the terminal for its own stdin and stdout, so `print` and `input` work there as they do
//! in a custom command, and passes values over the socket:
//!
//! - its input, when the parent has some, on a connection it makes at startup;
//! - its result, when the parent reads it, on a connection it makes once the result is ready.
//!   Since that is so late, a `nu` it `exec`s into can send the result in its place.
//!
//! Every connection starts with the token from the link, then the role (`i` or `o`) and a
//! newline, so the parent only talks to the process it started. After that the sender writes
//! messages: a tag byte, a little-endian `u64` length and that many bytes.
//!
//! - `N`: nothing.
//! - `V`: one value, as NUON.
//! - `L`: a list stream, then `I` messages holding the items ready at the time as a NUON list,
//!   then `E`.
//! - `B`: a byte stream, with its type as `b` (binary), `s` (string) or `u` (unknown), then `C`
//!   messages holding the bytes, then `E`.
//! - `X`: an error, as text. In a list stream it becomes an error item.
//!
//! The receiver answers the last message with one `K` byte. A child waits for it before it exits,
//! since a named pipe on Windows may lose what the other end hasn't read yet when it closes.

use super::run_external::shebang_words;
use interprocess::local_socket::{
    Listener, Stream,
    traits::{Listener as _, Stream as _},
};
use nu_protocol::{
    ByteStream, ByteStreamType, ListStream, OutDest, PipelineData, STRUCTURED_IO_FLAG, ShellError,
    Signals, Span, Spanned, StructuredIoLink, StructuredIoMode, Value,
    engine::EngineState,
    process::{ChildPipe, ChildProcess},
    shell_error::{bridge::ShellErrorBridge, generic::GenericError, io::IoError},
};
use nu_utils::local_socket::{
    bind_local_socket, cleanup_local_socket_dir, interpret_local_socket_name,
    make_local_socket_name,
};
use nuon::{ToNuonConfig, ToStyle, from_nuon, to_nuon};
use std::{
    collections::VecDeque,
    ffi::{OsStr, OsString},
    io::{self, ErrorKind, Read, Write},
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender},
    thread,
    time::Duration,
};

const NOTHING: u8 = b'N';
const VALUE: u8 = b'V';
const LIST: u8 = b'L';
const ITEMS: u8 = b'I';
const BYTES: u8 = b'B';
const CHUNK: u8 = b'C';
const END: u8 = b'E';
const ERROR: u8 = b'X';
const ACK: u8 = b'K';

const ROLE_INPUT: u8 = b'i';
const ROLE_OUTPUT: u8 = b'o';
/// Sent by the parent itself once the child has exited, so the accepting thread stops waiting.
const ROLE_WAKE: u8 = b'w';

/// Most list items sent in one `I` message. A sender only waits for the first one, so items that
/// come slowly still go one by one.
const MAX_BATCH: usize = 1024;

/// Which ways values go for one spawn: off unless the program asks for structured IO, see
/// [`asks_for_structured_io`].
///
/// Input goes over the channel whenever there is some. Output goes there when this process reads
/// the child's stdout, unless it also reads stderr: `complete`, `save` and `o+e>|` get the child's
/// text as before.
pub fn structured_io_for_child(
    executable: &Path,
    args: &[Spanned<OsString>],
    stdout: &OutDest,
    stderr: &OutDest,
    input: &PipelineData,
) -> StructuredIoMode {
    let stderr_captured = matches!(
        stderr,
        OutDest::Pipe | OutDest::PipeRaw | OutDest::PipeSeparate | OutDest::Value
    );
    let wanted = StructuredIoMode {
        input: !matches!(input, PipelineData::Empty),
        output: matches!(stdout, OutDest::Pipe | OutDest::PipeRaw | OutDest::Value)
            && !stderr_captured,
    };
    // Checked last: for a script it reads the start of the file.
    if wanted.any() && asks_for_structured_io(executable, args) {
        wanted
    } else {
        StructuredIoMode::default()
    }
}

/// Whether a spawn asks for structured IO: the `nu` binary with [`STRUCTURED_IO_FLAG`] among its
/// arguments, or a script whose shebang runs `nu` with it. Other programs never do, whatever their
/// arguments are, so `rg --structured-io` still prints what it finds.
fn asks_for_structured_io(executable: &Path, args: &[Spanned<OsString>]) -> bool {
    if is_nu_binary(executable) {
        return args.iter().any(|arg| arg.item == STRUCTURED_IO_FLAG);
    }
    shebang_words(executable).is_some_and(|words| {
        words.iter().any(|word| word == STRUCTURED_IO_FLAG)
            && words.iter().any(|word| is_nu_binary(Path::new(word)))
    })
}

/// Whether `path` names the `nu` binary.
fn is_nu_binary(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("nu") || name.eq_ignore_ascii_case("nu.exe"))
}

/// The parent's end of the channel for one child: the listening socket, until the child has
/// connected for every way or has exited.
pub struct StructuredIoServer {
    link: StructuredIoLink,
    listener: Listener,
    socket_dir: Option<PathBuf>,
    input: Option<SyncSender<Stream>>,
    output: Option<SyncSender<Stream>>,
}

/// Where the connections of a [`StructuredIoServer`] arrive. A receiver whose sender is gone means
/// the child exited without making that connection.
pub struct StructuredIoConnections {
    pub input: Option<Receiver<Stream>>,
    pub output: Option<Receiver<Stream>>,
}

impl StructuredIoServer {
    /// Opens the socket for `mode`, before the child starts.
    pub fn bind(
        mode: StructuredIoMode,
        span: Span,
    ) -> Result<(Self, StructuredIoConnections), ShellError> {
        let token = format!("{:032x}", rand::random::<u128>());
        // Short, since a Unix socket path has a length limit and temp dirs can be long.
        let socket = make_local_socket_name(&token[..12]);
        let (listener, socket_dir) = bind_local_socket(&socket).map_err(|err| {
            IoError::new_with_additional_context(
                err,
                span,
                None,
                format!("Could not open a socket for --structured-io at {socket:?}"),
            )
        })?;
        let socket = socket.into_string().map_err(|socket| {
            cleanup_local_socket_dir(socket_dir.as_ref());
            ShellError::Generic(GenericError::new(
                "Could not open a channel for --structured-io",
                format!("the socket path {socket:?} is not valid UTF-8"),
                span,
            ))
        })?;
        let channel = || {
            let (sender, receiver) = mpsc::sync_channel(1);
            (Some(sender), Some(receiver))
        };
        let ((input, input_rx), (output, output_rx)) = (
            if mode.input { channel() } else { (None, None) },
            if mode.output { channel() } else { (None, None) },
        );
        let server = Self {
            link: StructuredIoLink {
                token,
                mode,
                socket,
            },
            listener,
            socket_dir,
            input,
            output,
        };
        let connections = StructuredIoConnections {
            input: input_rx,
            output: output_rx,
        };
        Ok((server, connections))
    }

    /// The value for [`STRUCTURED_IO_ENV`](nu_protocol::STRUCTURED_IO_ENV) on the child.
    pub fn env_value(&self) -> String {
        self.link.to_env_value()
    }

    /// A function that stops the accepting thread, for when the child has exited and can't
    /// connect anymore. It connects to the socket itself, so a waiting `accept` returns.
    pub fn waker(&self) -> impl FnOnce() + Send + 'static {
        let link = self.link.clone();
        move || {
            // Fails once the thread has stopped and the socket is gone, which is fine.
            let _ = connect(&link, ROLE_WAKE);
        }
    }

    /// Accepts the child's connections on a thread of its own, now that the child runs. The socket
    /// goes away when the child has connected for every way or has exited.
    pub fn start(self, span: Span) -> Result<(), ShellError> {
        thread::Builder::new()
            .name("structured io listener".into())
            .spawn(move || self.accept())
            .map(|_| ())
            .map_err(|err| IoError::new(err, span, None).into())
    }

    fn accept(mut self) {
        let mut preamble = vec![0; self.link.token.len() + 2];
        while self.input.is_some() || self.output.is_some() {
            let Ok(mut stream) = self.listener.accept() else {
                break;
            };
            // Anything that doesn't start with the token isn't the child, so it's dropped.
            if stream.read_exact(&mut preamble).is_err()
                || !preamble.starts_with(self.link.token.as_bytes())
                || preamble.last() != Some(&b'\n')
            {
                continue;
            }
            let sender = match preamble[self.link.token.len()] {
                ROLE_INPUT => self.input.take(),
                ROLE_OUTPUT => self.output.take(),
                ROLE_WAKE => break,
                _ => None,
            };
            if let Some(sender) = sender {
                let _ = sender.send(stream);
            }
        }
    }
}

impl Drop for StructuredIoServer {
    fn drop(&mut self) {
        cleanup_local_socket_dir(self.socket_dir.as_ref());
    }
}

/// Connects to the parent's socket for `role`.
fn connect(link: &StructuredIoLink, role: u8) -> io::Result<Stream> {
    let mut stream = Stream::connect(interpret_local_socket_name(OsStr::new(&link.socket))?)?;
    stream.write_all(link.token.as_bytes())?;
    stream.write_all(&[role, b'\n'])?;
    Ok(stream)
}

/// Reads the child's result once it connects with it. `child` is waited for, so a child that fails
/// is reported like any external; for a byte stream, that happens when the stream ends.
///
/// A child that exits without connecting (`exit`, or a program that isn't `nu` after all) gives
/// nothing.
pub fn read_structured_output(
    child: ChildProcess,
    output: Receiver<Stream>,
    signals: &Signals,
    span: Span,
) -> Result<PipelineData, ShellError> {
    let stream = loop {
        match output.recv_timeout(Duration::from_millis(100)) {
            Ok(stream) => break Some(stream),
            Err(RecvTimeoutError::Timeout) => signals.check(&span)?,
            Err(RecvTimeoutError::Disconnected) => break None,
        }
    };
    match stream {
        Some(stream) => receive_pipeline(stream, Some(child), signals, span),
        None => child.wait().map(|()| PipelineData::empty()),
    }
}

/// Reads the input a parent sends this process at startup.
pub fn read_structured_input(
    link: &StructuredIoLink,
    signals: &Signals,
) -> Result<PipelineData, ShellError> {
    let stream = connect(link, ROLE_INPUT)
        .map_err(|err| IoError::new_internal(err, "Could not connect to the calling nu"))?;
    receive_pipeline(stream, None, signals, Span::unknown())
}

/// Sends this process's result to the `nu` that started it.
///
/// Returns quietly when that `nu` stopped reading, as a program does on a closed pipe.
pub fn send_structured_output(
    engine_state: &EngineState,
    link: &StructuredIoLink,
    data: PipelineData,
) -> Result<(), ShellError> {
    let mut stream = connect(link, ROLE_OUTPUT)
        .map_err(|err| IoError::new_internal(err, "Could not connect to the calling nu"))?;
    let sent = send_pipeline(engine_state, data, &mut stream);
    // Wait for the parent to take the last message before closing the channel.
    let _ = stream.read(&mut [0]);
    match sent {
        Err(SendError::Closed) => Ok(()),
        Err(SendError::Shell(err)) => Err(err),
        Ok(()) => Ok(()),
    }
}

/// Sends `data` to a child that connected for its input. The child closing its end early isn't an
/// error: it only means it stopped reading.
pub fn send_structured_input(
    engine_state: &EngineState,
    data: PipelineData,
    input: &Receiver<Stream>,
) -> Result<(), ShellError> {
    // No connection means the child exited first, so the input has nowhere to go.
    let Ok(mut stream) = input.recv() else {
        return Ok(());
    };
    match send_pipeline(engine_state, data, &mut stream) {
        Err(SendError::Shell(err)) => Err(err),
        Err(SendError::Closed) | Ok(()) => Ok(()),
    }
}

/// Why sending stopped.
enum SendError {
    /// The receiver closed the channel.
    Closed,
    /// Writing failed, or the data did (an external that failed).
    Shell(ShellError),
}

impl From<io::Error> for SendError {
    fn from(err: io::Error) -> Self {
        if is_closed(&err) {
            SendError::Closed
        } else {
            SendError::Shell(IoError::new_internal(err, "Could not send to another nu").into())
        }
    }
}

/// Whether `err` comes from the other end of the channel closing.
fn is_closed(err: &io::Error) -> bool {
    // A named pipe whose other end disconnected; the standard library has no kind for it.
    #[cfg(windows)]
    if err.raw_os_error() == Some(windows::Win32::Foundation::ERROR_PIPE_NOT_CONNECTED.0 as i32) {
        return true;
    }
    matches!(
        err.kind(),
        ErrorKind::BrokenPipe | ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted
    )
}

fn send_pipeline(
    engine_state: &EngineState,
    data: PipelineData,
    stream: &mut Stream,
) -> Result<(), SendError> {
    match data {
        PipelineData::Empty => Ok(write_message(stream, NOTHING, &[])?),
        PipelineData::Value(value, ..) => {
            match encode(engine_state, &value) {
                Ok(nuon) => write_message(stream, VALUE, nuon.as_bytes())?,
                Err(err) => write_message(stream, ERROR, err.as_bytes())?,
            }
            Ok(())
        }
        PipelineData::ListStream(list, ..) => send_list(engine_state, list, stream),
        PipelineData::ByteStream(bytes, ..) => send_bytes(bytes, stream),
    }
}

/// Sends the items of `list` as they come, in batches of the ones ready at the time.
fn send_list(
    engine_state: &EngineState,
    list: ListStream,
    stream: &mut Stream,
) -> Result<(), SendError> {
    write_message(stream, LIST, &[])?;
    // Items come from their own thread, so a batch can take whatever is ready without waiting for
    // more. The thread stops at the next item once the receiver goes away.
    let (sender, receiver) = mpsc::sync_channel(MAX_BATCH);
    thread::Builder::new()
        .name("structured io list sender".into())
        .spawn(move || {
            for item in list {
                if sender.send(item).is_err() {
                    break;
                }
            }
        })
        .map_err(|err| {
            SendError::Shell(IoError::new_internal(err, "Could not send a list").into())
        })?;
    while let Ok(first) = receiver.recv() {
        let mut batch = vec![first];
        while batch.len() < MAX_BATCH
            && let Ok(item) = receiver.try_recv()
        {
            batch.push(item);
        }
        send_batch(engine_state, batch, stream)?;
    }
    write_message(stream, END, &[])?;
    Ok(())
}

/// Sends `batch` as one `I` message, or, when some items have no NUON form or are errors, each
/// item apart, with those as `X` messages.
fn send_batch(
    engine_state: &EngineState,
    batch: Vec<Value>,
    stream: &mut Stream,
) -> Result<(), SendError> {
    let batch = Value::list(batch, Span::unknown());
    if let Ok(nuon) = encode(engine_state, &batch) {
        return Ok(write_message(stream, ITEMS, nuon.as_bytes())?);
    }
    for item in batch.into_list().unwrap_or_default() {
        match encode(engine_state, &Value::list(vec![item], Span::unknown())) {
            Ok(nuon) => write_message(stream, ITEMS, nuon.as_bytes())?,
            Err(err) => write_message(stream, ERROR, err.as_bytes())?,
        }
    }
    Ok(())
}

/// Sends the bytes of `bytes` as they come. If reading them fails, or `bytes` is an external that
/// failed, the stream ends with that error instead of `E`, and so does this call.
fn send_bytes(bytes: ByteStream, stream: &mut Stream) -> Result<(), SendError> {
    let type_ = match bytes.type_() {
        ByteStreamType::Binary => b'b',
        ByteStreamType::String => b's',
        ByteStreamType::Unknown => b'u',
    };
    write_message(stream, BYTES, &[type_])?;
    let mut writer = ChunkWriter {
        stream: &mut *stream,
        closed: false,
    };
    let written = bytes.write_to(&mut writer);
    if writer.closed {
        return Err(SendError::Closed);
    }
    match written {
        Ok(()) => Ok(write_message(stream, END, &[])?),
        Err(err) => {
            write_message(stream, ERROR, err.to_string().as_bytes())?;
            Err(SendError::Shell(err))
        }
    }
}

/// NUON for `value`, or the text of an error saying why there is none.
fn encode(engine_state: &EngineState, value: &Value) -> Result<String, String> {
    let error = match value {
        Value::Error { error, .. } => Some(error.to_string()),
        Value::List { vals, .. } => vals.iter().find_map(|item| match item {
            Value::Error { error, .. } => Some(error.to_string()),
            _ => None,
        }),
        _ => None,
    };
    if let Some(error) = error {
        return Err(error);
    }
    to_nuon(
        engine_state,
        value,
        ToNuonConfig::default().style(ToStyle::Raw),
    )
    .map_err(|err| format!("can't pass a {} to another nu: {err}", value.get_type()))
}

/// Writes each write as a `C` message.
struct ChunkWriter<'a> {
    stream: &'a mut Stream,
    /// Set when the receiver closed the channel, which the byte stream reports as just an error.
    closed: bool,
}

impl Write for ChunkWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        write_message(self.stream, CHUNK, buf).inspect_err(|err| self.closed = is_closed(err))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Reads what the other end sends as pipeline data. A list or byte stream keeps reading the
/// channel as it's consumed. `child`, the process at the other end, is waited for once all of it
/// is read.
fn receive_pipeline(
    mut stream: Stream,
    child: Option<ChildProcess>,
    signals: &Signals,
    span: Span,
) -> Result<PipelineData, ShellError> {
    let message = read_message(&mut stream).map_err(|err| receive_error(err, span))?;
    let finish = |stream: &mut Stream, child: Option<ChildProcess>| {
        acknowledge(stream);
        child.map_or(Ok(()), ChildProcess::wait)
    };
    match message {
        None => finish(&mut stream, child).map(|()| PipelineData::empty()),
        Some((NOTHING, _)) => finish(&mut stream, child).map(|()| PipelineData::empty()),
        Some((VALUE, nuon)) => {
            let value = decode(&nuon, span);
            finish(&mut stream, child)?;
            Ok(PipelineData::value(value?, None))
        }
        Some((ERROR, text)) => {
            // The error says more than the exit code that follows it.
            let _ = finish(&mut stream, child);
            Err(remote_error(&text, span))
        }
        Some((LIST, _)) => {
            let items = ListReceiver {
                stream: Some(stream),
                child,
                items: VecDeque::new(),
                span,
            };
            Ok(PipelineData::list_stream(
                ListStream::new(items, span, signals.clone()),
                None,
            ))
        }
        Some((BYTES, type_)) => {
            let type_ = match type_.first() {
                Some(b'b') => ByteStreamType::Binary,
                Some(b's') => ByteStreamType::String,
                _ => ByteStreamType::Unknown,
            };
            let reader = ChunkReader {
                stream: Some(stream),
                chunk: Vec::new(),
                at: 0,
                span,
            };
            let bytes = match child {
                Some(mut child) => {
                    child.stdout = Some(ChildPipe::Tee(Box::new(reader)));
                    ByteStream::child(child, span)
                }
                None => ByteStream::read(reader, span, signals.clone(), type_),
            };
            Ok(PipelineData::byte_stream(bytes.with_type(type_), None))
        }
        Some((tag, _)) => {
            let _ = finish(&mut stream, child);
            Err(protocol_error(tag, span))
        }
    }
}

/// The items of a list stream from another `nu`.
struct ListReceiver {
    /// `None` once the list ended.
    stream: Option<Stream>,
    child: Option<ChildProcess>,
    items: VecDeque<Value>,
    span: Span,
}

impl ListReceiver {
    /// Ends the list. A child that failed gives one more item, its error.
    fn finish(&mut self) -> Option<Value> {
        self.stream = None;
        let child = self.child.take()?;
        child.wait().err().map(|err| Value::error(err, self.span))
    }
}

impl Iterator for ListReceiver {
    type Item = Value;

    fn next(&mut self) -> Option<Value> {
        loop {
            if let Some(item) = self.items.pop_front() {
                return Some(item);
            }
            let stream = self.stream.as_mut()?;
            match read_message(stream) {
                Ok(Some((ITEMS, nuon))) => {
                    match decode(&nuon, self.span).and_then(Value::into_list) {
                        Ok(items) => self.items.extend(items),
                        Err(err) => {
                            let _ = self.finish();
                            return Some(Value::error(err, self.span));
                        }
                    }
                }
                Ok(Some((ERROR, text))) => {
                    return Some(Value::error(remote_error(&text, self.span), self.span));
                }
                Ok(Some((END, _))) => {
                    acknowledge(stream);
                    return self.finish();
                }
                // The other end went away, which its exit status explains, if anything.
                Ok(None) => return self.finish(),
                Ok(Some((tag, _))) => {
                    let _ = self.finish();
                    return Some(Value::error(protocol_error(tag, self.span), self.span));
                }
                Err(err) => {
                    let _ = self.finish();
                    return Some(Value::error(receive_error(err, self.span), self.span));
                }
            }
        }
    }
}

/// The bytes of a byte stream from another `nu`.
struct ChunkReader {
    /// `None` once the stream ended.
    stream: Option<Stream>,
    chunk: Vec<u8>,
    at: usize,
    span: Span,
}

impl Read for ChunkReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            if self.at < self.chunk.len() {
                let n = buf.len().min(self.chunk.len() - self.at);
                buf[..n].copy_from_slice(&self.chunk[self.at..self.at + n]);
                self.at += n;
                return Ok(n);
            }
            let Some(stream) = self.stream.as_mut() else {
                return Ok(0);
            };
            match read_message(stream)? {
                Some((CHUNK, chunk)) => {
                    self.chunk = chunk;
                    self.at = 0;
                }
                Some((END, _)) => {
                    acknowledge(stream);
                    self.stream = None;
                }
                // The other end went away, which its exit status explains, if anything.
                None => self.stream = None,
                Some((ERROR, text)) => {
                    self.stream = None;
                    return Err(ShellErrorBridge(remote_error(&text, self.span)).into());
                }
                Some((tag, _)) => {
                    self.stream = None;
                    return Err(ShellErrorBridge(protocol_error(tag, self.span)).into());
                }
            }
        }
    }
}

/// Tells the sender its last message arrived.
fn acknowledge(stream: &mut Stream) {
    let _ = stream.write_all(&[ACK]);
}

fn write_message(writer: &mut impl Write, tag: u8, payload: &[u8]) -> io::Result<()> {
    let mut header = [0; 9];
    header[0] = tag;
    header[1..].copy_from_slice(&(payload.len() as u64).to_le_bytes());
    writer.write_all(&header)?;
    writer.write_all(payload)
}

/// Reads one message, or `None` when the other end closed the channel between messages.
fn read_message(reader: &mut impl Read) -> io::Result<Option<(u8, Vec<u8>)>> {
    let mut header = [0; 9];
    let first = loop {
        match reader.read(&mut header[..1]) {
            Ok(n) => break n,
            Err(err) if err.kind() == ErrorKind::Interrupted => {}
            Err(err) if is_closed(&err) => return Ok(None),
            Err(err) => return Err(err),
        }
    };
    if first == 0 {
        return Ok(None);
    }
    reader.read_exact(&mut header[1..])?;
    let len = u64::from_le_bytes(header[1..].try_into().unwrap_or_default());
    let mut payload = Vec::new();
    reader.take(len).read_to_end(&mut payload)?;
    if payload.len() as u64 != len {
        return Err(ErrorKind::UnexpectedEof.into());
    }
    Ok(Some((header[0], payload)))
}

fn decode(nuon: &[u8], span: Span) -> Result<Value, ShellError> {
    let text = std::str::from_utf8(nuon).map_err(|err| {
        ShellError::Generic(GenericError::new(
            "Could not read a value from another nu",
            err.to_string(),
            span,
        ))
    })?;
    from_nuon(text, Some(span))
}

/// An error that the other `nu` sent.
fn remote_error(text: &[u8], span: Span) -> ShellError {
    ShellError::Generic(GenericError::new(
        "Error from another nu",
        String::from_utf8_lossy(text).into_owned(),
        span,
    ))
}

fn receive_error(err: io::Error, span: Span) -> ShellError {
    IoError::new_with_additional_context(err, span, None, "Could not read from another nu").into()
}

fn protocol_error(tag: u8, span: Span) -> ShellError {
    ShellError::Generic(GenericError::new(
        "Could not read a value from another nu",
        format!("unexpected message `{}`", tag as char),
        span,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_protocol::IntoSpanned;
    use std::io::Cursor;
    use tempfile::NamedTempFile;

    fn args(args: &[&str]) -> Vec<Spanned<OsString>> {
        args.iter()
            .map(|arg| OsString::from(arg).into_spanned(Span::test_data()))
            .collect()
    }

    #[test]
    fn only_nu_with_the_flag_asks() {
        let nu = Path::new("/usr/bin/nu");
        assert!(asks_for_structured_io(
            nu,
            &args(&["--structured-io", "-c", "1"])
        ));
        assert!(asks_for_structured_io(
            Path::new("/opt/nu/NU.EXE"),
            &args(&["--structured-io", "x.nu"])
        ));
        assert!(!asks_for_structured_io(nu, &args(&["-c", "1"])));
        assert!(!asks_for_structured_io(
            nu,
            &args(&["-c", "1 --structured-io"])
        ));
        assert!(!asks_for_structured_io(
            Path::new("/usr/bin/rg"),
            &args(&["--structured-io", "src"])
        ));
    }

    #[cfg(unix)]
    #[test]
    fn nu_shebang_with_the_flag_asks() {
        for (shebang, asks) in [
            ("#!/usr/bin/env -S nu --structured-io", true),
            ("#!/opt/homebrew/bin/nu --structured-io", true),
            ("#!/usr/bin/env nu", false),
            ("#!/usr/bin/env -S python3 --structured-io", false),
        ] {
            let mut file = NamedTempFile::new().expect("tempfile");
            writeln!(file, "{shebang}").expect("write");
            assert_eq!(
                asks_for_structured_io(file.path(), &args(&[])),
                asks,
                "{shebang}"
            );
        }
    }

    #[test]
    fn ways_follow_the_pipeline() {
        let nu = Path::new("nu");
        let flag = args(&["--structured-io", "-c", "1"]);
        let mode = |stdout: OutDest, stderr: OutDest, input: PipelineData| {
            structured_io_for_child(nu, &flag, &stdout, &stderr, &input)
        };
        let value = || PipelineData::value(Value::test_int(1), None);
        let empty = PipelineData::empty;
        assert_eq!(
            mode(OutDest::Pipe, OutDest::Inherit, value()),
            StructuredIoMode {
                input: true,
                output: true
            }
        );
        assert!(mode(OutDest::PipeRaw, OutDest::Inherit, empty()).output);
        assert!(mode(OutDest::Value, OutDest::Null, empty()).output);
        // At the end of a pipeline, `complete`, `o+e>|`.
        assert!(!mode(OutDest::Inherit, OutDest::Inherit, empty()).any());
        assert!(!mode(OutDest::PipeSeparate, OutDest::PipeSeparate, empty()).any());
        assert!(!mode(OutDest::Pipe, OutDest::Pipe, empty()).any());
        assert!(mode(OutDest::Inherit, OutDest::Inherit, value()).input);
    }

    #[test]
    fn messages_roundtrip() {
        let mut buffer = Vec::new();
        write_message(&mut buffer, VALUE, b"[1, 2]").expect("write");
        write_message(&mut buffer, END, &[]).expect("write");
        let mut reader = Cursor::new(buffer);
        assert_eq!(
            read_message(&mut reader).expect("read"),
            Some((VALUE, b"[1, 2]".to_vec()))
        );
        assert_eq!(
            read_message(&mut reader).expect("read"),
            Some((END, vec![]))
        );
        assert_eq!(read_message(&mut reader).expect("read"), None);
    }

    #[test]
    fn truncated_message_is_an_error() {
        let mut buffer = Vec::new();
        write_message(&mut buffer, VALUE, b"[1, 2]").expect("write");
        buffer.truncate(buffer.len() - 2);
        assert!(read_message(&mut Cursor::new(buffer)).is_err());
    }

    #[test]
    fn values_without_nuon_are_errors() {
        let engine_state = EngineState::new();
        let closure = Value::test_closure(nu_protocol::engine::Closure {
            block_id: nu_protocol::BlockId::new(0),
            captures: vec![],
        });
        assert!(encode(&engine_state, &closure).is_err());
        assert_eq!(
            encode(&engine_state, &Value::test_list(vec![Value::test_int(1)])).as_deref(),
            Ok("[1]")
        );
    }
}
