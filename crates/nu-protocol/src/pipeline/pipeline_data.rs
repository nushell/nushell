use crate::{
    ast::{Call, PathMember},
    engine::{EngineState, Stack},
    process::{ChildPipe, ChildProcess, ExitStatus},
    ByteStream, ByteStreamType, Config, ErrSpan, ListStream, OutDest, PipelineMetadata, Range,
    ShellError, Span, Type, Value,
};
use nu_utils::{stderr_write_all_and_flush, stdout_write_all_and_flush};
use std::{
    io::{Cursor, Read, Write},
    sync::{atomic::AtomicBool, Arc},
};

const LINE_ENDING_PATTERN: &[char] = &['\r', '\n'];

/// The foundational abstraction for input and output to commands
///
/// This represents either a single Value or a stream of values coming into the command or leaving a command.
///
/// A note on implementation:
///
/// We've tried a few variations of this structure. Listing these below so we have a record.
///
/// * We tried always assuming a stream in Nushell. This was a great 80% solution, but it had some rough edges.
/// Namely, how do you know the difference between a single string and a list of one string. How do you know
/// when to flatten the data given to you from a data source into the stream or to keep it as an unflattened
/// list?
///
/// * We tried putting the stream into Value. This had some interesting properties as now commands "just worked
/// on values", but lead to a few unfortunate issues.
///
/// The first is that you can't easily clone Values in a way that felt largely immutable. For example, if
/// you cloned a Value which contained a stream, and in one variable drained some part of it, then the second
/// variable would see different values based on what you did to the first.
///
/// To make this kind of mutation thread-safe, we would have had to produce a lock for the stream, which in
/// practice would have meant always locking the stream before reading from it. But more fundamentally, it
/// felt wrong in practice that observation of a value at runtime could affect other values which happen to
/// alias the same stream. By separating these, we don't have this effect. Instead, variables could get
/// concrete list values rather than streams, and be able to view them without non-local effects.
///
/// * A balance of the two approaches is what we've landed on: Values are thread-safe to pass, and we can stream
/// them into any sources. Streams are still available to model the infinite streams approach of original
/// Nushell.
#[derive(Debug)]
pub enum PipelineData {
    Empty,
    Value(Value, Option<PipelineMetadata>),
    ListStream(ListStream, Option<PipelineMetadata>),
    ByteStream(ByteStream, Option<PipelineMetadata>),
}

impl PipelineData {
    pub fn empty() -> PipelineData {
        PipelineData::Empty
    }

    /// create a `PipelineData::ByteStream` with proper exit_code
    ///
    /// It's useful to break running without raising error at user level.
    pub fn new_external_stream_with_only_exit_code(exit_code: i32) -> PipelineData {
        let span = Span::unknown();
        let mut child = ChildProcess::from_raw(None, None, None, span);
        child.set_exit_code(exit_code);
        PipelineData::ByteStream(ByteStream::child(child, span), None)
    }

    pub fn metadata(&self) -> Option<PipelineMetadata> {
        match self {
            PipelineData::Empty => None,
            PipelineData::Value(_, meta)
            | PipelineData::ListStream(_, meta)
            | PipelineData::ByteStream(_, meta) => meta.clone(),
        }
    }

    pub fn set_metadata(mut self, metadata: Option<PipelineMetadata>) -> Self {
        match &mut self {
            PipelineData::Empty => {}
            PipelineData::Value(_, meta)
            | PipelineData::ListStream(_, meta)
            | PipelineData::ByteStream(_, meta) => *meta = metadata,
        }
        self
    }

    pub fn is_nothing(&self) -> bool {
        matches!(self, PipelineData::Value(Value::Nothing { .. }, ..))
            || matches!(self, PipelineData::Empty)
    }

    /// PipelineData doesn't always have a Span, but we can try!
    pub fn span(&self) -> Option<Span> {
        match self {
            PipelineData::Empty => None,
            PipelineData::Value(value, ..) => Some(value.span()),
            PipelineData::ListStream(stream, ..) => Some(stream.span()),
            PipelineData::ByteStream(stream, ..) => Some(stream.span()),
        }
    }

    /// Get a type that is representative of the `PipelineData`.
    ///
    /// The type returned here makes no effort to collect a stream, so it may be a different type
    /// than would be returned by [`Value::get_type()`] on the result of [`.into_value()`].
    ///
    /// Specifically, a `ListStream` results in [`list stream`](Type::ListStream) rather than
    /// the fully complete [`list`](Type::List) type (which would require knowing the contents),
    /// and a `ByteStream` with [unknown](crate::ByteStreamType::Unknown) type results in
    /// [`any`](Type::Any) rather than [`string`](Type::String) or [`binary`](Type::Binary).
    pub fn get_type(&self) -> Type {
        match self {
            PipelineData::Empty => Type::Nothing,
            PipelineData::Value(value, _) => value.get_type(),
            PipelineData::ListStream(_, _) => Type::ListStream,
            PipelineData::ByteStream(stream, _) => stream.type_().into(),
        }
    }

    pub fn into_value(self, span: Span) -> Result<Value, ShellError> {
        match self {
            PipelineData::Empty => Ok(Value::nothing(span)),
            PipelineData::Value(value, ..) => Ok(value.with_span(span)),
            PipelineData::ListStream(stream, ..) => Ok(stream.into_value()),
            PipelineData::ByteStream(stream, ..) => stream.into_value(),
        }
    }

    /// Writes all values or redirects all output to the current [`OutDest`]s in `stack`.
    ///
    /// For [`OutDest::Pipe`] and [`OutDest::Capture`], this will return the `PipelineData` as is
    /// without consuming input and without writing anything.
    ///
    /// For the other [`OutDest`]s, the given `PipelineData` will be completely consumed
    /// and `PipelineData::Empty` will be returned.
    pub fn write_to_out_dests(
        self,
        engine_state: &EngineState,
        stack: &mut Stack,
    ) -> Result<PipelineData, ShellError> {
        match (self, stack.stdout()) {
            (PipelineData::ByteStream(stream, ..), stdout) => {
                stream.write_to_out_dests(stdout, stack.stderr())?;
            }
            (data, OutDest::Pipe | OutDest::Capture) => return Ok(data),
            (PipelineData::Empty, ..) => {}
            (PipelineData::Value(..), OutDest::Null) => {}
            (PipelineData::ListStream(stream, ..), OutDest::Null) => {
                // we need to drain the stream in case there are external commands in the pipeline
                stream.drain()?;
            }
            (PipelineData::Value(value, ..), OutDest::File(file)) => {
                let bytes = value_to_bytes(value)?;
                let mut file = file.as_ref();
                file.write_all(&bytes)?;
                file.flush()?;
            }
            (PipelineData::ListStream(stream, ..), OutDest::File(file)) => {
                let mut file = file.as_ref();
                // use BufWriter here?
                for value in stream {
                    let bytes = value_to_bytes(value)?;
                    file.write_all(&bytes)?;
                    file.write_all(b"\n")?;
                }
                file.flush()?;
            }
            (data @ (PipelineData::Value(..) | PipelineData::ListStream(..)), OutDest::Inherit) => {
                data.print(engine_state, stack, false, false)?;
            }
        }
        Ok(PipelineData::Empty)
    }

    pub fn drain(self) -> Result<Option<ExitStatus>, ShellError> {
        match self {
            PipelineData::Empty => Ok(None),
            PipelineData::Value(Value::Error { error, .. }, ..) => Err(*error),
            PipelineData::Value(..) => Ok(None),
            PipelineData::ListStream(stream, ..) => {
                stream.drain()?;
                Ok(None)
            }
            PipelineData::ByteStream(stream, ..) => stream.drain(),
        }
    }

    /// Try convert from self into iterator
    ///
    /// It returns Err if the `self` cannot be converted to an iterator.
    ///
    /// The `span` should be the span of the command or operation that would raise an error.
    pub fn into_iter_strict(self, span: Span) -> Result<PipelineIterator, ShellError> {
        Ok(PipelineIterator(match self {
            PipelineData::Value(value, ..) => {
                let val_span = value.span();
                match value {
                    Value::List { vals, .. } => PipelineIteratorInner::ListStream(
                        ListStream::new(vals.into_iter(), val_span, None).into_iter(),
                    ),
                    Value::Binary { val, .. } => PipelineIteratorInner::ListStream(
                        ListStream::new(
                            val.into_iter().map(move |x| Value::int(x as i64, val_span)),
                            val_span,
                            None,
                        )
                        .into_iter(),
                    ),
                    Value::Range { val, .. } => PipelineIteratorInner::ListStream(
                        ListStream::new(val.into_range_iter(val_span, None), val_span, None)
                            .into_iter(),
                    ),
                    // Propagate errors by explicitly matching them before the final case.
                    Value::Error { error, .. } => return Err(*error),
                    other => {
                        return Err(ShellError::OnlySupportsThisInputType {
                            exp_input_type: "list, binary, range, or byte stream".into(),
                            wrong_type: other.get_type().to_string(),
                            dst_span: span,
                            src_span: val_span,
                        })
                    }
                }
            }
            PipelineData::ListStream(stream, ..) => {
                PipelineIteratorInner::ListStream(stream.into_iter())
            }
            PipelineData::Empty => {
                return Err(ShellError::OnlySupportsThisInputType {
                    exp_input_type: "list, binary, range, or byte stream".into(),
                    wrong_type: "null".into(),
                    dst_span: span,
                    src_span: span,
                })
            }
            PipelineData::ByteStream(stream, ..) => {
                if let Some(chunks) = stream.chunks() {
                    PipelineIteratorInner::ByteStream(chunks)
                } else {
                    PipelineIteratorInner::Empty
                }
            }
        }))
    }

    pub fn collect_string(self, separator: &str, config: &Config) -> Result<String, ShellError> {
        match self {
            PipelineData::Empty => Ok(String::new()),
            PipelineData::Value(value, ..) => Ok(value.to_expanded_string(separator, config)),
            PipelineData::ListStream(stream, ..) => Ok(stream.into_string(separator, config)),
            PipelineData::ByteStream(stream, ..) => stream.into_string(),
        }
    }

    /// Retrieves string from pipeline data.
    ///
    /// As opposed to `collect_string` this raises error rather than converting non-string values.
    /// The `span` will be used if `ListStream` is encountered since it doesn't carry a span.
    pub fn collect_string_strict(
        self,
        span: Span,
    ) -> Result<(String, Span, Option<PipelineMetadata>), ShellError> {
        match self {
            PipelineData::Empty => Ok((String::new(), span, None)),
            PipelineData::Value(Value::String { val, .. }, metadata) => Ok((val, span, metadata)),
            PipelineData::Value(val, ..) => Err(ShellError::TypeMismatch {
                err_message: "string".into(),
                span: val.span(),
            }),
            PipelineData::ListStream(..) => Err(ShellError::TypeMismatch {
                err_message: "string".into(),
                span,
            }),
            PipelineData::ByteStream(stream, metadata) => {
                let span = stream.span();
                Ok((stream.into_string()?, span, metadata))
            }
        }
    }

    pub fn follow_cell_path(
        self,
        cell_path: &[PathMember],
        head: Span,
        insensitive: bool,
    ) -> Result<Value, ShellError> {
        match self {
            // FIXME: there are probably better ways of doing this
            PipelineData::ListStream(stream, ..) => Value::list(stream.into_iter().collect(), head)
                .follow_cell_path(cell_path, insensitive),
            PipelineData::Value(v, ..) => v.follow_cell_path(cell_path, insensitive),
            PipelineData::Empty => Err(ShellError::IncompatiblePathAccess {
                type_name: "empty pipeline".to_string(),
                span: head,
            }),
            PipelineData::ByteStream(stream, ..) => Err(ShellError::IncompatiblePathAccess {
                type_name: stream.type_().describe().to_owned(),
                span: stream.span(),
            }),
        }
    }

    /// Simplified mapper to help with simple values also. For full iterator support use `.into_iter()` instead
    pub fn map<F>(
        self,
        mut f: F,
        ctrlc: Option<Arc<AtomicBool>>,
    ) -> Result<PipelineData, ShellError>
    where
        Self: Sized,
        F: FnMut(Value) -> Value + 'static + Send,
    {
        match self {
            PipelineData::Value(value, metadata) => {
                let span = value.span();
                let pipeline = match value {
                    Value::List { vals, .. } => {
                        vals.into_iter().map(f).into_pipeline_data(span, ctrlc)
                    }
                    Value::Range { val, .. } => val
                        .into_range_iter(span, ctrlc.clone())
                        .map(f)
                        .into_pipeline_data(span, ctrlc),
                    value => match f(value) {
                        Value::Error { error, .. } => return Err(*error),
                        v => v.into_pipeline_data(),
                    },
                };
                Ok(pipeline.set_metadata(metadata))
            }
            PipelineData::Empty => Ok(PipelineData::Empty),
            PipelineData::ListStream(stream, metadata) => {
                Ok(PipelineData::ListStream(stream.map(f), metadata))
            }
            PipelineData::ByteStream(stream, metadata) => {
                Ok(f(stream.into_value()?).into_pipeline_data_with_metadata(metadata))
            }
        }
    }

    /// Simplified flatmapper. For full iterator support use `.into_iter()` instead
    pub fn flat_map<U, F>(
        self,
        mut f: F,
        ctrlc: Option<Arc<AtomicBool>>,
    ) -> Result<PipelineData, ShellError>
    where
        Self: Sized,
        U: IntoIterator<Item = Value> + 'static,
        <U as IntoIterator>::IntoIter: 'static + Send,
        F: FnMut(Value) -> U + 'static + Send,
    {
        match self {
            PipelineData::Empty => Ok(PipelineData::Empty),
            PipelineData::Value(value, metadata) => {
                let span = value.span();
                let pipeline = match value {
                    Value::List { vals, .. } => {
                        vals.into_iter().flat_map(f).into_pipeline_data(span, ctrlc)
                    }
                    Value::Range { val, .. } => val
                        .into_range_iter(span, ctrlc.clone())
                        .flat_map(f)
                        .into_pipeline_data(span, ctrlc),
                    value => f(value).into_iter().into_pipeline_data(span, ctrlc),
                };
                Ok(pipeline.set_metadata(metadata))
            }
            PipelineData::ListStream(stream, metadata) => Ok(PipelineData::ListStream(
                stream.modify(|iter| iter.flat_map(f)),
                metadata,
            )),
            PipelineData::ByteStream(stream, metadata) => {
                // TODO: is this behavior desired / correct ?
                let span = stream.span();
                let iter = match String::from_utf8(stream.into_bytes()?) {
                    Ok(mut str) => {
                        str.truncate(str.trim_end_matches(LINE_ENDING_PATTERN).len());
                        f(Value::string(str, span))
                    }
                    Err(err) => f(Value::binary(err.into_bytes(), span)),
                };
                Ok(iter
                    .into_iter()
                    .into_pipeline_data_with_metadata(span, ctrlc, metadata))
            }
        }
    }

    pub fn filter<F>(
        self,
        mut f: F,
        ctrlc: Option<Arc<AtomicBool>>,
    ) -> Result<PipelineData, ShellError>
    where
        Self: Sized,
        F: FnMut(&Value) -> bool + 'static + Send,
    {
        match self {
            PipelineData::Empty => Ok(PipelineData::Empty),
            PipelineData::Value(value, metadata) => {
                let span = value.span();
                let pipeline = match value {
                    Value::List { vals, .. } => {
                        vals.into_iter().filter(f).into_pipeline_data(span, ctrlc)
                    }
                    Value::Range { val, .. } => val
                        .into_range_iter(span, ctrlc.clone())
                        .filter(f)
                        .into_pipeline_data(span, ctrlc),
                    value => {
                        if f(&value) {
                            value.into_pipeline_data()
                        } else {
                            Value::nothing(span).into_pipeline_data()
                        }
                    }
                };
                Ok(pipeline.set_metadata(metadata))
            }
            PipelineData::ListStream(stream, metadata) => Ok(PipelineData::ListStream(
                stream.modify(|iter| iter.filter(f)),
                metadata,
            )),
            PipelineData::ByteStream(stream, metadata) => {
                // TODO: is this behavior desired / correct ?
                let span = stream.span();
                let value = match String::from_utf8(stream.into_bytes()?) {
                    Ok(mut str) => {
                        str.truncate(str.trim_end_matches(LINE_ENDING_PATTERN).len());
                        Value::string(str, span)
                    }
                    Err(err) => Value::binary(err.into_bytes(), span),
                };
                let value = if f(&value) {
                    value
                } else {
                    Value::nothing(span)
                };
                Ok(value.into_pipeline_data_with_metadata(metadata))
            }
        }
    }

    /// Try to catch the external command exit status and detect if it failed.
    ///
    /// This is useful for external commands with semicolon, we can detect errors early to avoid
    /// commands after the semicolon running.
    ///
    /// Returns `self` and a flag that indicates if the external command run failed. If `self` is
    /// not [`PipelineData::ByteStream`], the flag will be `false`.
    ///
    /// Currently this will consume an external command to completion.
    pub fn check_external_failed(self) -> Result<(Self, bool), ShellError> {
        if let PipelineData::ByteStream(stream, metadata) = self {
            let span = stream.span();
            match stream.into_child() {
                Ok(mut child) => {
                    // Only check children without stdout. This means that nothing
                    // later in the pipeline can possibly consume output from this external command.
                    if child.stdout.is_none() {
                        // Note:
                        // In run-external's implementation detail, the result sender thread
                        // send out stderr message first, then stdout message, then exit_code.
                        //
                        // In this clause, we already make sure that `stdout` is None
                        // But not the case of `stderr`, so if `stderr` is not None
                        // We need to consume stderr message before reading external commands' exit code.
                        //
                        // Or we'll never have a chance to read exit_code if stderr producer produce too much stderr message.
                        // So we consume stderr stream and rebuild it.
                        let stderr = child
                            .stderr
                            .take()
                            .map(|mut stderr| {
                                let mut buf = Vec::new();
                                stderr.read_to_end(&mut buf).err_span(span)?;
                                Ok::<_, ShellError>(buf)
                            })
                            .transpose()?;

                        let code = child.wait()?.code();
                        let mut child = ChildProcess::from_raw(None, None, None, span);
                        if let Some(stderr) = stderr {
                            child.stderr = Some(ChildPipe::Tee(Box::new(Cursor::new(stderr))));
                        }
                        child.set_exit_code(code);
                        let stream = ByteStream::child(child, span);
                        Ok((PipelineData::ByteStream(stream, metadata), code != 0))
                    } else {
                        let stream = ByteStream::child(child, span);
                        Ok((PipelineData::ByteStream(stream, metadata), false))
                    }
                }
                Err(stream) => Ok((PipelineData::ByteStream(stream, metadata), false)),
            }
        } else {
            Ok((self, false))
        }
    }

    /// Try to convert Value from Value::Range to Value::List.
    /// This is useful to expand Value::Range into array notation, specifically when
    /// converting `to json` or `to nuon`.
    /// `1..3 | to XX -> [1,2,3]`
    pub fn try_expand_range(self) -> Result<PipelineData, ShellError> {
        match self {
            PipelineData::Value(v, metadata) => {
                let span = v.span();
                match v {
                    Value::Range { val, .. } => {
                        match *val {
                            Range::IntRange(range) => {
                                if range.is_unbounded() {
                                    return Err(ShellError::GenericError {
                                        error: "Cannot create range".into(),
                                        msg: "Unbounded ranges are not allowed when converting to this format".into(),
                                        span: Some(span),
                                        help: Some("Consider using ranges with valid start and end point.".into()),
                                        inner: vec![],
                                    });
                                }
                            }
                            Range::FloatRange(range) => {
                                if range.is_unbounded() {
                                    return Err(ShellError::GenericError {
                                        error: "Cannot create range".into(),
                                        msg: "Unbounded ranges are not allowed when converting to this format".into(),
                                        span: Some(span),
                                        help: Some("Consider using ranges with valid start and end point.".into()),
                                        inner: vec![],
                                    });
                                }
                            }
                        }
                        let range_values: Vec<Value> = val.into_range_iter(span, None).collect();
                        Ok(PipelineData::Value(Value::list(range_values, span), None))
                    }
                    x => Ok(PipelineData::Value(x, metadata)),
                }
            }
            _ => Ok(self),
        }
    }

    /// Consume and print self data immediately.
    ///
    /// `no_newline` controls if we need to attach newline character to output.
    /// `to_stderr` controls if data is output to stderr, when the value is false, the data is output to stdout.
    pub fn print(
        self,
        engine_state: &EngineState,
        stack: &mut Stack,
        no_newline: bool,
        to_stderr: bool,
    ) -> Result<Option<ExitStatus>, ShellError> {
        match self {
            // Print byte streams directly as long as they aren't binary.
            PipelineData::ByteStream(stream, ..) if stream.type_() != ByteStreamType::Binary => {
                stream.print(to_stderr)
            }
            _ => {
                // If the table function is in the declarations, then we can use it
                // to create the table value that will be printed in the terminal
                if let Some(decl_id) = engine_state.table_decl_id {
                    let command = engine_state.get_decl(decl_id);
                    if command.block_id().is_some() {
                        self.write_all_and_flush(engine_state, no_newline, to_stderr)
                    } else {
                        let call = Call::new(Span::new(0, 0));
                        let table = command.run(engine_state, stack, &call, self)?;
                        table.write_all_and_flush(engine_state, no_newline, to_stderr)
                    }
                } else {
                    self.write_all_and_flush(engine_state, no_newline, to_stderr)
                }
            }
        }
    }

    fn write_all_and_flush(
        self,
        engine_state: &EngineState,
        no_newline: bool,
        to_stderr: bool,
    ) -> Result<Option<ExitStatus>, ShellError> {
        if let PipelineData::ByteStream(stream, ..) = self {
            // Copy ByteStreams directly
            stream.print(to_stderr)
        } else {
            let config = engine_state.get_config();
            for item in self {
                let mut out = if let Value::Error { error, .. } = item {
                    return Err(*error);
                } else {
                    item.to_expanded_string("\n", config)
                };

                if !no_newline {
                    out.push('\n');
                }

                if to_stderr {
                    stderr_write_all_and_flush(out)?
                } else {
                    stdout_write_all_and_flush(out)?
                }
            }

            Ok(None)
        }
    }
}

enum PipelineIteratorInner {
    Empty,
    Value(Value),
    ListStream(crate::list_stream::IntoIter),
    ByteStream(crate::byte_stream::Chunks),
}

pub struct PipelineIterator(PipelineIteratorInner);

impl IntoIterator for PipelineData {
    type Item = Value;

    type IntoIter = PipelineIterator;

    fn into_iter(self) -> Self::IntoIter {
        PipelineIterator(match self {
            PipelineData::Empty => PipelineIteratorInner::Empty,
            PipelineData::Value(value, ..) => {
                let span = value.span();
                match value {
                    Value::List { vals, .. } => PipelineIteratorInner::ListStream(
                        ListStream::new(vals.into_iter(), span, None).into_iter(),
                    ),
                    Value::Range { val, .. } => PipelineIteratorInner::ListStream(
                        ListStream::new(val.into_range_iter(span, None), span, None).into_iter(),
                    ),
                    x => PipelineIteratorInner::Value(x),
                }
            }
            PipelineData::ListStream(stream, ..) => {
                PipelineIteratorInner::ListStream(stream.into_iter())
            }
            PipelineData::ByteStream(stream, ..) => stream.chunks().map_or(
                PipelineIteratorInner::Empty,
                PipelineIteratorInner::ByteStream,
            ),
        })
    }
}

impl Iterator for PipelineIterator {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            PipelineIteratorInner::Empty => None,
            PipelineIteratorInner::Value(Value::Nothing { .. }, ..) => None,
            PipelineIteratorInner::Value(v, ..) => Some(std::mem::take(v)),
            PipelineIteratorInner::ListStream(stream, ..) => stream.next(),
            PipelineIteratorInner::ByteStream(stream) => stream.next().map(|x| match x {
                Ok(x) => x,
                Err(err) => Value::error(
                    err,
                    Span::unknown(), //FIXME: unclear where this span should come from
                ),
            }),
        }
    }
}

pub trait IntoPipelineData {
    fn into_pipeline_data(self) -> PipelineData;

    fn into_pipeline_data_with_metadata(
        self,
        metadata: impl Into<Option<PipelineMetadata>>,
    ) -> PipelineData;
}

impl<V> IntoPipelineData for V
where
    V: Into<Value>,
{
    fn into_pipeline_data(self) -> PipelineData {
        PipelineData::Value(self.into(), None)
    }

    fn into_pipeline_data_with_metadata(
        self,
        metadata: impl Into<Option<PipelineMetadata>>,
    ) -> PipelineData {
        PipelineData::Value(self.into(), metadata.into())
    }
}

pub trait IntoInterruptiblePipelineData {
    fn into_pipeline_data(self, span: Span, ctrlc: Option<Arc<AtomicBool>>) -> PipelineData;
    fn into_pipeline_data_with_metadata(
        self,
        span: Span,
        ctrlc: Option<Arc<AtomicBool>>,
        metadata: impl Into<Option<PipelineMetadata>>,
    ) -> PipelineData;
}

impl<I> IntoInterruptiblePipelineData for I
where
    I: IntoIterator + Send + 'static,
    I::IntoIter: Send + 'static,
    <I::IntoIter as Iterator>::Item: Into<Value>,
{
    fn into_pipeline_data(self, span: Span, ctrlc: Option<Arc<AtomicBool>>) -> PipelineData {
        ListStream::new(self.into_iter().map(Into::into), span, ctrlc).into()
    }

    fn into_pipeline_data_with_metadata(
        self,
        span: Span,
        ctrlc: Option<Arc<AtomicBool>>,
        metadata: impl Into<Option<PipelineMetadata>>,
    ) -> PipelineData {
        PipelineData::ListStream(
            ListStream::new(self.into_iter().map(Into::into), span, ctrlc),
            metadata.into(),
        )
    }
}

fn value_to_bytes(value: Value) -> Result<Vec<u8>, ShellError> {
    let bytes = match value {
        Value::String { val, .. } => val.into_bytes(),
        Value::Binary { val, .. } => val,
        Value::List { vals, .. } => {
            let val = vals
                .into_iter()
                .map(Value::coerce_into_string)
                .collect::<Result<Vec<String>, ShellError>>()?
                .join("\n")
                + "\n";

            val.into_bytes()
        }
        // Propagate errors by explicitly matching them before the final case.
        Value::Error { error, .. } => return Err(*error),
        value => value.coerce_into_string()?.into_bytes(),
    };
    Ok(bytes)
}
