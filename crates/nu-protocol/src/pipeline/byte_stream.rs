//! Module managing the streaming of raw bytes between pipeline elements
//!
//! This module also handles conversions the [`ShellError`] <-> [`io::Error`](std::io::Error),
//! so remember the usage of [`ShellErrorBridge`] where applicable.
#[cfg(feature = "os")]
use crate::process::{ChildPipe, ChildProcess};
use crate::{
    IntRange, PipelineData, ShellError, Signals, Span, Type, Value,
    shell_error::{bridge::ShellErrorBridge, io::IoError},
};
use serde::{Deserialize, Serialize};
use std::ops::Bound;
#[cfg(unix)]
use std::os::fd::OwnedFd;
#[cfg(windows)]
use std::os::windows::io::OwnedHandle;
use std::{
    fmt::Debug,
    fs::File,
    io::{self, BufRead, BufReader, Cursor, ErrorKind, Read, Write},
    process::Stdio,
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
    #[cfg(feature = "os")]
    Child(Box<ChildProcess>),
}

impl ByteStreamSource {
    fn reader(self) -> Option<SourceReader> {
        match self {
            ByteStreamSource::Read(read) => Some(SourceReader::Read(read)),
            ByteStreamSource::File(file) => Some(SourceReader::File(file)),
            #[cfg(feature = "os")]
            ByteStreamSource::Child(mut child) => child.stdout.take().map(|stdout| match stdout {
                ChildPipe::Pipe(pipe) => SourceReader::File(convert_file(pipe)),
                ChildPipe::Tee(tee) => SourceReader::Read(tee),
            }),
        }
    }

    /// Source is a `Child` or `File`, rather than `Read`. Currently affects trimming
    #[cfg(feature = "os")]
    pub fn is_external(&self) -> bool {
        matches!(self, ByteStreamSource::Child(..))
    }

    #[cfg(not(feature = "os"))]
    pub fn is_external(&self) -> bool {
        // without os support we never have externals
        false
    }
}

impl Debug for ByteStreamSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ByteStreamSource::Read(_) => f.debug_tuple("Read").field(&"..").finish(),
            ByteStreamSource::File(file) => f.debug_tuple("File").field(file).finish(),
            #[cfg(feature = "os")]
            ByteStreamSource::Child(child) => f.debug_tuple("Child").field(child).finish(),
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

impl Debug for SourceReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceReader::Read(_) => f.debug_tuple("Read").field(&"..").finish(),
            SourceReader::File(file) => f.debug_tuple("File").field(file).finish(),
        }
    }
}

/// Optional type color for [`ByteStream`], which determines type compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ByteStreamType {
    /// Compatible with [`Type::Binary`], and should only be converted to binary, even when the
    /// desired type is unknown.
    Binary,
    /// Compatible with [`Type::String`], and should only be converted to string, even when the
    /// desired type is unknown.
    ///
    /// This does not guarantee valid UTF-8 data, but it is conventionally so. Converting to
    /// `String` still requires validation of the data.
    String,
    /// Unknown whether the stream should contain binary or string data. This usually is the result
    /// of an external stream, e.g. an external command or file.
    #[default]
    Unknown,
}

impl ByteStreamType {
    /// Returns the string that describes the byte stream type - i.e., the same as what `describe`
    /// produces. This can be used in type mismatch error messages.
    pub fn describe(self) -> &'static str {
        match self {
            ByteStreamType::Binary => "binary (stream)",
            ByteStreamType::String => "string (stream)",
            ByteStreamType::Unknown => "byte stream",
        }
    }

    /// Returns true if the type is `Binary` or `Unknown`
    pub fn is_binary_coercible(self) -> bool {
        matches!(self, ByteStreamType::Binary | ByteStreamType::Unknown)
    }

    /// Returns true if the type is `String` or `Unknown`
    pub fn is_string_coercible(self) -> bool {
        matches!(self, ByteStreamType::String | ByteStreamType::Unknown)
    }
}

impl From<ByteStreamType> for Type {
    fn from(value: ByteStreamType) -> Self {
        match value {
            ByteStreamType::Binary => Type::Binary,
            ByteStreamType::String => Type::String,
            ByteStreamType::Unknown => Type::Any,
        }
    }
}

/// A potentially infinite, interruptible stream of bytes.
///
/// To create a [`ByteStream`], you can use any of the following methods:
/// - [`read`](ByteStream::read): takes any type that implements [`Read`].
/// - [`file`](ByteStream::file): takes a [`File`].
/// - [`from_iter`](ByteStream::from_iter): takes an [`Iterator`] whose items implement `AsRef<[u8]>`.
/// - [`from_result_iter`](ByteStream::from_result_iter): same as [`from_iter`](ByteStream::from_iter),
///   but each item is a `Result<T, ShellError>`.
/// - [`from_fn`](ByteStream::from_fn): uses a generator function to fill a buffer whenever it is
///   empty. This has high performance because it doesn't need to allocate for each chunk of data,
///   and can just reuse the same buffer.
///
/// Byte streams have a [type](.type_()) which is used to preserve type compatibility when they
/// are the result of an internal command. It is important that this be set to the correct value.
/// [`Unknown`](ByteStreamType::Unknown) is used only for external sources where the type can not
/// be inherently determined, and having it automatically act as a string or binary depending on
/// whether it parses as UTF-8 or not is desirable.
///
/// The data of a [`ByteStream`] can be accessed using one of the following methods:
/// - [`reader`](ByteStream::reader): returns a [`Read`]-able type to get the raw bytes in the stream.
/// - [`lines`](ByteStream::lines): splits the bytes on lines and returns an [`Iterator`]
///   where each item is a `Result<String, ShellError>`.
/// - [`chunks`](ByteStream::chunks): returns an [`Iterator`] of [`Value`]s where each value is
///   either a string or binary.
///   Try not to use this method if possible. Rather, please use [`reader`](ByteStream::reader)
///   (or [`lines`](ByteStream::lines) if it matches the situation).
///
/// Additionally, there are few methods to collect a [`ByteStream`] into memory:
/// - [`into_bytes`](ByteStream::into_bytes): collects all bytes into a [`Vec<u8>`].
/// - [`into_string`](ByteStream::into_string): collects all bytes into a [`String`], erroring if utf-8 decoding failed.
/// - [`into_value`](ByteStream::into_value): collects all bytes into a value typed appropriately
///   for the [type](.type_()) of this stream. If the type is [`Unknown`](ByteStreamType::Unknown),
///   it will produce a string value if the data is valid UTF-8, or a binary value otherwise.
///
/// There are also a few other methods to consume all the data of a [`ByteStream`]:
/// - [`drain`](ByteStream::drain): consumes all bytes and outputs nothing.
/// - [`write_to`](ByteStream::write_to): writes all bytes to the given [`Write`] destination.
/// - [`print`](ByteStream::print): a convenience wrapper around [`write_to`](ByteStream::write_to).
///   It prints all bytes to stdout or stderr.
///
/// Internally, [`ByteStream`]s currently come in three flavors according to [`ByteStreamSource`].
/// See its documentation for more information.
#[derive(Debug)]
pub struct ByteStream {
    stream: ByteStreamSource,
    span: Span,
    signals: Signals,
    type_: ByteStreamType,
    known_size: Option<u64>,
    caller_spans: Vec<Span>,
}

impl ByteStream {
    /// Create a new [`ByteStream`] from a [`ByteStreamSource`].
    pub fn new(
        stream: ByteStreamSource,
        span: Span,
        signals: Signals,
        type_: ByteStreamType,
    ) -> Self {
        Self {
            stream,
            span,
            signals,
            type_,
            known_size: None,
            caller_spans: vec![],
        }
    }

    /// Push a caller [`Span`] to the bytestream, it's useful to construct a backtrace.
    pub fn push_caller_span(&mut self, span: Span) {
        if span != self.span {
            self.caller_spans.push(span)
        }
    }

    /// Get all caller [`Span`], it's useful to construct a backtrace.
    pub fn get_caller_spans(&self) -> &Vec<Span> {
        &self.caller_spans
    }

    /// Create a [`ByteStream`] from an arbitrary reader. The type must be provided.
    pub fn read(
        reader: impl Read + Send + 'static,
        span: Span,
        signals: Signals,
        type_: ByteStreamType,
    ) -> Self {
        Self::new(
            ByteStreamSource::Read(Box::new(reader)),
            span,
            signals,
            type_,
        )
    }

    pub fn skip(self, span: Span, n: u64) -> Result<Self, ShellError> {
        let known_size = self.known_size.map(|len| len.saturating_sub(n));
        if let Some(mut reader) = self.reader() {
            // Copy the number of skipped bytes into the sink before proceeding
            io::copy(&mut (&mut reader).take(n), &mut io::sink())
                .map_err(|err| IoError::new(err, span, None))?;
            Ok(
                ByteStream::read(reader, span, Signals::empty(), ByteStreamType::Binary)
                    .with_known_size(known_size),
            )
        } else {
            Err(ShellError::TypeMismatch {
                err_message: "expected readable stream".into(),
                span,
            })
        }
    }

    pub fn take(self, span: Span, n: u64) -> Result<Self, ShellError> {
        let known_size = self.known_size.map(|s| s.min(n));
        if let Some(reader) = self.reader() {
            Ok(ByteStream::read(
                reader.take(n),
                span,
                Signals::empty(),
                ByteStreamType::Binary,
            )
            .with_known_size(known_size))
        } else {
            Err(ShellError::TypeMismatch {
                err_message: "expected readable stream".into(),
                span,
            })
        }
    }

    pub fn slice(
        self,
        val_span: Span,
        call_span: Span,
        range: IntRange,
    ) -> Result<Self, ShellError> {
        if let Some(len) = self.known_size {
            let start = range.absolute_start(len);
            let stream = self.skip(val_span, start);

            match range.absolute_end(len) {
                Bound::Unbounded => stream,
                Bound::Included(end) | Bound::Excluded(end) if end < start => {
                    stream.and_then(|s| s.take(val_span, 0))
                }
                Bound::Included(end) => {
                    let distance = end - start + 1;
                    stream.and_then(|s| s.take(val_span, distance.min(len)))
                }
                Bound::Excluded(end) => {
                    let distance = end - start;
                    stream.and_then(|s| s.take(val_span, distance.min(len)))
                }
            }
        } else if range.is_relative() {
            Err(ShellError::RelativeRangeOnInfiniteStream { span: call_span })
        } else {
            let start = range.start() as u64;
            let stream = self.skip(val_span, start);

            match range.distance() {
                Bound::Unbounded => stream,
                Bound::Included(distance) => stream.and_then(|s| s.take(val_span, distance + 1)),
                Bound::Excluded(distance) => stream.and_then(|s| s.take(val_span, distance)),
            }
        }
    }

    /// Create a [`ByteStream`] from a string. The type of the stream is always `String`.
    pub fn read_string(string: String, span: Span, signals: Signals) -> Self {
        let len = string.len();
        ByteStream::read(
            Cursor::new(string.into_bytes()),
            span,
            signals,
            ByteStreamType::String,
        )
        .with_known_size(Some(len as u64))
    }

    /// Create a [`ByteStream`] from a byte vector. The type of the stream is always `Binary`.
    pub fn read_binary(bytes: Vec<u8>, span: Span, signals: Signals) -> Self {
        let len = bytes.len();
        ByteStream::read(Cursor::new(bytes), span, signals, ByteStreamType::Binary)
            .with_known_size(Some(len as u64))
    }

    /// Create a [`ByteStream`] from a file.
    ///
    /// The type is implicitly `Unknown`, as it's not typically known whether files will
    /// return text or binary.
    pub fn file(file: File, span: Span, signals: Signals) -> Self {
        Self::new(
            ByteStreamSource::File(file),
            span,
            signals,
            ByteStreamType::Unknown,
        )
    }

    /// Create a [`ByteStream`] from a child process's stdout and stderr.
    ///
    /// The type is implicitly `Unknown`, as it's not typically known whether child processes will
    /// return text or binary.
    #[cfg(feature = "os")]
    pub fn child(child: ChildProcess, span: Span) -> Self {
        Self::new(
            ByteStreamSource::Child(Box::new(child)),
            span,
            Signals::empty(),
            ByteStreamType::Unknown,
        )
    }

    /// Create a [`ByteStream`] that reads from stdin.
    ///
    /// The type is implicitly `Unknown`, as it's not typically known whether stdin is text or
    /// binary.
    #[cfg(feature = "os")]
    pub fn stdin(span: Span) -> Result<Self, ShellError> {
        let stdin = os_pipe::dup_stdin().map_err(|err| IoError::new(err, span, None))?;
        let source = ByteStreamSource::File(convert_file(stdin));
        Ok(Self::new(
            source,
            span,
            Signals::empty(),
            ByteStreamType::Unknown,
        ))
    }

    #[cfg(not(feature = "os"))]
    pub fn stdin(span: Span) -> Result<Self, ShellError> {
        Err(ShellError::DisabledOsSupport {
            msg: "Stdin is not supported".to_string(),
            span: Some(span),
        })
    }

    /// Create a [`ByteStream`] from a generator function that writes data to the given buffer
    /// when called, and returns `Ok(false)` on end of stream.
    pub fn from_fn(
        span: Span,
        signals: Signals,
        type_: ByteStreamType,
        generator: impl FnMut(&mut Vec<u8>) -> Result<bool, ShellError> + Send + 'static,
    ) -> Self {
        Self::read(
            ReadGenerator {
                buffer: Cursor::new(Vec::new()),
                generator,
            },
            span,
            signals,
            type_,
        )
    }

    pub fn with_type(mut self, type_: ByteStreamType) -> Self {
        self.type_ = type_;
        self
    }

    /// Create a new [`ByteStream`] from an [`Iterator`] of bytes slices.
    ///
    /// The returned [`ByteStream`] will have a [`ByteStreamSource`] of `Read`.
    pub fn from_iter<I>(iter: I, span: Span, signals: Signals, type_: ByteStreamType) -> Self
    where
        I: IntoIterator,
        I::IntoIter: Send + 'static,
        I::Item: AsRef<[u8]> + Default + Send + 'static,
    {
        let iter = iter.into_iter();
        let cursor = Some(Cursor::new(I::Item::default()));
        Self::read(ReadIterator { iter, cursor }, span, signals, type_)
    }

    /// Create a new [`ByteStream`] from an [`Iterator`] of [`Result`] bytes slices.
    ///
    /// The returned [`ByteStream`] will have a [`ByteStreamSource`] of `Read`.
    pub fn from_result_iter<I, T>(
        iter: I,
        span: Span,
        signals: Signals,
        type_: ByteStreamType,
    ) -> Self
    where
        I: IntoIterator<Item = Result<T, ShellError>>,
        I::IntoIter: Send + 'static,
        T: AsRef<[u8]> + Default + Send + 'static,
    {
        let iter = iter.into_iter();
        let cursor = Some(Cursor::new(T::default()));
        Self::read(ReadResultIterator { iter, cursor }, span, signals, type_)
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

    /// Changes the [`Span`] associated with the [`ByteStream`].
    pub fn with_span(mut self, span: Span) -> Self {
        self.span = span;
        self
    }

    /// Returns the [`ByteStreamType`] associated with the [`ByteStream`].
    pub fn type_(&self) -> ByteStreamType {
        self.type_
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
            signals: self.signals,
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
            signals: self.signals,
        })
    }

    /// Convert the [`ByteStream`] into a [`SplitRead`] iterator where each element is a `Result<String, ShellError>`.
    ///
    /// Each call to [`next`](Iterator::next) reads the currently available data from the byte
    /// stream source, until `delimiter` or the end of the stream is encountered.
    ///
    /// If the source of the [`ByteStream`] is [`ByteStreamSource::Child`] and the child has no stdout,
    /// then the stream is considered empty and `None` will be returned.
    pub fn split(self, delimiter: Vec<u8>) -> Option<SplitRead> {
        let reader = self.stream.reader()?;
        Some(SplitRead::new(reader, delimiter, self.span, self.signals))
    }

    /// Convert the [`ByteStream`] into a [`Chunks`] iterator where each element is a `Result<Value, ShellError>`.
    ///
    /// Each call to [`next`](Iterator::next) reads the currently available data from the byte stream source,
    /// up to a maximum size. The values are typed according to the [type](.type_()) of the
    /// stream, and if that type is [`Unknown`](ByteStreamType::Unknown), string values will be
    /// produced as long as the stream continues to parse as valid UTF-8, but binary values will
    /// be produced instead of the stream fails to parse as UTF-8 instead at any point.
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
        Some(Chunks::new(reader, self.span, self.signals, self.type_))
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
            #[cfg(feature = "os")]
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
    #[cfg(feature = "os")]
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
        let from_io_error = IoError::factory(self.span, None);
        match self.stream {
            ByteStreamSource::Read(mut read) => {
                let mut buf = Vec::new();
                read.read_to_end(&mut buf).map_err(|err| {
                    match ShellErrorBridge::try_from(err) {
                        Ok(ShellErrorBridge(err)) => err,
                        Err(err) => ShellError::Io(from_io_error(err)),
                    }
                })?;
                Ok(buf)
            }
            ByteStreamSource::File(mut file) => {
                let mut buf = Vec::new();
                file.read_to_end(&mut buf).map_err(&from_io_error)?;
                Ok(buf)
            }
            #[cfg(feature = "os")]
            ByteStreamSource::Child(child) => child.into_bytes(),
        }
    }

    /// Collect the stream into a `String` in-memory. This can only succeed if the data contained is
    /// valid UTF-8.
    ///
    /// The trailing new line (`\n` or `\r\n`), if any, is removed from the [`String`] prior to
    /// being returned, if this is a stream coming from an external process or file.
    ///
    /// If the [type](.type_()) is specified as `Binary`, this operation always fails, even if the
    /// data would have been valid UTF-8.
    pub fn into_string(self) -> Result<String, ShellError> {
        let span = self.span;
        if self.type_.is_string_coercible() {
            let trim = self.stream.is_external();
            let bytes = self.into_bytes()?;
            let mut string = String::from_utf8(bytes).map_err(|err| ShellError::NonUtf8Custom {
                span,
                msg: err.to_string(),
            })?;
            if trim {
                trim_end_newline(&mut string);
            }
            Ok(string)
        } else {
            Err(ShellError::TypeMismatch {
                err_message: "expected string, but got binary".into(),
                span,
            })
        }
    }

    /// Collect all the bytes of the [`ByteStream`] into a [`Value`].
    ///
    /// If this is a `String` stream, the stream is decoded to UTF-8. If the stream came from an
    /// external process or file, the trailing new line (`\n` or `\r\n`), if any, is removed from
    /// the [`String`] prior to being returned.
    ///
    /// If this is a `Binary` stream, a [`Value::Binary`] is returned with any trailing new lines
    /// preserved.
    ///
    /// If this is an `Unknown` stream, the behavior depends on whether the stream parses as valid
    /// UTF-8 or not. If it does, this is uses the `String` behavior; if not, it uses the `Binary`
    /// behavior.
    pub fn into_value(self) -> Result<Value, ShellError> {
        let span = self.span;
        let trim = self.stream.is_external();
        let value = match self.type_ {
            // If the type is specified, then the stream should always become that type:
            ByteStreamType::Binary => Value::binary(self.into_bytes()?, span),
            ByteStreamType::String => Value::string(self.into_string()?, span),
            // If the type is not specified, then it just depends on whether it parses or not:
            ByteStreamType::Unknown => match String::from_utf8(self.into_bytes()?) {
                Ok(mut str) => {
                    if trim {
                        trim_end_newline(&mut str);
                    }
                    Value::string(str, span)
                }
                Err(err) => Value::binary(err.into_bytes(), span),
            },
        };
        Ok(value)
    }

    /// Consume and drop all bytes of the [`ByteStream`].
    pub fn drain(self) -> Result<(), ShellError> {
        match self.stream {
            ByteStreamSource::Read(read) => {
                copy_with_signals(read, io::sink(), self.span, &self.signals)?;
                Ok(())
            }
            ByteStreamSource::File(_) => Ok(()),
            #[cfg(feature = "os")]
            ByteStreamSource::Child(child) => child.wait(),
        }
    }

    /// Print all bytes of the [`ByteStream`] to stdout or stderr.
    pub fn print(self, to_stderr: bool) -> Result<(), ShellError> {
        if to_stderr {
            self.write_to(&mut io::stderr())
        } else {
            self.write_to(&mut io::stdout())
        }
    }

    /// Write all bytes of the [`ByteStream`] to `dest`.
    pub fn write_to(self, dest: impl Write) -> Result<(), ShellError> {
        let span = self.span;
        let signals = &self.signals;
        match self.stream {
            ByteStreamSource::Read(read) => {
                copy_with_signals(read, dest, span, signals)?;
            }
            ByteStreamSource::File(file) => {
                copy_with_signals(file, dest, span, signals)?;
            }
            #[cfg(feature = "os")]
            ByteStreamSource::Child(mut child) => {
                // All `OutDest`s except `OutDest::PipeSeparate` will cause `stderr` to be `None`.
                // Only `save`, `tee`, and `complete` set the stderr `OutDest` to `OutDest::PipeSeparate`,
                // and those commands have proper simultaneous handling of stdout and stderr.
                debug_assert!(child.stderr.is_none(), "stderr should not exist");

                if let Some(stdout) = child.stdout.take() {
                    match stdout {
                        ChildPipe::Pipe(pipe) => {
                            copy_with_signals(pipe, dest, span, signals)?;
                        }
                        ChildPipe::Tee(tee) => {
                            copy_with_signals(tee, dest, span, signals)?;
                        }
                    }
                }
                child.wait()?;
            }
        }
        Ok(())
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
                self.cursor = self
                    .iter
                    .next()
                    .transpose()
                    .map_err(ShellErrorBridge)?
                    .map(Cursor::new);
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
    signals: Signals,
}

impl Reader {
    pub fn span(&self) -> Span {
        self.span
    }
}

impl Read for Reader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.signals.check(self.span).map_err(ShellErrorBridge)?;
        self.reader.read(buf)
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
    signals: Signals,
}

impl Lines {
    pub fn span(&self) -> Span {
        self.span
    }
}

impl Iterator for Lines {
    type Item = Result<String, ShellError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.signals.interrupted() {
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
                Err(err) => Some(Err(IoError::new(err, self.span, None).into())),
            }
        }
    }
}

mod split_read {
    use std::io::{BufRead, ErrorKind};

    use memchr::memmem::Finder;

    pub struct SplitRead<R> {
        reader: Option<R>,
        buf: Option<Vec<u8>>,
        finder: Finder<'static>,
    }

    impl<R: BufRead> SplitRead<R> {
        pub fn new(reader: R, delim: impl AsRef<[u8]>) -> Self {
            // empty delimiter results in an infinite stream of empty items
            debug_assert!(!delim.as_ref().is_empty(), "delimiter can't be empty");
            Self {
                reader: Some(reader),
                buf: Some(Vec::new()),
                finder: Finder::new(delim.as_ref()).into_owned(),
            }
        }
    }

    impl<R: BufRead> Iterator for SplitRead<R> {
        type Item = Result<Vec<u8>, std::io::Error>;

        fn next(&mut self) -> Option<Self::Item> {
            let buf = self.buf.as_mut()?;
            let mut search_start = 0usize;

            loop {
                if let Some(i) = self.finder.find(&buf[search_start..]) {
                    let needle_idx = search_start + i;
                    let right = buf.split_off(needle_idx + self.finder.needle().len());
                    buf.truncate(needle_idx);
                    let left = std::mem::replace(buf, right);
                    return Some(Ok(left));
                }

                if let Some(mut r) = self.reader.take() {
                    search_start = buf.len().saturating_sub(self.finder.needle().len() + 1);
                    let available = match r.fill_buf() {
                        Ok(n) => n,
                        Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                        Err(e) => return Some(Err(e)),
                    };

                    buf.extend_from_slice(available);
                    let used = available.len();
                    r.consume(used);
                    if used != 0 {
                        self.reader = Some(r);
                    }
                    continue;
                } else {
                    return self.buf.take().map(Ok);
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::io::{self, Cursor, Read};

        #[test]
        fn simple() {
            let s = "foo-bar-baz";
            let cursor = Cursor::new(String::from(s));
            let mut split =
                SplitRead::new(cursor, "-").map(|r| String::from_utf8(r.unwrap()).unwrap());

            assert_eq!(split.next().as_deref(), Some("foo"));
            assert_eq!(split.next().as_deref(), Some("bar"));
            assert_eq!(split.next().as_deref(), Some("baz"));
            assert_eq!(split.next(), None);
        }

        #[test]
        fn with_empty_fields() -> Result<(), io::Error> {
            let s = "\0\0foo\0\0bar\0\0\0\0baz\0\0";
            let cursor = Cursor::new(String::from(s));
            let mut split =
                SplitRead::new(cursor, "\0\0").map(|r| String::from_utf8(r.unwrap()).unwrap());

            assert_eq!(split.next().as_deref(), Some(""));
            assert_eq!(split.next().as_deref(), Some("foo"));
            assert_eq!(split.next().as_deref(), Some("bar"));
            assert_eq!(split.next().as_deref(), Some(""));
            assert_eq!(split.next().as_deref(), Some("baz"));
            assert_eq!(split.next().as_deref(), Some(""));
            assert_eq!(split.next().as_deref(), None);

            Ok(())
        }

        #[test]
        fn complex_delimiter() -> Result<(), io::Error> {
            let s = "<|>foo<|>bar<|><|>baz<|>";
            let cursor = Cursor::new(String::from(s));
            let mut split =
                SplitRead::new(cursor, "<|>").map(|r| String::from_utf8(r.unwrap()).unwrap());

            assert_eq!(split.next().as_deref(), Some(""));
            assert_eq!(split.next().as_deref(), Some("foo"));
            assert_eq!(split.next().as_deref(), Some("bar"));
            assert_eq!(split.next().as_deref(), Some(""));
            assert_eq!(split.next().as_deref(), Some("baz"));
            assert_eq!(split.next().as_deref(), Some(""));
            assert_eq!(split.next().as_deref(), None);

            Ok(())
        }

        #[test]
        fn all_empty() -> Result<(), io::Error> {
            let s = "<><>";
            let cursor = Cursor::new(String::from(s));
            let mut split =
                SplitRead::new(cursor, "<>").map(|r| String::from_utf8(r.unwrap()).unwrap());

            assert_eq!(split.next().as_deref(), Some(""));
            assert_eq!(split.next().as_deref(), Some(""));
            assert_eq!(split.next().as_deref(), Some(""));
            assert_eq!(split.next(), None);

            Ok(())
        }

        #[should_panic = "delimiter can't be empty"]
        #[test]
        fn empty_delimiter() {
            let s = "abc";
            let cursor = Cursor::new(String::from(s));
            let _split = SplitRead::new(cursor, "").map(|e| e.unwrap());
        }

        #[test]
        fn delimiter_spread_across_reads() {
            let reader = Cursor::new("<|>foo<|")
                .chain(Cursor::new(">bar<|><"))
                .chain(Cursor::new("|>baz<|>"));

            let mut split =
                SplitRead::new(reader, "<|>").map(|r| String::from_utf8(r.unwrap()).unwrap());

            assert_eq!(split.next().unwrap(), "");
            assert_eq!(split.next().unwrap(), "foo");
            assert_eq!(split.next().unwrap(), "bar");
            assert_eq!(split.next().unwrap(), "");
            assert_eq!(split.next().unwrap(), "baz");
            assert_eq!(split.next().unwrap(), "");
            assert_eq!(split.next(), None);
        }
    }
}

pub struct SplitRead {
    internal: split_read::SplitRead<BufReader<SourceReader>>,
    span: Span,
    signals: Signals,
}

impl SplitRead {
    fn new(
        reader: SourceReader,
        delimiter: impl AsRef<[u8]>,
        span: Span,
        signals: Signals,
    ) -> Self {
        Self {
            internal: split_read::SplitRead::new(BufReader::new(reader), delimiter),
            span,
            signals,
        }
    }

    pub fn span(&self) -> Span {
        self.span
    }
}

impl Iterator for SplitRead {
    type Item = Result<Vec<u8>, ShellError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.signals.interrupted() {
            return None;
        }
        self.internal.next().map(|r| {
            r.map_err(|err| {
                ShellError::Io(IoError::new_internal(
                    err,
                    "Could not get next value for SplitRead",
                    crate::location!(),
                ))
            })
        })
    }
}

/// Turn a readable stream into [`Value`]s.
///
/// The `Value` type depends on the type of the stream ([`ByteStreamType`]). If `Unknown`, the
/// stream will return strings as long as UTF-8 parsing succeeds, but will start returning binary
/// if it fails.
pub struct Chunks {
    reader: BufReader<SourceReader>,
    pos: u64,
    error: bool,
    span: Span,
    signals: Signals,
    type_: ByteStreamType,
}

impl Chunks {
    fn new(reader: SourceReader, span: Span, signals: Signals, type_: ByteStreamType) -> Self {
        Self {
            reader: BufReader::new(reader),
            pos: 0,
            error: false,
            span,
            signals,
            type_,
        }
    }

    pub fn span(&self) -> Span {
        self.span
    }

    fn next_string(&mut self) -> Result<Option<String>, (Vec<u8>, ShellError)> {
        let from_io_error = |err: std::io::Error| match ShellErrorBridge::try_from(err) {
            Ok(err) => err.0,
            Err(err) => IoError::new(err, self.span, None).into(),
        };

        // Get some data from the reader
        let buf = self
            .reader
            .fill_buf()
            .map_err(from_io_error)
            .map_err(|err| (vec![], err))?;

        // If empty, this is EOF
        if buf.is_empty() {
            return Ok(None);
        }

        let mut buf = buf.to_vec();
        let mut consumed = 0;

        // If the buf length is under 4 bytes, it could be invalid, so try to get more
        if buf.len() < 4 {
            consumed += buf.len();
            self.reader.consume(buf.len());
            match self.reader.fill_buf() {
                Ok(more_bytes) => buf.extend_from_slice(more_bytes),
                Err(err) => return Err((buf, from_io_error(err))),
            }
        }

        // Try to parse utf-8 and decide what to do
        match String::from_utf8(buf) {
            Ok(string) => {
                self.reader.consume(string.len() - consumed);
                self.pos += string.len() as u64;
                Ok(Some(string))
            }
            Err(err) if err.utf8_error().error_len().is_none() => {
                // There is some valid data at the beginning, and this is just incomplete, so just
                // consume that and return it
                let valid_up_to = err.utf8_error().valid_up_to();
                if valid_up_to > consumed {
                    self.reader.consume(valid_up_to - consumed);
                }
                let mut buf = err.into_bytes();
                buf.truncate(valid_up_to);
                buf.shrink_to_fit();
                let string = String::from_utf8(buf)
                    .expect("failed to parse utf-8 even after correcting error");
                self.pos += string.len() as u64;
                Ok(Some(string))
            }
            Err(err) => {
                // There is an error at the beginning and we have no hope of parsing further.
                let shell_error = ShellError::NonUtf8Custom {
                    msg: format!("invalid utf-8 sequence starting at index {}", self.pos),
                    span: self.span,
                };
                let buf = err.into_bytes();
                // We are consuming the entire buf though, because we're returning it in case it
                // will be cast to binary
                if buf.len() > consumed {
                    self.reader.consume(buf.len() - consumed);
                }
                self.pos += buf.len() as u64;
                Err((buf, shell_error))
            }
        }
    }
}

impl Iterator for Chunks {
    type Item = Result<Value, ShellError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.error || self.signals.interrupted() {
            None
        } else {
            match self.type_ {
                // Binary should always be binary
                ByteStreamType::Binary => {
                    let buf = match self.reader.fill_buf() {
                        Ok(buf) => buf,
                        Err(err) => {
                            self.error = true;
                            return Some(Err(ShellError::Io(IoError::new(err, self.span, None))));
                        }
                    };
                    if !buf.is_empty() {
                        let len = buf.len();
                        let value = Value::binary(buf, self.span);
                        self.reader.consume(len);
                        self.pos += len as u64;
                        Some(Ok(value))
                    } else {
                        None
                    }
                }
                // String produces an error if UTF-8 can't be parsed
                ByteStreamType::String => match self.next_string().transpose()? {
                    Ok(string) => Some(Ok(Value::string(string, self.span))),
                    Err((_, err)) => {
                        self.error = true;
                        Some(Err(err))
                    }
                },
                // For Unknown, we try to create strings, but we switch to binary mode if we
                // fail
                ByteStreamType::Unknown => {
                    match self.next_string().transpose()? {
                        Ok(string) => Some(Ok(Value::string(string, self.span))),
                        Err((buf, _)) if !buf.is_empty() => {
                            // Switch to binary mode
                            self.type_ = ByteStreamType::Binary;
                            Some(Ok(Value::binary(buf, self.span)))
                        }
                        Err((_, err)) => {
                            self.error = true;
                            Some(Err(err))
                        }
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

#[cfg(unix)]
pub(crate) fn convert_file<T: From<OwnedFd>>(file: impl Into<OwnedFd>) -> T {
    file.into().into()
}

#[cfg(windows)]
pub(crate) fn convert_file<T: From<OwnedHandle>>(file: impl Into<OwnedHandle>) -> T {
    file.into().into()
}

const DEFAULT_BUF_SIZE: usize = 8192;

pub fn copy_with_signals(
    mut reader: impl Read,
    mut writer: impl Write,
    span: Span,
    signals: &Signals,
) -> Result<u64, ShellError> {
    let from_io_error = IoError::factory(span, None);
    if signals.is_empty() {
        match io::copy(&mut reader, &mut writer) {
            Ok(n) => {
                writer.flush().map_err(&from_io_error)?;
                Ok(n)
            }
            Err(err) => {
                let _ = writer.flush();
                match ShellErrorBridge::try_from(err) {
                    Ok(ShellErrorBridge(shell_error)) => Err(shell_error),
                    Err(err) => Err(from_io_error(err).into()),
                }
            }
        }
    } else {
        // #[cfg(any(target_os = "linux", target_os = "android"))]
        // {
        //     return crate::sys::kernel_copy::copy_spec(reader, writer);
        // }
        match generic_copy(&mut reader, &mut writer, span, signals) {
            Ok(len) => {
                writer.flush().map_err(&from_io_error)?;
                Ok(len)
            }
            Err(err) => {
                let _ = writer.flush();
                Err(err)
            }
        }
    }
}

// Copied from [`std::io::copy`]
fn generic_copy(
    mut reader: impl Read,
    mut writer: impl Write,
    span: Span,
    signals: &Signals,
) -> Result<u64, ShellError> {
    let from_io_error = IoError::factory(span, None);
    let buf = &mut [0; DEFAULT_BUF_SIZE];
    let mut len = 0;
    loop {
        signals.check(span)?;
        let n = match reader.read(buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => match ShellErrorBridge::try_from(e) {
                Ok(ShellErrorBridge(e)) => return Err(e),
                Err(e) => return Err(from_io_error(e).into()),
            },
        };
        len += n;
        writer.write_all(&buf[..n]).map_err(&from_io_error)?;
    }
    Ok(len as u64)
}

struct ReadGenerator<F>
where
    F: FnMut(&mut Vec<u8>) -> Result<bool, ShellError> + Send + 'static,
{
    buffer: Cursor<Vec<u8>>,
    generator: F,
}

impl<F> BufRead for ReadGenerator<F>
where
    F: FnMut(&mut Vec<u8>) -> Result<bool, ShellError> + Send + 'static,
{
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        // We have to loop, because it's important that we don't leave the buffer empty unless we're
        // truly at the end of the stream.
        while self.buffer.fill_buf()?.is_empty() {
            // Reset the cursor to the beginning and truncate
            self.buffer.set_position(0);
            self.buffer.get_mut().clear();
            // Ask the generator to generate data
            if !(self.generator)(self.buffer.get_mut()).map_err(ShellErrorBridge)? {
                // End of stream
                break;
            }
        }
        self.buffer.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.buffer.consume(amt);
    }
}

impl<F> Read for ReadGenerator<F>
where
    F: FnMut(&mut Vec<u8>) -> Result<bool, ShellError> + Send + 'static,
{
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // Straightforward implementation on top of BufRead
        let slice = self.fill_buf()?;
        let len = buf.len().min(slice.len());
        buf[..len].copy_from_slice(&slice[..len]);
        self.consume(len);
        Ok(len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_chunks<T>(data: Vec<T>, type_: ByteStreamType) -> Chunks
    where
        T: AsRef<[u8]> + Default + Send + 'static,
    {
        let reader = ReadIterator {
            iter: data.into_iter(),
            cursor: Some(Cursor::new(T::default())),
        };
        Chunks::new(
            SourceReader::Read(Box::new(reader)),
            Span::test_data(),
            Signals::empty(),
            type_,
        )
    }

    #[test]
    fn chunks_read_binary_passthrough() {
        let bins = vec![&[0, 1][..], &[2, 3][..]];
        let iter = test_chunks(bins.clone(), ByteStreamType::Binary);

        let bins_values: Vec<Value> = bins
            .into_iter()
            .map(|bin| Value::binary(bin, Span::test_data()))
            .collect();
        assert_eq!(
            bins_values,
            iter.collect::<Result<Vec<Value>, _>>().expect("error")
        );
    }

    #[test]
    fn chunks_read_string_clean() {
        let strs = vec!["Nushell", "が好きです"];
        let iter = test_chunks(strs.clone(), ByteStreamType::String);

        let strs_values: Vec<Value> = strs
            .into_iter()
            .map(|string| Value::string(string, Span::test_data()))
            .collect();
        assert_eq!(
            strs_values,
            iter.collect::<Result<Vec<Value>, _>>().expect("error")
        );
    }

    #[test]
    fn chunks_read_string_split_boundary() {
        let real = "Nushell最高!";
        let chunks = vec![&b"Nushell\xe6"[..], &b"\x9c\x80\xe9"[..], &b"\xab\x98!"[..]];
        let iter = test_chunks(chunks.clone(), ByteStreamType::String);

        let mut string = String::new();
        for value in iter {
            let chunk_string = value.expect("error").into_string().expect("not a string");
            string.push_str(&chunk_string);
        }
        assert_eq!(real, string);
    }

    #[test]
    fn chunks_read_string_utf8_error() {
        let chunks = vec![&b"Nushell\xe6"[..], &b"\x9c\x80\xe9"[..], &b"\xab"[..]];
        let iter = test_chunks(chunks, ByteStreamType::String);

        let mut string = String::new();
        for value in iter {
            match value {
                Ok(value) => string.push_str(&value.into_string().expect("not a string")),
                Err(err) => {
                    println!("string so far: {string:?}");
                    println!("got error: {err:?}");
                    assert!(!string.is_empty());
                    assert!(matches!(err, ShellError::NonUtf8Custom { .. }));
                    return;
                }
            }
        }
        panic!("no error");
    }

    #[test]
    fn chunks_read_unknown_fallback() {
        let chunks = vec![&b"Nushell"[..], &b"\x9c\x80\xe9abcd"[..], &b"efgh"[..]];
        let mut iter = test_chunks(chunks, ByteStreamType::Unknown);

        let mut get = || iter.next().expect("end of iter").expect("error");

        assert_eq!(Value::test_string("Nushell"), get());
        assert_eq!(Value::test_binary(b"\x9c\x80\xe9abcd"), get());
        // Once it's in binary mode it won't go back
        assert_eq!(Value::test_binary(b"efgh"), get());
    }
}
