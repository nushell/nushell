//! Interface used by the engine to communicate with the plugin.

use std::{
    io::{BufRead, Write},
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Arc, Mutex},
};

use nu_protocol::{
    ast::Call,
    engine::{EngineState, Stack},
    ListStream, PipelineData, RawStream, ShellError, Span, Value,
};

use crate::{
    plugin::PluginEncoder,
    protocol::{
        ExternalStreamInfo, PluginCall, PluginCallResponse, PluginCustomValue, PluginInput,
        PluginOutput, StreamData,
    },
};

use super::{
    make_external_stream, make_list_stream,
    stream_data_io::{impl_stream_data_io, StreamBuffer, StreamBuffers, StreamDataIo},
    write_full_external_stream, write_full_list_stream, PluginRead, PluginWrite,
};

#[cfg(test)]
mod tests;

/// Object safe trait for abstracting operations required of the plugin context.
pub(crate) trait PluginExecutionContext: Send + Sync {
    /// The plugin's filename
    fn filename(&self) -> &Path;
    /// The shell used to execute the plugin
    fn shell(&self) -> Option<&Path>;
    /// The [Span] for the command execution (`call.head`)
    fn command_span(&self) -> Span;
    /// The name of the command being executed
    fn command_name(&self) -> &str;
    /// The interrupt signal, if present
    fn ctrlc(&self) -> Option<&Arc<AtomicBool>>;
}

/// The execution context of a plugin.
#[derive(Debug)]
pub(crate) struct PluginExecutionNushellContext {
    filename: PathBuf,
    shell: Option<PathBuf>,
    command_span: Span,
    command_name: String,
    ctrlc: Option<Arc<AtomicBool>>,
    // If more operations are required of the context, fields can be added here.
    //
    // It may be required to insert the entire EngineState/Call/Stack in here to support
    // future features and that's okay
}

impl PluginExecutionNushellContext {
    pub fn new(
        filename: impl Into<PathBuf>,
        shell: Option<impl Into<PathBuf>>,
        engine_state: &EngineState,
        _stack: &Stack,
        call: &Call,
    ) -> PluginExecutionNushellContext {
        PluginExecutionNushellContext {
            filename: filename.into(),
            shell: shell.map(Into::into),
            command_span: call.head,
            command_name: engine_state.get_decl(call.decl_id).name().to_owned(),
            ctrlc: engine_state.ctrlc.clone(),
        }
    }
}

impl PluginExecutionContext for PluginExecutionNushellContext {
    fn filename(&self) -> &Path {
        &self.filename
    }

    fn shell(&self) -> Option<&Path> {
        self.shell.as_deref()
    }

    fn command_span(&self) -> Span {
        self.command_span
    }

    fn command_name(&self) -> &str {
        &self.command_name
    }

    fn ctrlc(&self) -> Option<&Arc<AtomicBool>> {
        self.ctrlc.as_ref()
    }
}

pub(crate) struct PluginInterfaceImpl<R, W> {
    // Always lock read and then write mutex, if using both
    // Stream inputs that can't be handled immediately can be put on the buffer
    read: Mutex<(R, StreamBuffers)>,
    write: Mutex<W>,
    context: Option<Arc<dyn PluginExecutionContext>>,
}

impl<R, W> PluginInterfaceImpl<R, W> {
    pub(crate) fn new(
        reader: R,
        writer: W,
        context: Option<Arc<dyn PluginExecutionContext>>,
    ) -> PluginInterfaceImpl<R, W> {
        PluginInterfaceImpl {
            read: Mutex::new((reader, StreamBuffers::default())),
            write: Mutex::new(writer),
            context,
        }
    }
}

// Implement the stream handling methods (see StreamDataIo).
impl_stream_data_io!(
    PluginInterfaceImpl,
    PluginOutput(read_output),
    PluginInput(write_input)
);

/// The trait indirection is so that we can hide the types with a trait object inside
/// PluginInterface. As such, this trait must remain object safe.
pub(crate) trait PluginInterfaceIo: StreamDataIo {
    fn context(&self) -> Option<&Arc<dyn PluginExecutionContext>>;
    fn write_call(&self, call: PluginCall) -> Result<(), ShellError>;
    fn read_call_response(&self) -> Result<PluginCallResponse, ShellError>;
}

impl<R, W> PluginInterfaceIo for PluginInterfaceImpl<R, W>
where
    R: PluginRead,
    W: PluginWrite,
{
    fn context(&self) -> Option<&Arc<dyn PluginExecutionContext>> {
        self.context.as_ref()
    }

    fn write_call(&self, call: PluginCall) -> Result<(), ShellError> {
        let mut write = self.write.lock().expect("write mutex poisoned");
        log::trace!("Writing plugin call: {call:?}");

        write.write_input(&PluginInput::Call(call))?;
        write.flush()?;

        log::trace!("Wrote plugin call");
        Ok(())
    }

    fn read_call_response(&self) -> Result<PluginCallResponse, ShellError> {
        log::trace!("Reading plugin call response");

        let mut read = self.read.lock().expect("read mutex poisoned");
        loop {
            match read.0.read_output()? {
                Some(PluginOutput::CallResponse(response)) => {
                    // Check the call input type to set the stream buffers up
                    match &response {
                        PluginCallResponse::ListStream => {
                            read.1 = StreamBuffers::new_list();
                            log::trace!("Read plugin call response. Expecting list stream");
                        }
                        PluginCallResponse::ExternalStream(ExternalStreamInfo {
                            stdout,
                            stderr,
                            has_exit_code,
                            ..
                        }) => {
                            read.1 = StreamBuffers::new_external(
                                stdout.is_some(),
                                stderr.is_some(),
                                *has_exit_code,
                            );
                            log::trace!("Read plugin call response. Expecting external stream");
                        }
                        _ => {
                            read.1 = StreamBuffers::default(); // no buffers
                            log::trace!("Read plugin call response. No stream expected");
                        }
                    }
                    return Ok(response);
                }
                // Skip over any remaining stream data for dropped streams
                Some(PluginOutput::StreamData(StreamData::List(_))) if read.1.list.is_dropped() => {
                    continue
                }
                Some(PluginOutput::StreamData(StreamData::ExternalStdout(_)))
                    if read.1.external_stdout.is_dropped() =>
                {
                    continue
                }
                Some(PluginOutput::StreamData(StreamData::ExternalStderr(_)))
                    if read.1.external_stderr.is_dropped() =>
                {
                    continue
                }
                Some(PluginOutput::StreamData(StreamData::ExternalExitCode(_)))
                    if read.1.external_exit_code.is_dropped() =>
                {
                    continue
                }
                // Other stream data is an error
                Some(PluginOutput::StreamData(_)) => {
                    return Err(ShellError::PluginFailedToDecode {
                        msg: "expected CallResponse, got unexpected StreamData".into(),
                    })
                }
                // End of input
                None => {
                    return Err(ShellError::PluginFailedToDecode {
                        msg: "unexpected end of stream before receiving call response".into(),
                    })
                }
            }
        }
    }
}

/// Implements communication and stream handling for a plugin instance.
#[derive(Clone)]
pub(crate) struct PluginInterface {
    io: Arc<dyn PluginInterfaceIo>,
    // FIXME: This is only necessary because trait upcasting is not yet supported, so we have to
    // generate this variant of the Arc while we know the actual type. It can be removed once
    // https://github.com/rust-lang/rust/issues/65991 is closed and released.
    io_stream: Arc<dyn StreamDataIo>,
}

impl<R, W> From<PluginInterfaceImpl<R, W>> for PluginInterface
where
    R: PluginRead + 'static,
    W: PluginWrite + 'static,
{
    fn from(plugin_impl: PluginInterfaceImpl<R, W>) -> Self {
        let arc = Arc::new(plugin_impl);
        PluginInterface {
            io: arc.clone(),
            io_stream: arc,
        }
    }
}

impl PluginInterface {
    /// Create the plugin interface from the given reader, writer, encoder, and context.
    pub(crate) fn new<R, W, E>(
        reader: R,
        writer: W,
        encoder: E,
        context: Option<Arc<dyn PluginExecutionContext>>,
    ) -> PluginInterface
    where
        R: BufRead + Send + 'static,
        W: Write + Send + 'static,
        E: PluginEncoder + 'static,
    {
        PluginInterfaceImpl::new((reader, encoder.clone()), (writer, encoder), context).into()
    }

    /// Write a [PluginCall] to the plugin
    pub(crate) fn write_call(&self, call: PluginCall) -> Result<(), ShellError> {
        self.io.write_call(call)
    }

    /// Read a [PluginCallResponse] back from the plugin
    pub(crate) fn read_call_response(&self) -> Result<PluginCallResponse, ShellError> {
        self.io.read_call_response()
    }

    /// Create [PipelineData] appropriate for the given [PluginCallResponse].
    ///
    /// Only usable with response types that emulate [PipelineData].
    ///
    /// # Panics
    ///
    /// If [PluginExecutionContext] was not provided when creating the interface.
    pub(crate) fn make_pipeline_data(
        &self,
        response: PluginCallResponse,
    ) -> Result<PipelineData, ShellError> {
        let context = self
            .io
            .context()
            .expect("PluginExecutionContext must be provided to call make_pipeline_data");

        match response {
            PluginCallResponse::Error(err) => Err(err.into()),
            PluginCallResponse::Signature(_) => Err(ShellError::GenericError {
                error: "Plugin missing value".into(),
                msg: "Received a signature from plugin instead of value or stream".into(),
                span: Some(context.command_span()),
                help: None,
                inner: vec![],
            }),
            PluginCallResponse::Empty => Ok(PipelineData::Empty),
            PluginCallResponse::Value(value) => Ok(PipelineData::Value(*value, None)),
            PluginCallResponse::PluginData(name, plugin_data) => {
                // Convert to PluginCustomData
                let value = Value::custom_value(
                    Box::new(PluginCustomValue {
                        name,
                        data: plugin_data.data,
                        filename: context.filename().to_owned(),
                        shell: context.shell().map(|p| p.to_owned()),
                        source: context.command_name().to_owned(),
                    }),
                    plugin_data.span,
                );
                Ok(PipelineData::Value(value, None))
            }
            PluginCallResponse::ListStream => Ok(make_list_stream(
                self.io_stream.clone(),
                context.ctrlc().cloned(),
            )),
            PluginCallResponse::ExternalStream(info) => Ok(make_external_stream(
                self.io_stream.clone(),
                &info,
                context.ctrlc().cloned(),
            )),
        }
    }

    /// Write the contents of a [ListStream] to `io`.
    pub fn write_full_list_stream(&self, list_stream: ListStream) -> Result<(), ShellError> {
        write_full_list_stream(&self.io_stream, list_stream)
    }

    /// Write the contents of a [PipelineData::ExternalStream].
    pub fn write_full_external_stream(
        &self,
        stdout: Option<RawStream>,
        stderr: Option<RawStream>,
        exit_code: Option<ListStream>,
    ) -> Result<(), ShellError> {
        write_full_external_stream(&self.io_stream, stdout, stderr, exit_code)
    }
}
