pub mod list_stream;
mod metadata;
mod out_dest;
mod raw_stream;

pub use list_stream::{ListStream, ValueIterator};
pub use metadata::*;
pub use out_dest::*;
pub use raw_stream::*;

use crate::{
    ast::{Call, PathMember},
    engine::{EngineState, Stack, StateWorkingSet},
    format_error, Config, Range, ShellError, Span, Value,
};
use nu_utils::{stderr_write_all_and_flush, stdout_write_all_and_flush};
use std::{
    io::Write,
    sync::{atomic::AtomicBool, Arc},
    thread,
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
    Value(Value, Option<PipelineMetadata>),
    ListStream(ListStream, Option<PipelineMetadata>),
    ExternalStream {
        stdout: Option<RawStream>,
        stderr: Option<RawStream>,
        exit_code: Option<ListStream>,
        span: Span,
        metadata: Option<PipelineMetadata>,
        trim_end_newline: bool,
    },
    Empty,
}

impl PipelineData {
    pub fn new_with_metadata(metadata: Option<PipelineMetadata>, span: Span) -> PipelineData {
        PipelineData::Value(Value::nothing(span), metadata)
    }

    /// create a `PipelineData::ExternalStream` with proper exit_code
    ///
    /// It's useful to break running without raising error at user level.
    pub fn new_external_stream_with_only_exit_code(exit_code: i64) -> PipelineData {
        PipelineData::ExternalStream {
            stdout: None,
            stderr: None,
            exit_code: Some(ListStream::new(
                [Value::int(exit_code, Span::unknown())].into_iter(),
                Span::unknown(),
                None,
            )),
            span: Span::unknown(),
            metadata: None,
            trim_end_newline: false,
        }
    }

    pub fn empty() -> PipelineData {
        PipelineData::Empty
    }

    pub fn metadata(&self) -> Option<PipelineMetadata> {
        match self {
            PipelineData::ListStream(_, x) => x.clone(),
            PipelineData::ExternalStream { metadata: x, .. } => x.clone(),
            PipelineData::Value(_, x) => x.clone(),
            PipelineData::Empty => None,
        }
    }

    pub fn set_metadata(mut self, metadata: Option<PipelineMetadata>) -> Self {
        match &mut self {
            PipelineData::ListStream(_, x) => *x = metadata,
            PipelineData::ExternalStream { metadata: x, .. } => *x = metadata,
            PipelineData::Value(_, x) => *x = metadata,
            PipelineData::Empty => {}
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
            PipelineData::ListStream(stream, ..) => Some(stream.span()),
            PipelineData::ExternalStream { span, .. } => Some(*span),
            PipelineData::Value(v, _) => Some(v.span()),
            PipelineData::Empty => None,
        }
    }

    pub fn into_value(self, span: Span) -> Value {
        match self {
            PipelineData::Empty => Value::nothing(span),
            PipelineData::Value(Value::Nothing { .. }, ..) => Value::nothing(span),
            PipelineData::Value(v, ..) => v.with_span(span),
            PipelineData::ListStream(s, ..) => Value::list(
                s.into_iter().collect(),
                span, // FIXME?
            ),
            PipelineData::ExternalStream {
                stdout: None,
                exit_code,
                ..
            } => {
                // Make sure everything has finished
                if let Some(exit_code) = exit_code {
                    let _: Vec<_> = exit_code.into_iter().collect();
                }
                Value::nothing(span)
            }
            PipelineData::ExternalStream {
                stdout: Some(mut s),
                exit_code,
                trim_end_newline,
                ..
            } => {
                let mut items = vec![];

                for val in &mut s {
                    match val {
                        Ok(val) => {
                            items.push(val);
                        }
                        Err(e) => {
                            return Value::error(e, span);
                        }
                    }
                }

                // Make sure everything has finished
                if let Some(exit_code) = exit_code {
                    let _: Vec<_> = exit_code.into_iter().collect();
                }

                // NOTE: currently trim-end-newline only handles for string output.
                // For binary, user might need origin data.
                if s.is_binary {
                    let mut output = vec![];
                    for item in items {
                        match item.coerce_into_binary() {
                            Ok(item) => {
                                output.extend(item);
                            }
                            Err(err) => {
                                return Value::error(err, span);
                            }
                        }
                    }

                    Value::binary(
                        output, span, // FIXME?
                    )
                } else {
                    let mut output = String::new();
                    for item in items {
                        match item.coerce_into_string() {
                            Ok(s) => output.push_str(&s),
                            Err(err) => {
                                return Value::error(err, span);
                            }
                        }
                    }
                    if trim_end_newline {
                        output.truncate(output.trim_end_matches(LINE_ENDING_PATTERN).len())
                    }
                    Value::string(
                        output, span, // FIXME?
                    )
                }
            }
        }
    }

    pub fn drain(self) -> Result<(), ShellError> {
        match self {
            PipelineData::Value(Value::Error { error, .. }, _) => Err(*error),
            PipelineData::Value(_, _) => Ok(()),
            PipelineData::ListStream(stream, _) => stream.drain(),
            PipelineData::ExternalStream { stdout, stderr, .. } => {
                if let Some(stdout) = stdout {
                    stdout.drain()?;
                }

                if let Some(stderr) = stderr {
                    stderr.drain()?;
                }

                Ok(())
            }
            PipelineData::Empty => Ok(()),
        }
    }

    pub fn drain_with_exit_code(self) -> Result<i64, ShellError> {
        match self {
            PipelineData::Value(Value::Error { error, .. }, _) => Err(*error),
            PipelineData::Value(_, _) => Ok(0),
            PipelineData::ListStream(stream, _) => {
                stream.drain()?;
                Ok(0)
            }
            PipelineData::ExternalStream {
                stdout,
                stderr,
                exit_code,
                ..
            } => {
                if let Some(stdout) = stdout {
                    stdout.drain()?;
                }

                if let Some(stderr) = stderr {
                    stderr.drain()?;
                }

                if let Some(exit_code) = exit_code {
                    let result = drain_exit_code(exit_code)?;
                    Ok(result)
                } else {
                    Ok(0)
                }
            }
            PipelineData::Empty => Ok(0),
        }
    }

    /// Try convert from self into iterator
    ///
    /// It returns Err if the `self` cannot be converted to an iterator.
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
                        ListStream::new(val.into_range_iter(value.span(), None), val_span, None)
                            .into_iter(),
                    ),
                    // Propagate errors by explicitly matching them before the final case.
                    Value::Error { error, .. } => return Err(*error),
                    other => {
                        return Err(ShellError::OnlySupportsThisInputType {
                            exp_input_type: "list, binary, raw data or range".into(),
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
                    exp_input_type: "list, binary, raw data or range".into(),
                    wrong_type: "null".into(),
                    dst_span: span,
                    src_span: span,
                })
            }
            PipelineData::ExternalStream {
                stdout: Some(stdout),
                ..
            } => PipelineIteratorInner::ExternalStream(stdout),
            PipelineData::ExternalStream { stdout: None, .. } => PipelineIteratorInner::Empty,
        }))
    }

    pub fn collect_string(self, separator: &str, config: &Config) -> Result<String, ShellError> {
        match self {
            PipelineData::Empty => Ok(String::new()),
            PipelineData::Value(v, ..) => Ok(v.to_expanded_string(separator, config)),
            PipelineData::ListStream(s, ..) => Ok(s.into_string(separator, config)),
            PipelineData::ExternalStream { stdout: None, .. } => Ok(String::new()),
            PipelineData::ExternalStream {
                stdout: Some(s),
                trim_end_newline,
                ..
            } => {
                let mut output = String::new();

                for val in s {
                    output.push_str(&val?.coerce_into_string()?);
                }
                if trim_end_newline {
                    output.truncate(output.trim_end_matches(LINE_ENDING_PATTERN).len());
                }
                Ok(output)
            }
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
            PipelineData::Value(val, _) => Err(ShellError::TypeMismatch {
                err_message: "string".into(),
                span: val.span(),
            }),
            PipelineData::ListStream(_, _) => Err(ShellError::TypeMismatch {
                err_message: "string".into(),
                span,
            }),
            PipelineData::ExternalStream {
                stdout: None,
                metadata,
                span,
                ..
            } => Ok((String::new(), span, metadata)),
            PipelineData::ExternalStream {
                stdout: Some(stdout),
                metadata,
                span,
                ..
            } => Ok((stdout.into_string()?.item, span, metadata)),
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
            PipelineData::ExternalStream { span, .. } => Err(ShellError::IncompatiblePathAccess {
                type_name: "external stream".to_string(),
                span,
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
            PipelineData::Value(value, ..) => {
                let span = value.span();
                match value {
                    Value::List { vals, .. } => {
                        Ok(vals.into_iter().map(f).into_pipeline_data(span, ctrlc))
                    }
                    Value::Range { val, .. } => Ok(val
                        .into_range_iter(span, ctrlc.clone())
                        .map(f)
                        .into_pipeline_data(span, ctrlc)),
                    value => match f(value) {
                        Value::Error { error, .. } => Err(*error),
                        v => Ok(v.into_pipeline_data()),
                    },
                }
            }
            PipelineData::Empty => Ok(PipelineData::Empty),
            PipelineData::ListStream(stream, ..) => {
                Ok(PipelineData::ListStream(stream.map(f), None))
            }
            PipelineData::ExternalStream { stdout: None, .. } => Ok(PipelineData::empty()),
            PipelineData::ExternalStream {
                stdout: Some(stream),
                trim_end_newline,
                ..
            } => {
                let collected = stream.into_bytes()?;

                if let Ok(mut st) = String::from_utf8(collected.clone().item) {
                    if trim_end_newline {
                        st.truncate(st.trim_end_matches(LINE_ENDING_PATTERN).len());
                    }
                    Ok(f(Value::string(st, collected.span)).into_pipeline_data())
                } else {
                    Ok(f(Value::binary(collected.item, collected.span)).into_pipeline_data())
                }
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
            PipelineData::Value(value, ..) => {
                let span = value.span();
                match value {
                    Value::List { vals, .. } => {
                        Ok(vals.into_iter().flat_map(f).into_pipeline_data(span, ctrlc))
                    }
                    Value::Range { val, .. } => Ok(val
                        .into_range_iter(span, ctrlc.clone())
                        .flat_map(f)
                        .into_pipeline_data(span, ctrlc)),
                    value => Ok(f(value).into_iter().into_pipeline_data(span, ctrlc)),
                }
            }
            PipelineData::ListStream(stream, ..) => {
                Ok(stream.modify(|iter| iter.flat_map(f)).into())
            }
            PipelineData::ExternalStream { stdout: None, .. } => Ok(PipelineData::Empty),
            PipelineData::ExternalStream {
                stdout: Some(stream),
                span,
                trim_end_newline,
                ..
            } => {
                let collected = stream.into_bytes()?;

                if let Ok(mut st) = String::from_utf8(collected.clone().item) {
                    if trim_end_newline {
                        st.truncate(st.trim_end_matches(LINE_ENDING_PATTERN).len())
                    }
                    Ok(f(Value::string(st, collected.span))
                        .into_iter()
                        .into_pipeline_data(span, ctrlc))
                } else {
                    Ok(f(Value::binary(collected.item, collected.span))
                        .into_iter()
                        .into_pipeline_data(span, ctrlc))
                }
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
            PipelineData::Value(value, ..) => {
                let span = value.span();
                match value {
                    Value::List { vals, .. } => {
                        Ok(vals.into_iter().filter(f).into_pipeline_data(span, ctrlc))
                    }
                    Value::Range { val, .. } => Ok(val
                        .into_range_iter(span, ctrlc.clone())
                        .filter(f)
                        .into_pipeline_data(span, ctrlc)),
                    value => {
                        if f(&value) {
                            Ok(value.into_pipeline_data())
                        } else {
                            Ok(Value::nothing(span).into_pipeline_data())
                        }
                    }
                }
            }
            PipelineData::ListStream(stream, ..) => Ok(stream.modify(|iter| iter.filter(f)).into()),
            PipelineData::ExternalStream { stdout: None, .. } => Ok(PipelineData::Empty),
            PipelineData::ExternalStream {
                stdout: Some(stream),
                trim_end_newline,
                ..
            } => {
                let collected = stream.into_bytes()?;

                if let Ok(mut st) = String::from_utf8(collected.clone().item) {
                    if trim_end_newline {
                        st.truncate(st.trim_end_matches(LINE_ENDING_PATTERN).len())
                    }
                    let v = Value::string(st, collected.span);

                    if f(&v) {
                        Ok(v.into_pipeline_data())
                    } else {
                        Ok(PipelineData::new_with_metadata(None, collected.span))
                    }
                } else {
                    let v = Value::binary(collected.item, collected.span);

                    if f(&v) {
                        Ok(v.into_pipeline_data())
                    } else {
                        Ok(PipelineData::new_with_metadata(None, collected.span))
                    }
                }
            }
        }
    }

    /// Try to catch the external stream exit status and detect if it failed.
    ///
    /// This is useful for external commands with semicolon, we can detect errors early to avoid
    /// commands after the semicolon running.
    ///
    /// Returns `self` and a flag that indicates if the external stream run failed. If `self` is
    /// not [`PipelineData::ExternalStream`], the flag will be `false`.
    ///
    /// Currently this will consume an external stream to completion.
    pub fn check_external_failed(self) -> (Self, bool) {
        let mut failed_to_run = false;
        // Only need ExternalStream without redirecting output.
        // It indicates we have no more commands to execute currently.
        if let PipelineData::ExternalStream {
            stdout: None,
            stderr,
            mut exit_code,
            span,
            metadata,
            trim_end_newline,
        } = self
        {
            let exit_code = exit_code.take();

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
            let stderr = stderr.map(|stderr_stream| {
                let stderr_ctrlc = stderr_stream.ctrlc.clone();
                let stderr_span = stderr_stream.span;
                let stderr_bytes = stderr_stream
                    .into_bytes()
                    .map(|bytes| bytes.item)
                    .unwrap_or_default();
                RawStream::new(
                    Box::new(std::iter::once(Ok(stderr_bytes))),
                    stderr_ctrlc,
                    stderr_span,
                    None,
                )
            });

            match exit_code {
                Some(exit_code_stream) => {
                    let exit_code: Vec<Value> = exit_code_stream.into_iter().collect();
                    if let Some(Value::Int { val: code, .. }) = exit_code.last() {
                        // if exit_code is not 0, it indicates error occurred, return back Err.
                        if *code != 0 {
                            failed_to_run = true;
                        }
                    }
                    (
                        PipelineData::ExternalStream {
                            stdout: None,
                            stderr,
                            exit_code: Some(ListStream::new(exit_code.into_iter(), span, None)),
                            span,
                            metadata,
                            trim_end_newline,
                        },
                        failed_to_run,
                    )
                }
                None => (
                    PipelineData::ExternalStream {
                        stdout: None,
                        stderr,
                        exit_code: None,
                        span,
                        metadata,
                        trim_end_newline,
                    },
                    failed_to_run,
                ),
            }
        } else {
            (self, false)
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
                        match val {
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
    ) -> Result<i64, ShellError> {
        // If the table function is in the declarations, then we can use it
        // to create the table value that will be printed in the terminal

        let config = engine_state.get_config();

        if let PipelineData::ExternalStream {
            stdout: stream,
            stderr: stderr_stream,
            exit_code,
            ..
        } = self
        {
            return print_if_stream(stream, stderr_stream, to_stderr, exit_code);
        }

        if let Some(decl_id) = engine_state.table_decl_id {
            let command = engine_state.get_decl(decl_id);
            if command.get_block_id().is_some() {
                return self.write_all_and_flush(engine_state, config, no_newline, to_stderr);
            }

            let call = Call::new(Span::new(0, 0));
            let table = command.run(engine_state, stack, &call, self)?;
            table.write_all_and_flush(engine_state, config, no_newline, to_stderr)?;
        } else {
            self.write_all_and_flush(engine_state, config, no_newline, to_stderr)?;
        };

        Ok(0)
    }

    /// Consume and print self data immediately.
    ///
    /// Unlike [`.print()`] does not call `table` to format data and just prints it
    /// one element on a line
    /// * `no_newline` controls if we need to attach newline character to output.
    /// * `to_stderr` controls if data is output to stderr, when the value is false, the data is output to stdout.
    pub fn print_not_formatted(
        self,
        engine_state: &EngineState,
        no_newline: bool,
        to_stderr: bool,
    ) -> Result<i64, ShellError> {
        if let PipelineData::ExternalStream {
            stdout: stream,
            stderr: stderr_stream,
            exit_code,
            ..
        } = self
        {
            print_if_stream(stream, stderr_stream, to_stderr, exit_code)
        } else {
            let config = engine_state.get_config();
            self.write_all_and_flush(engine_state, config, no_newline, to_stderr)
        }
    }

    fn write_all_and_flush(
        self,
        engine_state: &EngineState,
        config: &Config,
        no_newline: bool,
        to_stderr: bool,
    ) -> Result<i64, ShellError> {
        for item in self {
            let mut is_err = false;
            let mut out = if let Value::Error { error, .. } = item {
                let working_set = StateWorkingSet::new(engine_state);
                // Value::Errors must always go to stderr, not stdout.
                is_err = true;
                format_error(&working_set, &*error)
            } else if no_newline {
                item.to_expanded_string("", config)
            } else {
                item.to_expanded_string("\n", config)
            };

            if !no_newline {
                out.push('\n');
            }

            if !to_stderr && !is_err {
                stdout_write_all_and_flush(out)?
            } else {
                stderr_write_all_and_flush(out)?
            }
        }

        Ok(0)
    }
}

enum PipelineIteratorInner {
    Empty,
    Value(Value),
    ListStream(list_stream::IntoIter),
    ExternalStream(RawStream),
}

pub struct PipelineIterator(PipelineIteratorInner);

impl IntoIterator for PipelineData {
    type Item = Value;

    type IntoIter = PipelineIterator;

    fn into_iter(self) -> Self::IntoIter {
        PipelineIterator(match self {
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
            PipelineData::ExternalStream {
                stdout: Some(stdout),
                ..
            } => PipelineIteratorInner::ExternalStream(stdout),
            PipelineData::ExternalStream { stdout: None, .. } => PipelineIteratorInner::Empty,
            PipelineData::Empty => PipelineIteratorInner::Empty,
        })
    }
}

pub fn print_if_stream(
    stream: Option<RawStream>,
    stderr_stream: Option<RawStream>,
    to_stderr: bool,
    exit_code: Option<ListStream>,
) -> Result<i64, ShellError> {
    if let Some(stderr_stream) = stderr_stream {
        thread::Builder::new()
            .name("stderr consumer".to_string())
            .spawn(move || {
                let RawStream {
                    stream,
                    leftover,
                    ctrlc,
                    ..
                } = stderr_stream;
                let mut stderr = std::io::stderr();
                let _ = stderr.write_all(&leftover);
                drop(leftover);
                for bytes in stream {
                    if nu_utils::ctrl_c::was_pressed(&ctrlc) {
                        break;
                    }
                    match bytes {
                        Ok(bytes) => {
                            let _ = stderr.write_all(&bytes);
                        }
                        Err(err) => {
                            // we don't have access to EngineState, but maybe logging the debug
                            // impl is better than nothing
                            eprintln!("Error in stderr stream: {err:?}");
                            break;
                        }
                    }
                }
            })?;
    }

    if let Some(stream) = stream {
        for s in stream {
            let s_live = s?;
            let bin_output = s_live.coerce_into_binary()?;

            if !to_stderr {
                stdout_write_all_and_flush(&bin_output)?
            } else {
                stderr_write_all_and_flush(&bin_output)?
            }
        }
    }

    // Make sure everything has finished
    if let Some(exit_code) = exit_code {
        return drain_exit_code(exit_code);
    }

    Ok(0)
}

fn drain_exit_code(exit_code: ListStream) -> Result<i64, ShellError> {
    let mut exit_codes: Vec<_> = exit_code.into_iter().collect();
    match exit_codes.pop() {
        #[cfg(unix)]
        Some(Value::Error { error, .. }) => Err(*error),
        Some(Value::Int { val, .. }) => Ok(val),
        _ => Ok(0),
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
            PipelineIteratorInner::ExternalStream(stream) => stream.next().map(|x| match x {
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
