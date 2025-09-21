#[cfg(feature = "os")]
use crate::process::ExitStatusFuture;
use crate::{
    ByteStream, ByteStreamSource, ByteStreamType, Config, ListStream, OutDest, PipelineMetadata,
    Range, ShellError, Signals, Span, Type, Value,
    ast::{Call, PathMember},
    engine::{EngineState, Stack},
    location,
    shell_error::{io::IoError, location::Location},
};
use std::{
    borrow::Cow,
    io::Write,
    ops::Deref,
    sync::{Arc, Mutex},
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
///   Namely, how do you know the difference between a single string and a list of one string. How do you know
///   when to flatten the data given to you from a data source into the stream or to keep it as an unflattened
///   list?
///
/// * We tried putting the stream into Value. This had some interesting properties as now commands "just worked
///   on values", but lead to a few unfortunate issues.
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
///   them into any sources. Streams are still available to model the infinite streams approach of original
///   Nushell.
#[derive(Debug)]
pub enum PipelineData {
    Empty,
    Value(Value, Option<PipelineMetadata>),
    ListStream(ListStream, Option<PipelineMetadata>),
    ByteStream(ByteStream, Option<PipelineMetadata>),
}

impl PipelineData {
    pub const fn empty() -> PipelineData {
        PipelineData::Empty
    }

    pub fn value(val: Value, metadata: impl Into<Option<PipelineMetadata>>) -> Self {
        PipelineData::Value(val, metadata.into())
    }

    pub fn list_stream(stream: ListStream, metadata: impl Into<Option<PipelineMetadata>>) -> Self {
        PipelineData::ListStream(stream, metadata.into())
    }

    pub fn byte_stream(stream: ByteStream, metadata: impl Into<Option<PipelineMetadata>>) -> Self {
        PipelineData::ByteStream(stream, metadata.into())
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

    /// Change the span of the [`PipelineData`].
    ///
    /// Returns `Value(Nothing)` with the given span if it was [`PipelineData::empty()`].
    pub fn with_span(self, span: Span) -> Self {
        match self {
            PipelineData::Empty => PipelineData::value(Value::nothing(span), None),
            PipelineData::Value(value, metadata) => {
                PipelineData::value(value.with_span(span), metadata)
            }
            PipelineData::ListStream(stream, metadata) => {
                PipelineData::list_stream(stream.with_span(span), metadata)
            }
            PipelineData::ByteStream(stream, metadata) => {
                PipelineData::byte_stream(stream.with_span(span), metadata)
            }
        }
    }

    /// Get a type that is representative of the `PipelineData`.
    ///
    /// The type returned here makes no effort to collect a stream, so it may be a different type
    /// than would be returned by [`Value::get_type()`] on the result of
    /// [`.into_value()`](Self::into_value).
    ///
    /// Specifically, a `ListStream` results in `list<any>` rather than
    /// the fully complete [`list`](Type::List) type (which would require knowing the contents),
    /// and a `ByteStream` with [unknown](crate::ByteStreamType::Unknown) type results in
    /// [`any`](Type::Any) rather than [`string`](Type::String) or [`binary`](Type::Binary).
    pub fn get_type(&self) -> Type {
        match self {
            PipelineData::Empty => Type::Nothing,
            PipelineData::Value(value, _) => value.get_type(),
            PipelineData::ListStream(_, _) => Type::list(Type::Any),
            PipelineData::ByteStream(stream, _) => stream.type_().into(),
        }
    }

    /// Determine if the `PipelineData` is a [subtype](https://en.wikipedia.org/wiki/Subtyping) of `other`.
    ///
    /// This check makes no effort to collect a stream, so it may be a different result
    /// than would be returned by calling [`Value::is_subtype_of()`] on the result of
    /// [`.into_value()`](Self::into_value).
    ///
    /// A `ListStream` acts the same as an empty list type: it is a subtype of any [`list`](Type::List)
    /// or [`table`](Type::Table) type. After converting to a value, it may become a more specific type.
    /// For example, a `ListStream` is a subtype of `list<int>` and `list<string>`.
    /// If calling [`.into_value()`](Self::into_value) results in a `list<int>`,
    /// then the value would not be a subtype of `list<string>`, in contrast to the original `ListStream`.
    ///
    /// A `ByteStream` is a subtype of [`string`](Type::String) if it is coercible into a string.
    /// Likewise, a `ByteStream` is a subtype of [`binary`](Type::Binary) if it is coercible into a binary value.
    pub fn is_subtype_of(&self, other: &Type) -> bool {
        match (self, other) {
            (_, Type::Any) => true,
            (PipelineData::Empty, Type::Nothing) => true,
            (PipelineData::Value(val, ..), ty) => val.is_subtype_of(ty),

            // a list stream could be a list with any type, including a table
            (PipelineData::ListStream(..), Type::List(..) | Type::Table(..)) => true,

            (PipelineData::ByteStream(stream, ..), Type::String)
                if stream.type_().is_string_coercible() =>
            {
                true
            }
            (PipelineData::ByteStream(stream, ..), Type::Binary)
                if stream.type_().is_binary_coercible() =>
            {
                true
            }

            (PipelineData::Empty, _) => false,
            (PipelineData::ListStream(..), _) => false,
            (PipelineData::ByteStream(..), _) => false,
        }
    }

    pub fn into_value(self, span: Span) -> Result<Value, ShellError> {
        match self {
            PipelineData::Empty => Ok(Value::nothing(span)),
            PipelineData::Value(value, ..) => {
                if value.span() == Span::unknown() {
                    Ok(value.with_span(span))
                } else {
                    Ok(value)
                }
            }
            PipelineData::ListStream(stream, ..) => Ok(stream.into_value()),
            PipelineData::ByteStream(stream, ..) => stream.into_value(),
        }
    }

    /// Converts any `Value` variant that can be represented as a stream into its stream variant.
    ///
    /// This means that lists and ranges are converted into list streams, and strings and binary are
    /// converted into byte streams.
    ///
    /// Returns an `Err` with the original stream if the variant couldn't be converted to a stream
    /// variant. If the variant is already a stream variant, it is returned as-is.
    pub fn try_into_stream(self, engine_state: &EngineState) -> Result<PipelineData, PipelineData> {
        let span = self.span().unwrap_or(Span::unknown());
        match self {
            PipelineData::ListStream(..) | PipelineData::ByteStream(..) => Ok(self),
            PipelineData::Value(Value::List { .. } | Value::Range { .. }, ref metadata) => {
                let metadata = metadata.clone();
                Ok(PipelineData::list_stream(
                    ListStream::new(self.into_iter(), span, engine_state.signals().clone()),
                    metadata,
                ))
            }
            PipelineData::Value(Value::String { val, .. }, metadata) => {
                Ok(PipelineData::byte_stream(
                    ByteStream::read_string(val, span, engine_state.signals().clone()),
                    metadata,
                ))
            }
            PipelineData::Value(Value::Binary { val, .. }, metadata) => {
                Ok(PipelineData::byte_stream(
                    ByteStream::read_binary(val, span, engine_state.signals().clone()),
                    metadata,
                ))
            }
            _ => Err(self),
        }
    }

    /// Drain and write this [`PipelineData`] to `dest`.
    ///
    /// Values are converted to bytes and separated by newlines if this is a `ListStream`.
    pub fn write_to(self, mut dest: impl Write) -> Result<(), ShellError> {
        match self {
            PipelineData::Empty => Ok(()),
            PipelineData::Value(value, ..) => {
                let bytes = value_to_bytes(value)?;
                dest.write_all(&bytes).map_err(|err| {
                    IoError::new_internal(
                        err,
                        "Could not write PipelineData to dest",
                        crate::location!(),
                    )
                })?;
                dest.flush().map_err(|err| {
                    IoError::new_internal(
                        err,
                        "Could not flush PipelineData to dest",
                        crate::location!(),
                    )
                })?;
                Ok(())
            }
            PipelineData::ListStream(stream, ..) => {
                for value in stream {
                    let bytes = value_to_bytes(value)?;
                    dest.write_all(&bytes).map_err(|err| {
                        IoError::new_internal(
                            err,
                            "Could not write PipelineData to dest",
                            crate::location!(),
                        )
                    })?;
                    dest.write_all(b"\n").map_err(|err| {
                        IoError::new_internal(
                            err,
                            "Could not write linebreak after PipelineData to dest",
                            crate::location!(),
                        )
                    })?;
                }
                dest.flush().map_err(|err| {
                    IoError::new_internal(
                        err,
                        "Could not flush PipelineData to dest",
                        crate::location!(),
                    )
                })?;
                Ok(())
            }
            PipelineData::ByteStream(stream, ..) => stream.write_to(dest),
        }
    }

    /// Drain this [`PipelineData`] according to the current stdout [`OutDest`]s in `stack`.
    ///
    /// For [`OutDest::Pipe`] and [`OutDest::PipeSeparate`], this will return the [`PipelineData`]
    /// as is. For [`OutDest::Value`], this will collect into a value and return it. For
    /// [`OutDest::Print`], the [`PipelineData`] is drained and printed. Otherwise, the
    /// [`PipelineData`] is drained, but only printed if it is the output of an external command.
    pub fn drain_to_out_dests(
        self,
        engine_state: &EngineState,
        stack: &mut Stack,
    ) -> Result<Self, ShellError> {
        match stack.pipe_stdout().unwrap_or(&OutDest::Inherit) {
            OutDest::Print => {
                self.print_table(engine_state, stack, false, false)?;
                Ok(Self::Empty)
            }
            OutDest::Pipe | OutDest::PipeSeparate => Ok(self),
            OutDest::Value => {
                let metadata = self.metadata();
                let span = self.span().unwrap_or(Span::unknown());
                self.into_value(span).map(|val| Self::Value(val, metadata))
            }
            OutDest::File(file) => {
                self.write_to(file.as_ref())?;
                Ok(Self::Empty)
            }
            OutDest::Null | OutDest::Inherit => {
                self.drain()?;
                Ok(Self::Empty)
            }
        }
    }

    pub fn drain(self) -> Result<(), ShellError> {
        match self {
            Self::Empty => Ok(()),
            Self::Value(Value::Error { error, .. }, ..) => Err(*error),
            Self::Value(..) => Ok(()),
            Self::ListStream(stream, ..) => stream.drain(),
            Self::ByteStream(stream, ..) => stream.drain(),
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
                        ListStream::new(vals.into_iter(), val_span, Signals::empty()).into_iter(),
                    ),
                    Value::Binary { val, .. } => PipelineIteratorInner::ListStream(
                        ListStream::new(
                            val.into_iter().map(move |x| Value::int(x as i64, val_span)),
                            val_span,
                            Signals::empty(),
                        )
                        .into_iter(),
                    ),
                    Value::Range { val, .. } => PipelineIteratorInner::ListStream(
                        ListStream::new(
                            val.into_range_iter(val_span, Signals::empty()),
                            val_span,
                            Signals::empty(),
                        )
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
                        });
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
                });
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
    ) -> Result<Value, ShellError> {
        match self {
            // FIXME: there are probably better ways of doing this
            PipelineData::ListStream(stream, ..) => Value::list(stream.into_iter().collect(), head)
                .follow_cell_path(cell_path)
                .map(Cow::into_owned),
            PipelineData::Value(v, ..) => v.follow_cell_path(cell_path).map(Cow::into_owned),
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
    pub fn map<F>(self, mut f: F, signals: &Signals) -> Result<PipelineData, ShellError>
    where
        Self: Sized,
        F: FnMut(Value) -> Value + 'static + Send,
    {
        match self {
            PipelineData::Value(value, metadata) => {
                let span = value.span();
                let pipeline = match value {
                    Value::List { vals, .. } => vals
                        .into_iter()
                        .map(f)
                        .into_pipeline_data(span, signals.clone()),
                    Value::Range { val, .. } => val
                        .into_range_iter(span, Signals::empty())
                        .map(f)
                        .into_pipeline_data(span, signals.clone()),
                    value => match f(value) {
                        Value::Error { error, .. } => return Err(*error),
                        v => v.into_pipeline_data(),
                    },
                };
                Ok(pipeline.set_metadata(metadata))
            }
            PipelineData::Empty => Ok(PipelineData::empty()),
            PipelineData::ListStream(stream, metadata) => {
                Ok(PipelineData::list_stream(stream.map(f), metadata))
            }
            PipelineData::ByteStream(stream, metadata) => {
                Ok(f(stream.into_value()?).into_pipeline_data_with_metadata(metadata))
            }
        }
    }

    /// Simplified flatmapper. For full iterator support use `.into_iter()` instead
    pub fn flat_map<U, F>(self, mut f: F, signals: &Signals) -> Result<PipelineData, ShellError>
    where
        Self: Sized,
        U: IntoIterator<Item = Value> + 'static,
        <U as IntoIterator>::IntoIter: 'static + Send,
        F: FnMut(Value) -> U + 'static + Send,
    {
        match self {
            PipelineData::Empty => Ok(PipelineData::empty()),
            PipelineData::Value(value, metadata) => {
                let span = value.span();
                let pipeline = match value {
                    Value::List { vals, .. } => vals
                        .into_iter()
                        .flat_map(f)
                        .into_pipeline_data(span, signals.clone()),
                    Value::Range { val, .. } => val
                        .into_range_iter(span, Signals::empty())
                        .flat_map(f)
                        .into_pipeline_data(span, signals.clone()),
                    value => f(value)
                        .into_iter()
                        .into_pipeline_data(span, signals.clone()),
                };
                Ok(pipeline.set_metadata(metadata))
            }
            PipelineData::ListStream(stream, metadata) => Ok(PipelineData::list_stream(
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
                Ok(iter.into_iter().into_pipeline_data_with_metadata(
                    span,
                    signals.clone(),
                    metadata,
                ))
            }
        }
    }

    pub fn filter<F>(self, mut f: F, signals: &Signals) -> Result<PipelineData, ShellError>
    where
        Self: Sized,
        F: FnMut(&Value) -> bool + 'static + Send,
    {
        match self {
            PipelineData::Empty => Ok(PipelineData::empty()),
            PipelineData::Value(value, metadata) => {
                let span = value.span();
                let pipeline = match value {
                    Value::List { vals, .. } => vals
                        .into_iter()
                        .filter(f)
                        .into_pipeline_data(span, signals.clone()),
                    Value::Range { val, .. } => val
                        .into_range_iter(span, Signals::empty())
                        .filter(f)
                        .into_pipeline_data(span, signals.clone()),
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
            PipelineData::ListStream(stream, metadata) => Ok(PipelineData::list_stream(
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
                        let range_values: Vec<Value> =
                            val.into_range_iter(span, Signals::empty()).collect();
                        Ok(PipelineData::value(Value::list(range_values, span), None))
                    }
                    x => Ok(PipelineData::value(x, metadata)),
                }
            }
            _ => Ok(self),
        }
    }

    /// Consume and print self data immediately, formatted using table command.
    ///
    /// This does not respect the display_output hook. If a value is being printed out by a command,
    /// this function should be used. Otherwise, `nu_cli::util::print_pipeline` should be preferred.
    ///
    /// `no_newline` controls if we need to attach newline character to output.
    /// `to_stderr` controls if data is output to stderr, when the value is false, the data is output to stdout.
    pub fn print_table(
        self,
        engine_state: &EngineState,
        stack: &mut Stack,
        no_newline: bool,
        to_stderr: bool,
    ) -> Result<(), ShellError> {
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
                        let table = command.run(engine_state, stack, &(&call).into(), self)?;
                        table.write_all_and_flush(engine_state, no_newline, to_stderr)
                    }
                } else {
                    self.write_all_and_flush(engine_state, no_newline, to_stderr)
                }
            }
        }
    }

    /// Consume and print self data without any extra formatting.
    ///
    /// This does not use the `table` command to format data, and also prints binary values and
    /// streams in their raw format without generating a hexdump first.
    ///
    /// `no_newline` controls if we need to attach newline character to output.
    /// `to_stderr` controls if data is output to stderr, when the value is false, the data is output to stdout.
    pub fn print_raw(
        self,
        engine_state: &EngineState,
        no_newline: bool,
        to_stderr: bool,
    ) -> Result<(), ShellError> {
        let span = self.span();
        if let PipelineData::Value(Value::Binary { val: bytes, .. }, _) = self {
            if to_stderr {
                write_all_and_flush(
                    bytes,
                    &mut std::io::stderr().lock(),
                    "stderr",
                    span,
                    engine_state.signals(),
                )?;
            } else {
                write_all_and_flush(
                    bytes,
                    &mut std::io::stdout().lock(),
                    "stdout",
                    span,
                    engine_state.signals(),
                )?;
            }
            Ok(())
        } else {
            self.write_all_and_flush(engine_state, no_newline, to_stderr)
        }
    }

    fn write_all_and_flush(
        self,
        engine_state: &EngineState,
        no_newline: bool,
        to_stderr: bool,
    ) -> Result<(), ShellError> {
        let span = self.span();
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
                    write_all_and_flush(
                        out,
                        &mut std::io::stderr().lock(),
                        "stderr",
                        span,
                        engine_state.signals(),
                    )?;
                } else {
                    write_all_and_flush(
                        out,
                        &mut std::io::stdout().lock(),
                        "stdout",
                        span,
                        engine_state.signals(),
                    )?;
                }
            }

            Ok(())
        }
    }

    pub fn unsupported_input_error(
        self,
        expected_type: impl Into<String>,
        span: Span,
    ) -> ShellError {
        match self {
            PipelineData::Empty => ShellError::PipelineEmpty { dst_span: span },
            PipelineData::Value(value, ..) => ShellError::OnlySupportsThisInputType {
                exp_input_type: expected_type.into(),
                wrong_type: value.get_type().get_non_specified_string(),
                dst_span: span,
                src_span: value.span(),
            },
            PipelineData::ListStream(stream, ..) => ShellError::OnlySupportsThisInputType {
                exp_input_type: expected_type.into(),
                wrong_type: "list (stream)".into(),
                dst_span: span,
                src_span: stream.span(),
            },
            PipelineData::ByteStream(stream, ..) => ShellError::OnlySupportsThisInputType {
                exp_input_type: expected_type.into(),
                wrong_type: stream.type_().describe().into(),
                dst_span: span,
                src_span: stream.span(),
            },
        }
    }

    // PipelineData might connect to a running process which has an exit status future
    // Use this method to retrieve that future, it's useful for implementing `pipefail` feature.
    #[cfg(feature = "os")]
    pub fn clone_exit_status_future(&self) -> Option<Arc<Mutex<ExitStatusFuture>>> {
        match self {
            PipelineData::Empty | PipelineData::Value(..) | PipelineData::ListStream(..) => None,
            PipelineData::ByteStream(stream, ..) => match stream.source() {
                ByteStreamSource::Read(..) | ByteStreamSource::File(..) => None,
                ByteStreamSource::Child(c) => Some(c.clone_exit_status_future()),
            },
        }
    }
}

pub fn write_all_and_flush<T>(
    data: T,
    destination: &mut impl Write,
    destination_name: &str,
    span: Option<Span>,
    signals: &Signals,
) -> Result<(), ShellError>
where
    T: AsRef<[u8]>,
{
    let io_error_map = |err: std::io::Error, location: Location| {
        let context = format!("Writing to {destination_name} failed");
        match span {
            None => IoError::new_internal(err, context, location),
            Some(span) if span == Span::unknown() => IoError::new_internal(err, context, location),
            Some(span) => IoError::new_with_additional_context(err, span, None, context),
        }
    };

    let span = span.unwrap_or(Span::unknown());
    const OUTPUT_CHUNK_SIZE: usize = 8192;
    for chunk in data.as_ref().chunks(OUTPUT_CHUNK_SIZE) {
        signals.check(&span)?;
        destination
            .write_all(chunk)
            .map_err(|err| io_error_map(err, location!()))?;
    }
    destination
        .flush()
        .map_err(|err| io_error_map(err, location!()))?;
    Ok(())
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
                        ListStream::new(vals.into_iter(), span, Signals::empty()).into_iter(),
                    ),
                    Value::Range { val, .. } => PipelineIteratorInner::ListStream(
                        ListStream::new(
                            val.into_range_iter(span, Signals::empty()),
                            span,
                            Signals::empty(),
                        )
                        .into_iter(),
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
        PipelineData::value(self.into(), None)
    }

    fn into_pipeline_data_with_metadata(
        self,
        metadata: impl Into<Option<PipelineMetadata>>,
    ) -> PipelineData {
        PipelineData::value(self.into(), metadata.into())
    }
}

pub trait IntoInterruptiblePipelineData {
    fn into_pipeline_data(self, span: Span, signals: Signals) -> PipelineData;
    fn into_pipeline_data_with_metadata(
        self,
        span: Span,
        signals: Signals,
        metadata: impl Into<Option<PipelineMetadata>>,
    ) -> PipelineData;
}

impl<I> IntoInterruptiblePipelineData for I
where
    I: IntoIterator + Send + 'static,
    I::IntoIter: Send + 'static,
    <I::IntoIter as Iterator>::Item: Into<Value>,
{
    fn into_pipeline_data(self, span: Span, signals: Signals) -> PipelineData {
        ListStream::new(self.into_iter().map(Into::into), span, signals).into()
    }

    fn into_pipeline_data_with_metadata(
        self,
        span: Span,
        signals: Signals,
        metadata: impl Into<Option<PipelineMetadata>>,
    ) -> PipelineData {
        PipelineData::list_stream(
            ListStream::new(self.into_iter().map(Into::into), span, signals),
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

/// A wrapper to [`PipelineData`] which can also track exit status.
///
/// We use exit status tracking to implement the `pipefail` feature.
pub struct PipelineExecutionData {
    pub body: PipelineData,
    #[cfg(feature = "os")]
    pub exit: Vec<Option<(Arc<Mutex<ExitStatusFuture>>, Span)>>,
}

impl Deref for PipelineExecutionData {
    type Target = PipelineData;

    fn deref(&self) -> &Self::Target {
        &self.body
    }
}

impl PipelineExecutionData {
    pub fn empty() -> Self {
        Self {
            body: PipelineData::empty(),
            #[cfg(feature = "os")]
            exit: vec![],
        }
    }
}

impl From<PipelineData> for PipelineExecutionData {
    #[cfg(feature = "os")]
    fn from(value: PipelineData) -> Self {
        let value_span = value.span().unwrap_or_else(Span::unknown);
        let exit_status_future = value.clone_exit_status_future().map(|f| (f, value_span));
        Self {
            body: value,
            exit: vec![exit_status_future],
        }
    }

    #[cfg(not(feature = "os"))]
    fn from(value: PipelineData) -> Self {
        Self { body: value }
    }
}
