use crate::{
    process::{ChildPipe, ChildProcess, ExitStatus},
    ErrSpan, IntoSpanned, OutDest, PipelineData, ShellError, Span, Value,
};
#[cfg(unix)]
use std::os::fd::OwnedFd;
#[cfg(windows)]
use std::os::windows::io::OwnedHandle;
use std::{
    fmt::Debug,
    fs::File,
    io::{self, BufRead, BufReader, Cursor, ErrorKind, Read, Write},
    process::Stdio,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

/// The source of bytes for a [`ByteStream`].
///
/// Currently, there are only three possibilities:
/// 1. `Read` (any `dyn` type that implements [`Read`])
/// 2. [`File`]
/// 3. [`ChildProcess`]
pub enum ByteStreamSource {
    Read(Box<dyn Read + Send + 'static>),
    File(File),
    Child(Box<ChildProcess>),
}

impl ByteStreamSource {
    fn reader(self) -> Option<SourceReader> {
        match self {
            ByteStreamSource::Read(read) => Some(SourceReader::Read(read)),
            ByteStreamSource::File(file) => Some(SourceReader::File(file)),
            ByteStreamSource::Child(mut child) => child.stdout.take().map(|stdout| match stdout {
                ChildPipe::Pipe(pipe) => SourceReader::File(convert_file(pipe)),
                ChildPipe::Tee(tee) => SourceReader::Read(tee),
            }),
        }
    }
}

enum SourceReader {
    Read(Box<dyn Read + Send + 'static>),
    File(File),
}

impl Read for SourceReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            SourceReader::Read(reader) => reader.read(buf),
            SourceReader::File(file) => file.read(buf),
        }
    }
}

/// A potentially infinite, interruptible stream of bytes.
///
/// The data of a [`ByteStream`] can be accessed using one of the following methods:
/// - [`reader`](ByteStream::reader): returns a [`Read`]-able type to get the raw bytes in the stream.
/// - [`lines`](ByteStream::lines): splits the bytes on lines and returns an [`Iterator`]
///   where each item is a `Result<String, ShellError>`.
/// - [`chunks`](ByteStream::chunks): returns an [`Iterator`] of [`Value`]s where each value is either a string or binary.
///   Try not to use this method if possible. Rather, please use [`reader`](ByteStream::reader)
///   (or [`lines`](ByteStream::lines) if it matches the situation).
///
/// Additionally, there are few methods to collect a [`Bytestream`] into memory:
/// - [`into_bytes`](ByteStream::into_bytes): collects all bytes into a [`Vec<u8>`].
/// - [`into_string`](ByteStream::into_string): collects all bytes into a [`String`], erroring if utf-8 decoding failed.
/// - [`into_value`](ByteStream::into_value): collects all bytes into a string [`Value`].
///   If utf-8 decoding failed, then a binary [`Value`] is returned instead.
///
/// There are also a few other methods to consume all the data of a [`Bytestream`]:
/// - [`drain`](ByteStream::drain): consumes all bytes and outputs nothing.
/// - [`write_to`](ByteStream::write_to): writes all bytes to the given [`Write`] destination.
/// - [`print`](ByteStream::print): a convenience wrapper around [`write_to`](ByteStream::write_to).
///   It prints all bytes to stdout or stderr.
///
/// Internally, [`ByteStream`]s currently come in three flavors according to [`ByteStreamSource`].
/// See its documentation for more information.
pub struct ByteStream {
    stream: ByteStreamSource,
    span: Span,
    ctrlc: Option<Arc<AtomicBool>>,
    known_size: Option<u64>,
}

impl ByteStream {
    /// Create a new [`ByteStream`] from a [`ByteStreamSource`].
    pub fn new(stream: ByteStreamSource, span: Span, interrupt: Option<Arc<AtomicBool>>) -> Self {
        Self {
            stream,
            span,
            ctrlc: interrupt,
            known_size: None,
        }
    }

    /// Create a new [`ByteStream`] from a [`ByteStreamSource::Read`].
    pub fn read(
        reader: impl Read + Send + 'static,
        span: Span,
        interrupt: Option<Arc<AtomicBool>>,
    ) -> Self {
        Self::new(ByteStreamSource::Read(Box::new(reader)), span, interrupt)
    }

    /// Create a new [`ByteStream`] from a [`ByteStreamSource::File`].
    pub fn file(file: File, span: Span, interrupt: Option<Arc<AtomicBool>>) -> Self {
        Self::new(ByteStreamSource::File(file), span, interrupt)
    }

    /// Create a new [`ByteStream`] from a [`ByteStreamSource::Child`].
    pub fn child(child: ChildProcess, span: Span) -> Self {
        Self::new(ByteStreamSource::Child(Box::new(child)), span, None)
    }

    /// Create a new [`ByteStream`] that reads from stdin.
    pub fn stdin(span: Span) -> Result<Self, ShellError> {
        let stdin = os_pipe::dup_stdin().err_span(span)?;
        let source = ByteStreamSource::File(convert_file(stdin));
        Ok(Self::new(source, span, None))
    }

    /// Create a new [`ByteStream`] from an [`Iterator`] of bytes slices.
    ///
    /// The returned [`ByteStream`] will have a [`ByteStreamSource`] of `Read`.
    pub fn from_iter<I>(iter: I, span: Span, interrupt: Option<Arc<AtomicBool>>) -> Self
    where
        I: IntoIterator,
        I::IntoIter: Send + 'static,
        I::Item: AsRef<[u8]> + Default + Send + 'static,
    {
        let iter = iter.into_iter();
        let cursor = Some(Cursor::new(I::Item::default()));
        Self::read(ReadIterator { iter, cursor }, span, interrupt)
    }

    /// Create a new [`ByteStream`] from an [`Iterator`] of [`Result`] bytes slices.
    ///
    /// The returned [`ByteStream`] will have a [`ByteStreamSource`] of `Read`.
    pub fn from_result_iter<I, T>(iter: I, span: Span, interrupt: Option<Arc<AtomicBool>>) -> Self
    where
        I: IntoIterator<Item = Result<T, ShellError>>,
        I::IntoIter: Send + 'static,
        T: AsRef<[u8]> + Default + Send + 'static,
    {
        let iter = iter.into_iter();
        let cursor = Some(Cursor::new(T::default()));
        Self::read(ReadResultIterator { iter, cursor }, span, interrupt)
    }

    /// Set the known size, in number of bytes, of the [`ByteStream`].
    pub fn with_known_size(mut self, size: Option<u64>) -> Self {
        self.known_size = size;
        self
    }

    /// Get a reference to the inner [`ByteStreamSource`] of the [`ByteStream`].
    pub fn source(&self) -> &ByteStreamSource {
        &self.stream
    }

    /// Get a mutable reference to the inner [`ByteStreamSource`] of the [`ByteStream`].
    pub fn source_mut(&mut self) -> &mut ByteStreamSource {
        &mut self.stream
    }

    /// Returns the [`Span`] associated with the [`ByteStream`].
    pub fn span(&self) -> Span {
        self.span
    }

    /// Returns the known size, in number of bytes, of the [`ByteStream`].
    pub fn known_size(&self) -> Option<u64> {
        self.known_size
    }

    /// Convert the [`ByteStream`] into its [`Reader`] which allows one to [`Read`] the raw bytes of the stream.
    ///
    /// [`Reader`] is buffered and also implements [`BufRead`].
    ///
    /// If the source of the [`ByteStream`] is [`ByteStreamSource::Child`] and the child has no stdout,
    /// then the stream is considered empty and `None` will be returned.
    pub fn reader(self) -> Option<Reader> {
        let reader = self.stream.reader()?;
        Some(Reader {
            reader: BufReader::new(reader),
            span: self.span,
            ctrlc: self.ctrlc,
        })
    }

    /// Convert the [`ByteStream`] into a [`Lines`] iterator where each element is a `Result<String, ShellError>`.
    ///
    /// There is no limit on how large each line will be. Ending new lines (`\n` or `\r\n`) are
    /// stripped from each line. If a line fails to be decoded as utf-8, then it will become a [`ShellError`].
    ///
    /// If the source of the [`ByteStream`] is [`ByteStreamSource::Child`] and the child has no stdout,
    /// then the stream is considered empty and `None` will be returned.
    pub fn lines(self) -> Option<Lines> {
        let reader = self.stream.reader()?;
        Some(Lines {
            reader: BufReader::new(reader),
            span: self.span,
            ctrlc: self.ctrlc,
        })
    }

    /// Convert the [`ByteStream`] into a [`Chunks`] iterator where each element is a `Result<Value, ShellError>`.
    ///
    /// Each call to [`next`](Iterator::next) reads the currently available data from the byte stream source,
    /// up to a maximum size. If the chunk of bytes, or an expected portion of it, succeeds utf-8 decoding,
    /// then it is returned as a [`Value::String`]. Otherwise, it is turned into a [`Value::Binary`].
    /// Any and all newlines are kept intact in each chunk.
    ///
    /// Where possible, prefer [`reader`](ByteStream::reader) or [`lines`](ByteStream::lines) over this method.
    /// Those methods are more likely to be used in a semantically correct way
    /// (and [`reader`](ByteStream::reader) is more efficient too).
    ///
    /// If the source of the [`ByteStream`] is [`ByteStreamSource::Child`] and the child has no stdout,
    /// then the stream is considered empty and `None` will be returned.
    pub fn chunks(self) -> Option<Chunks> {
        let reader = self.stream.reader()?;
        Some(Chunks {
            reader: BufReader::new(reader),
            span: self.span,
            ctrlc: self.ctrlc,
            leftover: Vec::new(),
        })
    }

    /// Convert the [`ByteStream`] into its inner [`ByteStreamSource`].
    pub fn into_source(self) -> ByteStreamSource {
        self.stream
    }

    /// Attempt to convert the [`ByteStream`] into a [`Stdio`].
    ///
    /// This will succeed if the [`ByteStreamSource`] of the [`ByteStream`] is either:
    /// - [`File`](ByteStreamSource::File)
    /// - [`Child`](ByteStreamSource::Child) and the child has a stdout that is `Some(ChildPipe::Pipe(..))`.
    ///
    /// All other cases return an `Err` with the original [`ByteStream`] in it.
    pub fn into_stdio(mut self) -> Result<Stdio, Self> {
        match self.stream {
            ByteStreamSource::Read(..) => Err(self),
            ByteStreamSource::File(file) => Ok(file.into()),
            ByteStreamSource::Child(child) => {
                if let ChildProcess {
                    stdout: Some(ChildPipe::Pipe(stdout)),
                    stderr,
                    ..
                } = *child
                {
                    debug_assert!(stderr.is_none(), "stderr should not exist");
                    Ok(stdout.into())
                } else {
                    self.stream = ByteStreamSource::Child(child);
                    Err(self)
                }
            }
        }
    }

    /// Attempt to convert the [`ByteStream`] into a [`ChildProcess`].
    ///
    /// This will only succeed if the [`ByteStreamSource`] of the [`ByteStream`] is [`Child`](ByteStreamSource::Child).
    /// All other cases return an `Err` with the original [`ByteStream`] in it.
    pub fn into_child(self) -> Result<ChildProcess, Self> {
        if let ByteStreamSource::Child(child) = self.stream {
            Ok(*child)
        } else {
            Err(self)
        }
    }

    /// Collect all the bytes of the [`ByteStream`] into a [`Vec<u8>`].
    ///
    /// Any trailing new lines are kept in the returned [`Vec`].
    pub fn into_bytes(self) -> Result<Vec<u8>, ShellError> {
        // todo!() ctrlc
        match self.stream {
            ByteStreamSource::Read(mut read) => {
                let mut buf = Vec::new();
                read.read_to_end(&mut buf).err_span(self.span)?;
                Ok(buf)
            }
            ByteStreamSource::File(mut file) => {
                let mut buf = Vec::new();
                file.read_to_end(&mut buf).err_span(self.span)?;
                Ok(buf)
            }
            ByteStreamSource::Child(child) => child.into_bytes(),
        }
    }

    /// Collect all the bytes of the [`ByteStream`] into a [`String`].
    ///
    /// The trailing new line (`\n` or `\r\n`), if any, is removed from the [`String`] prior to being returned.
    ///
    /// If utf-8 decoding fails, an error is returned.
    pub fn into_string(self) -> Result<String, ShellError> {
        let span = self.span;
        let bytes = self.into_bytes()?;
        let mut string = String::from_utf8(bytes).map_err(|_| ShellError::NonUtf8 { span })?;
        trim_end_newline(&mut string);
        Ok(string)
    }

    /// Collect all the bytes of the [`ByteStream`] into a [`Value`].
    ///
    /// If the collected bytes are successfully decoded as utf-8, then a [`Value::String`] is returned.
    /// The trailing new line (`\n` or `\r\n`), if any, is removed from the [`String`] prior to being returned.
    /// Otherwise, a [`Value::Binary`] is returned with any trailing new lines preserved.
    pub fn into_value(self) -> Result<Value, ShellError> {
        let span = self.span;
        let bytes = self.into_bytes()?;
        let value = match String::from_utf8(bytes) {
            Ok(mut str) => {
                trim_end_newline(&mut str);
                Value::string(str, span)
            }
            Err(err) => Value::binary(err.into_bytes(), span),
        };
        Ok(value)
    }

    /// Consume and drop all bytes of the [`ByteStream`].
    ///
    /// If the source of the [`ByteStream`] is [`ByteStreamSource::Child`],
    /// then the [`ExitStatus`] of the [`ChildProcess`] is returned.
    pub fn drain(self) -> Result<Option<ExitStatus>, ShellError> {
        match self.stream {
            ByteStreamSource::Read(mut read) => {
                copy_with_interrupt(&mut read, &mut io::sink(), self.span, self.ctrlc.as_deref())?;
                Ok(None)
            }
            ByteStreamSource::File(_) => Ok(None),
            ByteStreamSource::Child(child) => Ok(Some(child.wait()?)),
        }
    }

    /// Print all bytes of the [`ByteStream`] to stdout or stderr.
    ///
    /// If the source of the [`ByteStream`] is [`ByteStreamSource::Child`],
    /// then the [`ExitStatus`] of the [`ChildProcess`] is returned.
    pub fn print(self, to_stderr: bool) -> Result<Option<ExitStatus>, ShellError> {
        if to_stderr {
            self.write_to(&mut io::stderr())
        } else {
            self.write_to(&mut io::stdout())
        }
    }

    /// Write all bytes of the [`ByteStream`] to `dest`.
    ///
    /// If the source of the [`ByteStream`] is [`ByteStreamSource::Child`],
    /// then the [`ExitStatus`] of the [`ChildProcess`] is returned.
    pub fn write_to(self, dest: &mut impl Write) -> Result<Option<ExitStatus>, ShellError> {
        let span = self.span;
        let ctrlc = self.ctrlc.as_deref();
        match self.stream {
            ByteStreamSource::Read(mut read) => {
                copy_with_interrupt(&mut read, dest, span, ctrlc)?;
                Ok(None)
            }
            ByteStreamSource::File(mut file) => {
                copy_with_interrupt(&mut file, dest, span, ctrlc)?;
                Ok(None)
            }
            ByteStreamSource::Child(mut child) => {
                // All `OutDest`s except `OutDest::Capture` will cause `stderr` to be `None`.
                // Only `save`, `tee`, and `complete` set the stderr `OutDest` to `OutDest::Capture`,
                // and those commands have proper simultaneous handling of stdout and stderr.
                debug_assert!(child.stderr.is_none(), "stderr should not exist");

                if let Some(stdout) = child.stdout.take() {
                    match stdout {
                        ChildPipe::Pipe(mut pipe) => {
                            copy_with_interrupt(&mut pipe, dest, span, ctrlc)?;
                        }
                        ChildPipe::Tee(mut tee) => {
                            copy_with_interrupt(&mut tee, dest, span, ctrlc)?;
                        }
                    }
                }
                Ok(Some(child.wait()?))
            }
        }
    }

    pub(crate) fn write_to_out_dests(
        self,
        stdout: &OutDest,
        stderr: &OutDest,
    ) -> Result<Option<ExitStatus>, ShellError> {
        let span = self.span;
        let ctrlc = self.ctrlc.as_deref();

        match self.stream {
            ByteStreamSource::Read(read) => {
                write_to_out_dest(read, stdout, true, span, ctrlc)?;
                Ok(None)
            }
            ByteStreamSource::File(mut file) => {
                match stdout {
                    OutDest::Pipe | OutDest::Capture | OutDest::Null => {}
                    OutDest::Inherit => {
                        copy_with_interrupt(&mut file, &mut io::stdout(), span, ctrlc)?;
                    }
                    OutDest::File(f) => {
                        copy_with_interrupt(&mut file, &mut f.as_ref(), span, ctrlc)?;
                    }
                }
                Ok(None)
            }
            ByteStreamSource::Child(mut child) => {
                match (child.stdout.take(), child.stderr.take()) {
                    (Some(out), Some(err)) => {
                        // To avoid deadlocks, we must spawn a separate thread to wait on stderr.
                        thread::scope(|s| {
                            let err_thread = thread::Builder::new()
                                .spawn_scoped(s, || match err {
                                    ChildPipe::Pipe(pipe) => {
                                        write_to_out_dest(pipe, stderr, false, span, ctrlc)
                                    }
                                    ChildPipe::Tee(tee) => {
                                        write_to_out_dest(tee, stderr, false, span, ctrlc)
                                    }
                                })
                                .err_span(span);

                            match out {
                                ChildPipe::Pipe(pipe) => {
                                    write_to_out_dest(pipe, stdout, true, span, ctrlc)
                                }
                                ChildPipe::Tee(tee) => {
                                    write_to_out_dest(tee, stdout, true, span, ctrlc)
                                }
                            }?;

                            if let Ok(result) = err_thread?.join() {
                                result?;
                            } else {
                                // thread panicked, which should not happen
                                debug_assert!(false)
                            }

                            Ok::<_, ShellError>(())
                        })?;
                    }
                    (Some(out), None) => {
                        // single output stream, we can consume directly
                        write_to_out_dest(out, stdout, true, span, ctrlc)?;
                    }
                    (None, Some(err)) => {
                        // single output stream, we can consume directly
                        write_to_out_dest(err, stderr, false, span, ctrlc)?;
                    }
                    (None, None) => {}
                }
                Ok(Some(child.wait()?))
            }
        }
    }
}

impl Debug for ByteStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ByteStream").finish()
    }
}

impl From<ByteStream> for PipelineData {
    fn from(stream: ByteStream) -> Self {
        Self::ByteStream(stream, None)
    }
}

struct ReadIterator<I>
where
    I: Iterator,
    I::Item: AsRef<[u8]>,
{
    iter: I,
    cursor: Option<Cursor<I::Item>>,
}

impl<I> Read for ReadIterator<I>
where
    I: Iterator,
    I::Item: AsRef<[u8]>,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        while let Some(cursor) = self.cursor.as_mut() {
            let read = cursor.read(buf)?;
            if read == 0 {
                self.cursor = self.iter.next().map(Cursor::new);
            } else {
                return Ok(read);
            }
        }
        Ok(0)
    }
}

struct ReadResultIterator<I, T>
where
    I: Iterator<Item = Result<T, ShellError>>,
    T: AsRef<[u8]>,
{
    iter: I,
    cursor: Option<Cursor<T>>,
}

impl<I, T> Read for ReadResultIterator<I, T>
where
    I: Iterator<Item = Result<T, ShellError>>,
    T: AsRef<[u8]>,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        while let Some(cursor) = self.cursor.as_mut() {
            let read = cursor.read(buf)?;
            if read == 0 {
                self.cursor = self.iter.next().transpose()?.map(Cursor::new);
            } else {
                return Ok(read);
            }
        }
        Ok(0)
    }
}

pub struct Reader {
    reader: BufReader<SourceReader>,
    span: Span,
    ctrlc: Option<Arc<AtomicBool>>,
}

impl Reader {
    pub fn span(&self) -> Span {
        self.span
    }
}

impl Read for Reader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if nu_utils::ctrl_c::was_pressed(&self.ctrlc) {
            Err(ShellError::InterruptedByUser {
                span: Some(self.span),
            }
            .into())
        } else {
            self.reader.read(buf)
        }
    }
}

impl BufRead for Reader {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.reader.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.reader.consume(amt)
    }
}

pub struct Lines {
    reader: BufReader<SourceReader>,
    span: Span,
    ctrlc: Option<Arc<AtomicBool>>,
}

impl Lines {
    pub fn span(&self) -> Span {
        self.span
    }
}

impl Iterator for Lines {
    type Item = Result<String, ShellError>;

    fn next(&mut self) -> Option<Self::Item> {
        if nu_utils::ctrl_c::was_pressed(&self.ctrlc) {
            None
        } else {
            let mut buf = Vec::new();
            match self.reader.read_until(b'\n', &mut buf) {
                Ok(0) => None,
                Ok(_) => {
                    let Ok(mut string) = String::from_utf8(buf) else {
                        return Some(Err(ShellError::NonUtf8 { span: self.span }));
                    };
                    trim_end_newline(&mut string);
                    Some(Ok(string))
                }
                Err(e) => Some(Err(e.into_spanned(self.span).into())),
            }
        }
    }
}

pub struct Chunks {
    reader: BufReader<SourceReader>,
    span: Span,
    ctrlc: Option<Arc<AtomicBool>>,
    leftover: Vec<u8>,
}

impl Chunks {
    pub fn span(&self) -> Span {
        self.span
    }
}

impl Iterator for Chunks {
    type Item = Result<Value, ShellError>;

    fn next(&mut self) -> Option<Self::Item> {
        if nu_utils::ctrl_c::was_pressed(&self.ctrlc) {
            None
        } else {
            match self.reader.fill_buf() {
                Ok(buf) => {
                    self.leftover.extend_from_slice(buf);
                    let len = buf.len();
                    self.reader.consume(len);
                }
                Err(err) => return Some(Err(err.into_spanned(self.span).into())),
            };

            if self.leftover.is_empty() {
                return None;
            }

            match String::from_utf8(std::mem::take(&mut self.leftover)) {
                Ok(str) => Some(Ok(Value::string(str, self.span))),
                Err(err) => {
                    if err.utf8_error().error_len().is_some() {
                        Some(Ok(Value::binary(err.into_bytes(), self.span)))
                    } else {
                        let i = err.utf8_error().valid_up_to();
                        let mut bytes = err.into_bytes();
                        self.leftover = bytes.split_off(i);
                        let str = String::from_utf8(bytes).expect("valid utf8");
                        Some(Ok(Value::string(str, self.span)))
                    }
                }
            }
        }
    }
}

fn trim_end_newline(string: &mut String) {
    if string.ends_with('\n') {
        string.pop();
        if string.ends_with('\r') {
            string.pop();
        }
    }
}

fn write_to_out_dest(
    mut read: impl Read,
    stream: &OutDest,
    stdout: bool,
    span: Span,
    ctrlc: Option<&AtomicBool>,
) -> Result<(), ShellError> {
    match stream {
        OutDest::Pipe | OutDest::Capture => return Ok(()),
        OutDest::Null => copy_with_interrupt(&mut read, &mut io::sink(), span, ctrlc),
        OutDest::Inherit if stdout => {
            copy_with_interrupt(&mut read, &mut io::stdout(), span, ctrlc)
        }
        OutDest::Inherit => copy_with_interrupt(&mut read, &mut io::stderr(), span, ctrlc),
        OutDest::File(file) => copy_with_interrupt(&mut read, &mut file.as_ref(), span, ctrlc),
    }?;
    Ok(())
}

#[cfg(unix)]
pub(crate) fn convert_file<T: From<OwnedFd>>(file: impl Into<OwnedFd>) -> T {
    file.into().into()
}

#[cfg(windows)]
pub(crate) fn convert_file<T: From<OwnedHandle>>(file: impl Into<OwnedHandle>) -> T {
    file.into().into()
}

const DEFAULT_BUF_SIZE: usize = 8192;

pub fn copy_with_interrupt<R: ?Sized, W: ?Sized>(
    reader: &mut R,
    writer: &mut W,
    span: Span,
    interrupt: Option<&AtomicBool>,
) -> Result<u64, ShellError>
where
    R: Read,
    W: Write,
{
    if let Some(interrupt) = interrupt {
        // #[cfg(any(target_os = "linux", target_os = "android"))]
        // {
        //     return crate::sys::kernel_copy::copy_spec(reader, writer);
        // }
        match generic_copy(reader, writer, span, interrupt) {
            Ok(len) => {
                writer.flush().err_span(span)?;
                Ok(len)
            }
            Err(err) => {
                let _ = writer.flush();
                Err(err)
            }
        }
    } else {
        match io::copy(reader, writer) {
            Ok(n) => {
                writer.flush().err_span(span)?;
                Ok(n)
            }
            Err(err) => {
                let _ = writer.flush();
                Err(err.into_spanned(span).into())
            }
        }
    }
}

// Copied from [`std::io::copy`]
fn generic_copy<R: ?Sized, W: ?Sized>(
    reader: &mut R,
    writer: &mut W,
    span: Span,
    interrupt: &AtomicBool,
) -> Result<u64, ShellError>
where
    R: Read,
    W: Write,
{
    let buf = &mut [0; DEFAULT_BUF_SIZE];
    let mut len = 0;
    loop {
        if interrupt.load(Ordering::Relaxed) {
            return Err(ShellError::InterruptedByUser { span: Some(span) });
        }
        let n = match reader.read(buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => return Err(e.into_spanned(span).into()),
        };
        len += n;
        writer.write_all(&buf[..n]).err_span(span)?;
    }
    Ok(len as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_chunks<T>(data: Vec<T>) -> Chunks
    where
        T: AsRef<[u8]> + Default + Send + 'static,
    {
        let reader = ReadIterator {
            iter: data.into_iter(),
            cursor: Some(Cursor::new(T::default())),
        };
        Chunks {
            reader: BufReader::new(SourceReader::Read(Box::new(reader))),
            span: Span::test_data(),
            ctrlc: None,
            leftover: Vec::new(),
        }
    }

    #[test]
    fn chunks_read_string() {
        let data = vec!["Nushell", "が好きです"];
        let chunks = test_chunks(data.clone());
        let actual = chunks.collect::<Result<Vec<_>, _>>().unwrap();
        let expected = data.into_iter().map(Value::test_string).collect::<Vec<_>>();
        assert_eq!(expected, actual);
    }

    #[test]
    fn chunks_read_string_split_utf8() {
        let expected = "Nushell最高!";
        let chunks = test_chunks(vec![&b"Nushell\xe6"[..], b"\x9c\x80\xe9", b"\xab\x98!"]);

        let actual = chunks
            .into_iter()
            .map(|value| value.and_then(Value::into_string))
            .collect::<Result<String, _>>()
            .unwrap();

        assert_eq!(expected, actual);
    }

    #[test]
    fn chunks_returns_string_or_binary() {
        let chunks = test_chunks(vec![b"Nushell".as_slice(), b"\x9c\x80\xe9abcd", b"efgh"]);
        let actual = chunks.collect::<Result<Vec<_>, _>>().unwrap();
        let expected = vec![
            Value::test_string("Nushell"),
            Value::test_binary(b"\x9c\x80\xe9abcd"),
            Value::test_string("efgh"),
        ];
        assert_eq!(actual, expected)
    }
}
