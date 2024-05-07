use crate::{
    io::{convert_file, copy_with_interrupt},
    process::{ChildPipe, ChildProcess, ExitStatus},
    ErrSpan, IntoSpanned, OutDest, PipelineData, ShellError, Span, Value,
};
use std::{
    fmt::Debug,
    fs::File,
    io::{self, BufRead, BufReader, Cursor, Read, Write},
    process::Stdio,
    sync::{atomic::AtomicBool, Arc},
    thread,
};

pub enum ByteStreamSource {
    Read(Box<dyn Read + Send + 'static>),
    File(File),
    Child(Box<ChildProcess>),
}

impl ByteStreamSource {
    fn reader(self) -> Option<ByteStreamSourceReader> {
        match self {
            ByteStreamSource::Read(read) => Some(ByteStreamSourceReader::Read(read)),
            ByteStreamSource::File(file) => Some(ByteStreamSourceReader::File(file)),
            ByteStreamSource::Child(mut child) => child.stdout.take().map(|stdout| match stdout {
                ChildPipe::Pipe(pipe) => ByteStreamSourceReader::File(convert_file(pipe)),
                ChildPipe::Tee(tee) => ByteStreamSourceReader::Read(tee),
            }),
        }
    }
}

enum ByteStreamSourceReader {
    Read(Box<dyn Read + Send + 'static>),
    File(File),
}

impl Read for ByteStreamSourceReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            ByteStreamSourceReader::Read(reader) => reader.read(buf),
            ByteStreamSourceReader::File(file) => file.read(buf),
        }
    }
}

pub struct ByteStream {
    stream: ByteStreamSource,
    span: Span,
    ctrlc: Option<Arc<AtomicBool>>,
    known_size: Option<u64>,
}

impl ByteStream {
    pub fn new(stream: ByteStreamSource, span: Span, interrupt: Option<Arc<AtomicBool>>) -> Self {
        Self {
            stream,
            span,
            ctrlc: interrupt,
            known_size: None,
        }
    }

    pub fn read(
        reader: impl Read + Send + 'static,
        span: Span,
        interrupt: Option<Arc<AtomicBool>>,
    ) -> Self {
        Self::new(ByteStreamSource::Read(Box::new(reader)), span, interrupt)
    }

    pub fn file(file: File, span: Span, interrupt: Option<Arc<AtomicBool>>) -> Self {
        Self::new(ByteStreamSource::File(file), span, interrupt)
    }

    pub fn child(child: ChildProcess, span: Span) -> Self {
        Self::new(ByteStreamSource::Child(Box::new(child)), span, None)
    }

    pub fn stdin(span: Span) -> Result<Self, ShellError> {
        let stdin = os_pipe::dup_stdin().err_span(span)?;
        let source = ByteStreamSource::File(convert_file(stdin));
        Ok(Self::new(source, span, None))
    }

    pub fn from_iter<T>(
        iter: impl Iterator<Item = T> + Send + 'static,
        span: Span,
        interrupt: Option<Arc<AtomicBool>>,
    ) -> Self
    where
        T: AsRef<[u8]> + Send + Default + 'static,
    {
        Self::read(ReadIterator::new(iter), span, interrupt)
    }

    pub fn from_result_iter<T>(
        iter: impl Iterator<Item = Result<T, ShellError>> + Send + 'static,
        span: Span,
        interrupt: Option<Arc<AtomicBool>>,
    ) -> Self
    where
        T: AsRef<[u8]> + Send + Default + 'static,
    {
        Self::read(ReadResultIterator::new(iter), span, interrupt)
    }

    pub fn with_known_size(mut self, size: Option<u64>) -> Self {
        self.known_size = size;
        self
    }

    pub fn source(&self) -> &ByteStreamSource {
        &self.stream
    }

    pub fn source_mut(&mut self) -> &mut ByteStreamSource {
        &mut self.stream
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn known_size(&self) -> Option<u64> {
        self.known_size
    }

    pub fn reader(self) -> Option<ByteStreamReader> {
        let reader = self.stream.reader()?;
        Some(ByteStreamReader {
            reader: BufReader::new(reader),
            span: self.span,
            ctrlc: self.ctrlc,
        })
    }

    pub fn lines(self) -> Option<Lines> {
        let reader = self.stream.reader()?;
        Some(Lines {
            lines: ByteLines::new(reader),
            span: self.span,
            ctrlc: self.ctrlc,
        })
    }

    pub fn values(self) -> Option<Values> {
        let reader = self.stream.reader()?;
        Some(Values {
            lines: ByteLines::new(reader),
            span: self.span,
            ctrlc: self.ctrlc,
        })
    }

    pub fn into_source(self) -> ByteStreamSource {
        self.stream
    }

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
                    if stderr.is_some() {
                        debug_assert!(false, "stderr should not exist");
                    }
                    Ok(stdout.into())
                } else {
                    self.stream = ByteStreamSource::Child(child);
                    Err(self)
                }
            }
        }
    }

    pub fn into_child(self) -> Result<ChildProcess, Self> {
        match self.stream {
            ByteStreamSource::Child(child) => Ok(*child),
            _ => Err(self),
        }
    }

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

    pub fn into_string(self) -> Result<String, ShellError> {
        let trim = matches!(self.stream, ByteStreamSource::Child(..));
        let span = self.span;
        let bytes = self.into_bytes()?;
        let mut string = String::from_utf8(bytes).map_err(|_| ShellError::NonUtf8 { span })?;
        if trim {
            trim_end_newline(&mut string);
        }
        Ok(string)
    }

    pub fn into_value(self) -> Result<Value, ShellError> {
        let trim = matches!(self.stream, ByteStreamSource::Child(..));
        let span = self.span;
        let bytes = self.into_bytes()?;
        let value = match String::from_utf8(bytes) {
            Ok(mut str) => {
                if trim {
                    trim_end_newline(&mut str);
                }
                Value::string(str, span)
            }
            Err(err) => Value::binary(err.into_bytes(), span),
        };
        Ok(value)
    }

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

    fn print_to(self, mut dest: impl Write) -> Result<Option<ExitStatus>, ShellError> {
        let span = self.span;
        let ctrlc = self.ctrlc.as_deref();

        match self.stream {
            ByteStreamSource::Read(mut read) => {
                copy_with_interrupt(&mut read, &mut dest, span, ctrlc)?;
                Ok(None)
            }
            ByteStreamSource::File(mut file) => {
                copy_with_interrupt(&mut file, &mut dest, span, ctrlc)?;
                Ok(None)
            }
            ByteStreamSource::Child(mut child) => {
                match (child.stdout.take(), child.stderr.take()) {
                    (Some(stdout), Some(stderr)) => {
                        thread::scope(|s| {
                            // To avoid deadlocks, we must spawn a separate thread to wait on stderr.
                            let err_thread = thread::Builder::new()
                                .spawn_scoped(s, move || match stderr {
                                    ChildPipe::Pipe(mut pipe) => copy_with_interrupt(
                                        &mut pipe,
                                        &mut io::stderr(),
                                        span,
                                        ctrlc,
                                    ),
                                    ChildPipe::Tee(mut tee) => copy_with_interrupt(
                                        &mut tee,
                                        &mut io::stderr(),
                                        span,
                                        ctrlc,
                                    ),
                                })
                                .err_span(span);

                            match stdout {
                                ChildPipe::Pipe(mut pipe) => {
                                    copy_with_interrupt(&mut pipe, &mut dest, span, ctrlc)
                                }
                                ChildPipe::Tee(mut tee) => {
                                    copy_with_interrupt(&mut tee, &mut dest, span, ctrlc)
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
                    (Some(mut stdout), None) => {
                        // single output stream, we can consume directly
                        copy_with_interrupt(&mut stdout, &mut dest, span, ctrlc)?;
                    }
                    (None, Some(mut stderr)) => {
                        // single output stream, we can consume directly
                        copy_with_interrupt(&mut stderr, &mut io::stderr(), span, ctrlc)?;
                    }
                    (None, None) => {}
                }
                Ok(Some(child.wait()?))
            }
        }
    }

    pub fn print(self, to_stderr: bool) -> Result<Option<ExitStatus>, ShellError> {
        if to_stderr {
            self.print_to(io::stderr())
        } else {
            self.print_to(io::stdout())
        }
    }

    pub fn write_to(self, dest: &mut impl Write) -> Result<(), ShellError> {
        let span = self.span;
        let ctrlc = self.ctrlc.as_deref();
        if let Some(reader) = self.stream.reader() {
            match reader {
                ByteStreamSourceReader::Read(mut reader) => {
                    copy_with_interrupt(&mut reader, dest, span, ctrlc)?;
                }
                ByteStreamSourceReader::File(mut file) => {
                    copy_with_interrupt(&mut file, dest, span, ctrlc)?;
                }
            };
        }
        Ok(())
    }

    pub fn write_to_out_dests(
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

pub struct ByteStreamReader {
    reader: BufReader<ByteStreamSourceReader>,
    span: Span,
    ctrlc: Option<Arc<AtomicBool>>,
}

impl ByteStreamReader {
    pub fn span(&self) -> Span {
        self.span
    }
}

impl Read for ByteStreamReader {
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

impl BufRead for ByteStreamReader {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.reader.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.reader.consume(amt)
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

impl<I> ReadIterator<I>
where
    I: Iterator,
    I::Item: AsRef<[u8]> + Default,
{
    pub fn new(iter: I) -> Self {
        Self {
            iter: iter.into_iter(),
            cursor: Some(Cursor::new(I::Item::default())),
        }
    }
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

impl<I, T> ReadResultIterator<I, T>
where
    I: Iterator<Item = Result<T, ShellError>>,
    T: AsRef<[u8]> + Default,
{
    pub fn new(iter: I) -> Self {
        Self {
            iter: iter.into_iter(),
            cursor: Some(Cursor::new(T::default())),
        }
    }
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

pub struct Lines {
    lines: ByteLines<ByteStreamSourceReader>,
    span: Span,
    ctrlc: Option<Arc<AtomicBool>>,
}

impl Lines {
    pub fn span(&self) -> Span {
        self.span
    }
}

impl Iterator for Lines {
    type Item = Result<Vec<u8>, ShellError>;

    fn next(&mut self) -> Option<Self::Item> {
        if nu_utils::ctrl_c::was_pressed(&self.ctrlc) {
            None
        } else {
            match self.lines.next() {
                Some(Ok(line)) => Some(Ok(line)),
                Some(Err(err)) => Some(Err(err.into_spanned(self.span).into())),
                None => None,
            }
        }
    }
}

pub struct Values {
    lines: ByteLines<ByteStreamSourceReader>,
    span: Span,
    ctrlc: Option<Arc<AtomicBool>>,
}

impl Values {
    pub fn span(&self) -> Span {
        self.span
    }
}

impl Iterator for Values {
    type Item = Result<Value, ShellError>;

    fn next(&mut self) -> Option<Self::Item> {
        if nu_utils::ctrl_c::was_pressed(&self.ctrlc) {
            None
        } else {
            match self.lines.next() {
                Some(Ok(line)) => Some(Ok(match String::from_utf8(line) {
                    Ok(str) => Value::string(str, self.span),
                    Err(err) => Value::binary(err.into_bytes(), self.span),
                })),
                Some(Err(err)) => Some(Err(err.into_spanned(self.span).into())),
                None => None,
            }
        }
    }
}

struct ByteLines<R: Read>(BufReader<R>);

impl<R: Read> ByteLines<R> {
    pub fn new(read: R) -> Self {
        Self(BufReader::new(read))
    }
}

impl<R: Read> Iterator for ByteLines<R> {
    type Item = io::Result<Vec<u8>>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut buf = Vec::new();
        // `read_until` will never stop reading unless `\n` or EOF is encountered,
        // so we may want to limit the number of bytes by using `take` as the Rust docs suggest.
        // let capacity = self.0.capacity() as u64;
        // let mut reader = (&mut self.0).take(capacity);
        let reader = &mut self.0;
        match reader.read_until(b'\n', &mut buf) {
            Ok(0) => None,
            Ok(_) => {
                if buf.ends_with(&[b'\n']) {
                    let _ = buf.pop();
                    if buf.ends_with(&[b'\r']) {
                        let _ = buf.pop();
                    }
                }
                Some(Ok(buf))
            }
            Err(e) => Some(Err(e)),
        }
    }
}
