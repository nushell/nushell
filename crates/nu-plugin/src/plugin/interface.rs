//! Implements the stream multiplexing interface for both the plugin side and the engine side.

use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};

use nu_protocol::{ListStream, PipelineData, RawStream, ShellError, Span, Value};

use crate::{
    plugin::PluginEncoder,
    protocol::{
        CallInput, ExternalStreamInfo, PluginCustomValue, PluginData, PluginInput, PluginOutput,
        RawStreamInfo,
    },
};

mod stream_data_io;
use stream_data_io::*;

mod engine;
pub use engine::EngineInterface;

mod plugin;
pub(crate) use plugin::{PluginExecutionContext, PluginExecutionNushellContext, PluginInterface};

#[cfg(test)]
mod test_util;

/// Read [PluginInput] or [PluginOutput] from the stream.
///
/// This abstraction is really only used to make testing easier; in general this will usually be
/// used on a pair of a [reader](std::io::BufRead) and an [encoder](PluginEncoder).
trait PluginRead: Send {
    /// Returns `Ok(None)` on end of stream.
    fn read_input(&mut self) -> Result<Option<PluginInput>, ShellError>;

    /// Returns `Ok(None)` on end of stream.
    fn read_output(&mut self) -> Result<Option<PluginOutput>, ShellError>;
}

impl<R, E> PluginRead for (R, E)
where
    R: std::io::BufRead + Send,
    E: PluginEncoder,
{
    fn read_input(&mut self) -> Result<Option<PluginInput>, ShellError> {
        self.1.decode_input(&mut self.0)
    }

    fn read_output(&mut self) -> Result<Option<PluginOutput>, ShellError> {
        self.1.decode_output(&mut self.0)
    }
}

/// Write [PluginInput] or [PluginOutput] to the stream.
///
/// This abstraction is really only used to make testing easier; in general this will usually be
/// used on a pair of a [writer](std:::io::Write) and an [encoder](PluginEncoder).
trait PluginWrite: Send {
    fn write_input(&mut self, input: &PluginInput) -> Result<(), ShellError>;
    fn write_output(&mut self, output: &PluginOutput) -> Result<(), ShellError>;

    /// Flush any internal buffers, if applicable.
    fn flush(&mut self) -> Result<(), ShellError>;
}

impl<W, E> PluginWrite for (W, E)
where
    W: std::io::Write + Send,
    E: PluginEncoder,
{
    fn write_input(&mut self, input: &PluginInput) -> Result<(), ShellError> {
        self.1.encode_input(input, &mut self.0)
    }

    fn write_output(&mut self, output: &PluginOutput) -> Result<(), ShellError> {
        self.1.encode_output(output, &mut self.0)
    }

    fn flush(&mut self) -> Result<(), ShellError> {
        self.0.flush().map_err(|err| ShellError::IOError {
            msg: err.to_string(),
        })
    }
}

/// Iterate through values received on a `ListStream` input.
///
/// Non-fused iterator: should generally call .fuse() when using it, to ensure messages aren't
/// attempted to be read after end-of-input.
struct PluginListStream {
    io: Arc<dyn StreamDataIo>,
}

impl Iterator for PluginListStream {
    type Item = Value;

    fn next(&mut self) -> Option<Value> {
        match self.io.read_list() {
            Ok(value) => value,
            Err(err) => Some(Value::error(err, Span::unknown())),
        }
    }
}

impl Drop for PluginListStream {
    fn drop(&mut self) {
        // Signal that we don't need the stream anymore.
        self.io.drop_list();
    }
}

/// Create [PipelineData] for receiving a [ListStream] input.
fn make_list_stream(source: Arc<dyn StreamDataIo>, ctrlc: Option<Arc<AtomicBool>>) -> PipelineData {
    PipelineData::ListStream(
        ListStream::from_stream(PluginListStream { io: source }.fuse(), ctrlc),
        None,
    )
}

/// Write the contents of a [ListStream] to `io`.
fn write_full_list_stream(
    io: &Arc<dyn StreamDataIo>,
    list_stream: ListStream,
) -> Result<(), ShellError> {
    // Consume the stream and write it via StreamDataIo.
    for value in list_stream {
        io.write_list(Some(match value {
            Value::LazyRecord { val, .. } => val.collect()?,
            _ => value,
        }))?;
    }
    // End of stream
    io.write_list(None)
}

/// Iterate through byte chunks received on the `stdout` stream of an `ListStream` input.
///
/// Non-fused iterator: should generally call .fuse() when using it, to ensure messages aren't
/// attempted to be read after end-of-input.
struct PluginExternalStdoutStream {
    io: Arc<dyn StreamDataIo>,
}

impl Iterator for PluginExternalStdoutStream {
    type Item = Result<Vec<u8>, ShellError>;

    fn next(&mut self) -> Option<Result<Vec<u8>, ShellError>> {
        self.io.read_external_stdout().transpose()
    }
}

impl Drop for PluginExternalStdoutStream {
    fn drop(&mut self) {
        // Signal that we don't need the stream anymore.
        self.io.drop_external_stdout();
    }
}

/// Iterate through byte chunks received on the `stderr` stream of an `ListStream` input.
///
/// Non-fused iterator: should generally call .fuse() when using it, to ensure messages aren't
/// attempted to be read after end-of-input.
struct PluginExternalStderrStream {
    io: Arc<dyn StreamDataIo>,
}

impl Iterator for PluginExternalStderrStream {
    type Item = Result<Vec<u8>, ShellError>;

    fn next(&mut self) -> Option<Result<Vec<u8>, ShellError>> {
        self.io.read_external_stderr().transpose()
    }
}

impl Drop for PluginExternalStderrStream {
    fn drop(&mut self) {
        // Signal that we don't need the stream anymore.
        self.io.drop_external_stderr();
    }
}

/// Iterate through values received on the `exit_code` stream of an `ListStream` input.
///
/// Non-fused iterator: should generally call .fuse() when using it, to ensure messages aren't
/// attempted to be read after end-of-input.
struct PluginExternalExitCodeStream {
    io: Arc<dyn StreamDataIo>,
}

impl Iterator for PluginExternalExitCodeStream {
    type Item = Value;

    fn next(&mut self) -> Option<Value> {
        match self.io.read_external_exit_code() {
            Ok(value) => value,
            Err(err) => Some(Value::error(err, Span::unknown())),
        }
    }
}

impl Drop for PluginExternalExitCodeStream {
    fn drop(&mut self) {
        // Signal that we don't need the stream anymore.
        self.io.drop_external_exit_code();
    }
}

/// Create [PipelineData] for receiving an [ExternalStream] input.
fn make_external_stream(
    source: Arc<dyn StreamDataIo>,
    info: &ExternalStreamInfo,
    ctrlc: Option<Arc<AtomicBool>>,
) -> PipelineData {
    PipelineData::ExternalStream {
        stdout: info.stdout.as_ref().map(|stdout_info| {
            let stream = PluginExternalStdoutStream { io: source.clone() }.fuse();
            let mut raw = RawStream::new(
                Box::new(stream),
                ctrlc.clone(),
                info.span,
                stdout_info.known_size,
            );
            raw.is_binary = stdout_info.is_binary;
            raw
        }),
        stderr: info.stderr.as_ref().map(|stderr_info| {
            let stream = PluginExternalStderrStream { io: source.clone() }.fuse();
            let mut raw = RawStream::new(
                Box::new(stream),
                ctrlc.clone(),
                info.span,
                stderr_info.known_size,
            );
            raw.is_binary = stderr_info.is_binary;
            raw
        }),
        exit_code: info.has_exit_code.then(|| {
            ListStream::from_stream(
                PluginExternalExitCodeStream { io: source.clone() }.fuse(),
                ctrlc.clone(),
            )
        }),
        span: info.span,
        metadata: None,
        trim_end_newline: info.trim_end_newline,
    }
}

/// Write the contents of a [PipelineData::ExternalStream] to `io`.
fn write_full_external_stream(
    io: &Arc<dyn StreamDataIo>,
    stdout: Option<RawStream>,
    stderr: Option<RawStream>,
    exit_code: Option<ListStream>,
) -> Result<(), ShellError> {
    // Consume all streams simultaneously by launching three threads
    for thread in [
        stdout.map(|stdout| {
            let io = io.clone();
            std::thread::spawn(move || {
                for bytes in stdout.stream {
                    io.write_external_stdout(Some(bytes))?;
                }
                io.write_external_stdout(None)
            })
        }),
        stderr.map(|stderr| {
            let io = io.clone();
            std::thread::spawn(move || {
                for bytes in stderr.stream {
                    io.write_external_stderr(Some(bytes))?;
                }
                io.write_external_stderr(None)
            })
        }),
        exit_code.map(|exit_code| {
            let io = io.clone();
            std::thread::spawn(move || {
                for value in exit_code {
                    io.write_external_exit_code(Some(value))?;
                }
                io.write_external_exit_code(None)
            })
        }),
    ]
    .into_iter()
    .flatten()
    {
        thread.join().expect("stream consumer thread panicked")?;
    }
    Ok(())
}

/// Prepare [CallInput] for [PipelineData].
///
/// Handles converting [PluginCustomValue] to [CallInput::Data] if the `plugin_filename` is correct.
///
/// Does not actually send any stream data. You still need to call either [write_full_list_stream]
/// or [write_full_external_stream] as appropriate.
pub(crate) fn make_call_input_from_pipeline_data(
    input: &PipelineData,
    plugin_name: &str,
    plugin_filename: &Path,
) -> Result<CallInput, ShellError> {
    match *input {
        PipelineData::Value(ref value @ Value::CustomValue { ref val, .. }, _) => {
            match val.as_any().downcast_ref::<PluginCustomValue>() {
                Some(plugin_data) if plugin_data.filename == plugin_filename => {
                    Ok(CallInput::Data(PluginData {
                        data: plugin_data.data.clone(),
                        span: value.span(),
                    }))
                }
                _ => {
                    let custom_value_name = val.value_string();
                    Err(ShellError::GenericError {
                        error: format!(
                            "Plugin {} can not handle the custom value {}",
                            plugin_name, custom_value_name
                        ),
                        msg: format!("custom value {custom_value_name}"),
                        span: Some(value.span()),
                        help: None,
                        inner: vec![],
                    })
                }
            }
        }
        PipelineData::Value(Value::LazyRecord { ref val, .. }, _) => {
            Ok(CallInput::Value(val.collect()?))
        }
        PipelineData::Value(ref value, _) => Ok(CallInput::Value(value.clone())),
        PipelineData::ListStream(_, _) => Ok(CallInput::ListStream),
        PipelineData::ExternalStream {
            span,
            ref stdout,
            ref stderr,
            ref exit_code,
            trim_end_newline,
            ..
        } => Ok(CallInput::ExternalStream(ExternalStreamInfo {
            span,
            stdout: stdout.as_ref().map(RawStreamInfo::from),
            stderr: stderr.as_ref().map(RawStreamInfo::from),
            has_exit_code: exit_code.is_some(),
            trim_end_newline,
        })),
        PipelineData::Empty => Ok(CallInput::Empty),
    }
}
