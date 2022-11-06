use crate::{
    ast::{Call, PathMember},
    engine::{EngineState, Stack, StateWorkingSet},
    format_error, Config, ListStream, RawStream, ShellError, Span, Value,
};
use nu_utils::{stderr_write_all_and_flush, stdout_write_all_and_flush};
use std::sync::{atomic::AtomicBool, Arc};

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
    },
}

#[derive(Debug, Clone)]
pub struct PipelineMetadata {
    pub data_source: DataSource,
}

#[derive(Debug, Clone)]
pub enum DataSource {
    Ls,
}

impl PipelineData {
    pub fn new(span: Span) -> PipelineData {
        PipelineData::Value(Value::Nothing { span }, None)
    }

    pub fn new_with_metadata(metadata: Option<PipelineMetadata>, span: Span) -> PipelineData {
        PipelineData::Value(Value::Nothing { span }, metadata)
    }

    pub fn metadata(&self) -> Option<PipelineMetadata> {
        match self {
            PipelineData::ListStream(_, x) => x.clone(),
            PipelineData::ExternalStream { metadata: x, .. } => x.clone(),
            PipelineData::Value(_, x) => x.clone(),
        }
    }

    pub fn set_metadata(mut self, metadata: Option<PipelineMetadata>) -> Self {
        match &mut self {
            PipelineData::ListStream(_, x) => *x = metadata,
            PipelineData::ExternalStream { metadata: x, .. } => *x = metadata,
            PipelineData::Value(_, x) => *x = metadata,
        }

        self
    }

    pub fn is_nothing(&self) -> bool {
        matches!(self, PipelineData::Value(Value::Nothing { .. }, ..))
    }

    pub fn into_value(self, span: Span) -> Value {
        match self {
            PipelineData::Value(Value::Nothing { .. }, ..) => Value::nothing(span),
            PipelineData::Value(v, ..) => v,
            PipelineData::ListStream(s, ..) => Value::List {
                vals: s.collect(),
                span, // FIXME?
            },
            PipelineData::ExternalStream {
                stdout: None,
                exit_code,
                ..
            } => {
                // Make sure everything has finished
                if let Some(exit_code) = exit_code {
                    let _: Vec<_> = exit_code.into_iter().collect();
                }
                Value::Nothing { span }
            }
            PipelineData::ExternalStream {
                stdout: Some(mut s),
                exit_code,
                ..
            } => {
                let mut items = vec![];

                for val in &mut s {
                    match val {
                        Ok(val) => {
                            items.push(val);
                        }
                        Err(e) => {
                            return Value::Error { error: e };
                        }
                    }
                }

                // Make sure everything has finished
                if let Some(exit_code) = exit_code {
                    let _: Vec<_> = exit_code.into_iter().collect();
                }

                if s.is_binary {
                    let mut output = vec![];
                    for item in items {
                        match item.as_binary() {
                            Ok(item) => {
                                output.extend(item);
                            }
                            Err(err) => {
                                return Value::Error { error: err };
                            }
                        }
                    }

                    Value::Binary {
                        val: output,
                        span, // FIXME?
                    }
                } else {
                    let mut output = String::new();
                    for item in items {
                        match item.as_string() {
                            Ok(s) => output.push_str(&s),
                            Err(err) => {
                                return Value::Error { error: err };
                            }
                        }
                    }
                    Value::String {
                        val: output,
                        span, // FIXME?
                    }
                }
            }
        }
    }

    pub fn into_interruptible_iter(self, ctrlc: Option<Arc<AtomicBool>>) -> PipelineIterator {
        let mut iter = self.into_iter();

        if let PipelineIterator(PipelineData::ListStream(s, ..)) = &mut iter {
            s.ctrlc = ctrlc;
        }

        iter
    }

    pub fn collect_string(self, separator: &str, config: &Config) -> Result<String, ShellError> {
        match self {
            PipelineData::Value(v, ..) => Ok(v.into_string(separator, config)),
            PipelineData::ListStream(s, ..) => Ok(s.into_string(separator, config)),
            PipelineData::ExternalStream { stdout: None, .. } => Ok(String::new()),
            PipelineData::ExternalStream {
                stdout: Some(s), ..
            } => {
                let mut output = String::new();

                for val in s {
                    match val {
                        Ok(val) => match val.as_string() {
                            Ok(s) => output.push_str(&s),
                            Err(err) => return Err(err),
                        },
                        Err(e) => return Err(e),
                    }
                }
                Ok(output)
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
            PipelineData::ListStream(stream, ..) => Value::List {
                vals: stream.collect(),
                span: head,
            }
            .follow_cell_path(cell_path, insensitive),
            PipelineData::Value(v, ..) => v.follow_cell_path(cell_path, insensitive),
            _ => Err(ShellError::IOError("can't follow stream paths".into())),
        }
    }

    pub fn upsert_cell_path(
        &mut self,
        cell_path: &[PathMember],
        callback: Box<dyn FnOnce(&Value) -> Value>,
        head: Span,
    ) -> Result<(), ShellError> {
        match self {
            // FIXME: there are probably better ways of doing this
            PipelineData::ListStream(stream, ..) => Value::List {
                vals: stream.collect(),
                span: head,
            }
            .upsert_cell_path(cell_path, callback),
            PipelineData::Value(v, ..) => v.upsert_cell_path(cell_path, callback),
            _ => Ok(()),
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
            PipelineData::Value(Value::List { vals, .. }, ..) => {
                Ok(vals.into_iter().map(f).into_pipeline_data(ctrlc))
            }
            PipelineData::ListStream(stream, ..) => Ok(stream.map(f).into_pipeline_data(ctrlc)),
            PipelineData::ExternalStream { stdout: None, .. } => {
                Ok(PipelineData::new(Span { start: 0, end: 0 }))
            }
            PipelineData::ExternalStream {
                stdout: Some(stream),
                ..
            } => {
                let collected = stream.into_bytes()?;

                if let Ok(st) = String::from_utf8(collected.clone().item) {
                    Ok(f(Value::String {
                        val: st,
                        span: collected.span,
                    })
                    .into_pipeline_data())
                } else {
                    Ok(f(Value::Binary {
                        val: collected.item,
                        span: collected.span,
                    })
                    .into_pipeline_data())
                }
            }

            PipelineData::Value(Value::Range { val, .. }, ..) => Ok(val
                .into_range_iter(ctrlc.clone())?
                .map(f)
                .into_pipeline_data(ctrlc)),
            PipelineData::Value(v, ..) => match f(v) {
                Value::Error { error } => Err(error),
                v => Ok(v.into_pipeline_data()),
            },
        }
    }

    /// Simplified flatmapper. For full iterator support use `.into_iter()` instead
    pub fn flat_map<U: 'static, F>(
        self,
        mut f: F,
        ctrlc: Option<Arc<AtomicBool>>,
    ) -> Result<PipelineData, ShellError>
    where
        Self: Sized,
        U: IntoIterator<Item = Value>,
        <U as IntoIterator>::IntoIter: 'static + Send,
        F: FnMut(Value) -> U + 'static + Send,
    {
        match self {
            PipelineData::Value(Value::List { vals, .. }, ..) => {
                Ok(vals.into_iter().flat_map(f).into_pipeline_data(ctrlc))
            }
            PipelineData::ListStream(stream, ..) => {
                Ok(stream.flat_map(f).into_pipeline_data(ctrlc))
            }
            PipelineData::ExternalStream { stdout: None, .. } => {
                Ok(PipelineData::new(Span { start: 0, end: 0 }))
            }
            PipelineData::ExternalStream {
                stdout: Some(stream),
                ..
            } => {
                let collected = stream.into_bytes()?;

                if let Ok(st) = String::from_utf8(collected.clone().item) {
                    Ok(f(Value::String {
                        val: st,
                        span: collected.span,
                    })
                    .into_iter()
                    .into_pipeline_data(ctrlc))
                } else {
                    Ok(f(Value::Binary {
                        val: collected.item,
                        span: collected.span,
                    })
                    .into_iter()
                    .into_pipeline_data(ctrlc))
                }
            }
            PipelineData::Value(Value::Range { val, .. }, ..) => {
                match val.into_range_iter(ctrlc.clone()) {
                    Ok(iter) => Ok(iter.flat_map(f).into_pipeline_data(ctrlc)),
                    Err(error) => Err(error),
                }
            }
            PipelineData::Value(v, ..) => Ok(f(v).into_iter().into_pipeline_data(ctrlc)),
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
            PipelineData::Value(Value::List { vals, .. }, ..) => {
                Ok(vals.into_iter().filter(f).into_pipeline_data(ctrlc))
            }
            PipelineData::ListStream(stream, ..) => Ok(stream.filter(f).into_pipeline_data(ctrlc)),
            PipelineData::ExternalStream { stdout: None, .. } => {
                Ok(PipelineData::new(Span { start: 0, end: 0 }))
            }
            PipelineData::ExternalStream {
                stdout: Some(stream),
                ..
            } => {
                let collected = stream.into_bytes()?;

                if let Ok(st) = String::from_utf8(collected.clone().item) {
                    let v = Value::String {
                        val: st,
                        span: collected.span,
                    };

                    if f(&v) {
                        Ok(v.into_pipeline_data())
                    } else {
                        Ok(PipelineData::new(collected.span))
                    }
                } else {
                    let v = Value::Binary {
                        val: collected.item,
                        span: collected.span,
                    };

                    if f(&v) {
                        Ok(v.into_pipeline_data())
                    } else {
                        Ok(PipelineData::new(collected.span))
                    }
                }
            }
            PipelineData::Value(Value::Range { val, .. }, ..) => Ok(val
                .into_range_iter(ctrlc.clone())?
                .filter(f)
                .into_pipeline_data(ctrlc)),
            PipelineData::Value(v, ..) => {
                if f(&v) {
                    Ok(v.into_pipeline_data())
                } else {
                    Ok(Value::Nothing { span: v.span()? }.into_pipeline_data())
                }
            }
        }
    }

    /// Consume and print self data immediately.
    ///
    /// `no_newline` controls if we need to attach newline character to output.
    /// `to_stderr` controls if data is output to stderr, when the value is false, the data is ouput to stdout.
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
        // let stdout = std::io::stdout();

        if let PipelineData::ExternalStream {
            stdout: stream,
            stderr: stderr_stream,
            exit_code,
            ..
        } = self
        {
            return print_if_stream(stream, stderr_stream, to_stderr, exit_code);
            /*
            if let Ok(exit_code) = print_if_stream(stream, stderr_stream, to_stderr, exit_code) {
                return Ok(exit_code);
            }
            return Ok(0);
            */
        }

        match engine_state.find_decl("table".as_bytes(), &[]) {
            Some(decl_id) => {
                let command = engine_state.get_decl(decl_id);
                if command.get_block_id().is_some() {
                    return self.write_all_and_flush(engine_state, config, no_newline, to_stderr);
                }

                let table = command.run(engine_state, stack, &Call::new(Span::new(0, 0)), self)?;

                table.write_all_and_flush(engine_state, config, no_newline, to_stderr)?;
            }
            None => {
                self.write_all_and_flush(engine_state, config, no_newline, to_stderr)?;
            }
        };

        Ok(0)
    }

    fn write_all_and_flush(
        self,
        engine_state: &EngineState,
        config: &Config,
        no_newline: bool,
        to_stderr: bool,
    ) -> Result<i64, ShellError> {
        for item in self {
            let mut out = if let Value::Error { error } = item {
                let working_set = StateWorkingSet::new(engine_state);

                format_error(&working_set, &error)
            } else if no_newline {
                item.into_string("", config)
            } else {
                item.into_string("\n", config)
            };

            if !no_newline {
                out.push('\n');
            }

            if !to_stderr {
                stdout_write_all_and_flush(out)?
            } else {
                stderr_write_all_and_flush(out)?
            }
        }

        Ok(0)
    }
}

pub struct PipelineIterator(PipelineData);

impl IntoIterator for PipelineData {
    type Item = Value;

    type IntoIter = PipelineIterator;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            PipelineData::Value(Value::List { vals, .. }, metadata) => {
                PipelineIterator(PipelineData::ListStream(
                    ListStream {
                        stream: Box::new(vals.into_iter()),
                        ctrlc: None,
                    },
                    metadata,
                ))
            }
            PipelineData::Value(Value::Range { val, .. }, metadata) => {
                match val.into_range_iter(None) {
                    Ok(iter) => PipelineIterator(PipelineData::ListStream(
                        ListStream {
                            stream: Box::new(iter),
                            ctrlc: None,
                        },
                        metadata,
                    )),
                    Err(error) => PipelineIterator(PipelineData::ListStream(
                        ListStream {
                            stream: Box::new(std::iter::once(Value::Error { error })),
                            ctrlc: None,
                        },
                        metadata,
                    )),
                }
            }
            x => PipelineIterator(x),
        }
    }
}

pub fn print_if_stream(
    stream: Option<RawStream>,
    stderr_stream: Option<RawStream>,
    to_stderr: bool,
    exit_code: Option<ListStream>,
) -> Result<i64, ShellError> {
    // NOTE: currently we don't need anything from stderr
    // so directly consumes `stderr_stream` to make sure that everything is done.
    std::thread::spawn(move || stderr_stream.map(|x| x.into_bytes()));
    if let Some(stream) = stream {
        for s in stream {
            let s_live = s?;
            let bin_output = s_live.as_binary()?;

            if !to_stderr {
                stdout_write_all_and_flush(bin_output)?
            } else {
                stderr_write_all_and_flush(bin_output)?
            }
        }
    }

    // Make sure everything has finished
    if let Some(exit_code) = exit_code {
        let mut exit_codes: Vec<_> = exit_code.into_iter().collect();
        return match exit_codes.pop() {
            #[cfg(unix)]
            Some(Value::Error { error }) => Err(error),
            Some(Value::Int { val, .. }) => Ok(val),
            _ => Ok(0),
        };
    }

    Ok(0)
}

impl Iterator for PipelineIterator {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            PipelineData::Value(Value::Nothing { .. }, ..) => None,
            PipelineData::Value(v, ..) => Some(std::mem::take(v)),
            PipelineData::ListStream(stream, ..) => stream.next(),
            PipelineData::ExternalStream { stdout: None, .. } => None,
            PipelineData::ExternalStream {
                stdout: Some(stream),
                ..
            } => stream.next().map(|x| match x {
                Ok(x) => x,
                Err(err) => Value::Error { error: err },
            }),
        }
    }
}

pub trait IntoPipelineData {
    fn into_pipeline_data(self) -> PipelineData;
}

impl<V> IntoPipelineData for V
where
    V: Into<Value>,
{
    fn into_pipeline_data(self) -> PipelineData {
        PipelineData::Value(self.into(), None)
    }
}

pub trait IntoInterruptiblePipelineData {
    fn into_pipeline_data(self, ctrlc: Option<Arc<AtomicBool>>) -> PipelineData;
    fn into_pipeline_data_with_metadata(
        self,
        metadata: PipelineMetadata,
        ctrlc: Option<Arc<AtomicBool>>,
    ) -> PipelineData;
}

impl<I> IntoInterruptiblePipelineData for I
where
    I: IntoIterator + Send + 'static,
    I::IntoIter: Send + 'static,
    <I::IntoIter as Iterator>::Item: Into<Value>,
{
    fn into_pipeline_data(self, ctrlc: Option<Arc<AtomicBool>>) -> PipelineData {
        PipelineData::ListStream(
            ListStream {
                stream: Box::new(self.into_iter().map(Into::into)),
                ctrlc,
            },
            None,
        )
    }

    fn into_pipeline_data_with_metadata(
        self,
        metadata: PipelineMetadata,
        ctrlc: Option<Arc<AtomicBool>>,
    ) -> PipelineData {
        PipelineData::ListStream(
            ListStream {
                stream: Box::new(self.into_iter().map(Into::into)),
                ctrlc,
            },
            Some(metadata),
        )
    }
}
