//! Implements the stream multiplexing interface for both the plugin side and the engine side.

use crate::{
    plugin::Encoder,
    protocol::{
        ExternalStreamInfo, ListStreamInfo, PipelineDataHeader, RawStreamInfo, StreamMessage,
    },
    sequence::Sequence,
};
use nu_protocol::{ListStream, PipelineData, RawStream, ShellError};
use std::{
    io::Write,
    sync::{
        atomic::{AtomicBool, Ordering::Relaxed},
        Arc, Mutex,
    },
    thread,
};

mod stream;

mod engine;
pub use engine::{EngineInterface, EngineInterfaceManager, ReceivedPluginCall};

mod plugin;
pub use plugin::{PluginInterface, PluginInterfaceManager};

use self::stream::{StreamManager, StreamManagerHandle, StreamWriter, WriteStreamMessage};

#[cfg(test)]
mod test_util;

#[cfg(test)]
mod tests;

/// The maximum number of list stream values to send without acknowledgement. This should be tuned
/// with consideration for memory usage.
const LIST_STREAM_HIGH_PRESSURE: i32 = 100;

/// The maximum number of raw stream buffers to send without acknowledgement. This should be tuned
/// with consideration for memory usage.
const RAW_STREAM_HIGH_PRESSURE: i32 = 50;

/// Read input/output from the stream.
///
/// This is not a public API.
#[doc(hidden)]
pub trait PluginRead<T> {
    /// Returns `Ok(None)` on end of stream.
    fn read(&mut self) -> Result<Option<T>, ShellError>;
}

impl<R, E, T> PluginRead<T> for (R, E)
where
    R: std::io::BufRead,
    E: Encoder<T>,
{
    fn read(&mut self) -> Result<Option<T>, ShellError> {
        self.1.decode(&mut self.0)
    }
}

impl<R, T> PluginRead<T> for &mut R
where
    R: PluginRead<T>,
{
    fn read(&mut self) -> Result<Option<T>, ShellError> {
        (**self).read()
    }
}

/// Write input/output to the stream.
///
/// The write should be atomic, without interference from other threads.
///
/// This is not a public API.
#[doc(hidden)]
pub trait PluginWrite<T>: Send + Sync {
    fn write(&self, data: &T) -> Result<(), ShellError>;

    /// Flush any internal buffers, if applicable.
    fn flush(&self) -> Result<(), ShellError>;

    /// True if this output is stdout, so that plugins can avoid using stdout for their own purpose
    fn is_stdout(&self) -> bool {
        false
    }
}

impl<E, T> PluginWrite<T> for (std::io::Stdout, E)
where
    E: Encoder<T>,
{
    fn write(&self, data: &T) -> Result<(), ShellError> {
        let mut lock = self.0.lock();
        self.1.encode(data, &mut lock)
    }

    fn flush(&self) -> Result<(), ShellError> {
        self.0.lock().flush().map_err(|err| ShellError::IOError {
            msg: err.to_string(),
        })
    }

    fn is_stdout(&self) -> bool {
        true
    }
}

impl<W, E, T> PluginWrite<T> for (Mutex<W>, E)
where
    W: std::io::Write + Send,
    E: Encoder<T>,
{
    fn write(&self, data: &T) -> Result<(), ShellError> {
        let mut lock = self.0.lock().map_err(|_| ShellError::NushellFailed {
            msg: "writer mutex poisoned".into(),
        })?;
        self.1.encode(data, &mut *lock)
    }

    fn flush(&self) -> Result<(), ShellError> {
        let mut lock = self.0.lock().map_err(|_| ShellError::NushellFailed {
            msg: "writer mutex poisoned".into(),
        })?;
        lock.flush().map_err(|err| ShellError::IOError {
            msg: err.to_string(),
        })
    }
}

impl<W, T> PluginWrite<T> for &W
where
    W: PluginWrite<T>,
{
    fn write(&self, data: &T) -> Result<(), ShellError> {
        (**self).write(data)
    }

    fn flush(&self) -> Result<(), ShellError> {
        (**self).flush()
    }

    fn is_stdout(&self) -> bool {
        (**self).is_stdout()
    }
}

/// An interface manager handles I/O and state management for communication between a plugin and the
/// engine. See [`PluginInterfaceManager`] for communication from the engine side to a plugin, or
/// [`EngineInterfaceManager`] for communication from the plugin side to the engine.
///
/// There is typically one [`InterfaceManager`] consuming input from a background thread, and
/// managing shared state.
///
/// This is not a public API.
#[doc(hidden)]
pub trait InterfaceManager {
    /// The corresponding interface type.
    type Interface: Interface + 'static;

    /// The input message type.
    type Input;

    /// Make a new interface that communicates with this [`InterfaceManager`].
    fn get_interface(&self) -> Self::Interface;

    /// Consume an input message.
    ///
    /// When implementing, call [`.consume_stream_message()`] for any encapsulated
    /// [`StreamMessage`]s received.
    fn consume(&mut self, input: Self::Input) -> Result<(), ShellError>;

    /// Get the [`StreamManager`] for handling operations related to stream messages.
    fn stream_manager(&self) -> &StreamManager;

    /// Prepare [`PipelineData`] after reading. This is called by `read_pipeline_data()` as
    /// a hook so that values that need special handling can be taken care of.
    fn prepare_pipeline_data(&self, data: PipelineData) -> Result<PipelineData, ShellError>;

    /// Consume an input stream message.
    ///
    /// This method is provided for implementors to use.
    fn consume_stream_message(&mut self, message: StreamMessage) -> Result<(), ShellError> {
        self.stream_manager().handle_message(message)
    }

    /// Generate `PipelineData` for reading a stream, given a [`PipelineDataHeader`] that was
    /// received from the other side.
    ///
    /// This method is provided for implementors to use.
    fn read_pipeline_data(
        &self,
        header: PipelineDataHeader,
        ctrlc: Option<&Arc<AtomicBool>>,
    ) -> Result<PipelineData, ShellError> {
        self.prepare_pipeline_data(match header {
            PipelineDataHeader::Empty => PipelineData::Empty,
            PipelineDataHeader::Value(value) => PipelineData::Value(value, None),
            PipelineDataHeader::ListStream(info) => {
                let handle = self.stream_manager().get_handle();
                let reader = handle.read_stream(info.id, self.get_interface())?;
                PipelineData::ListStream(ListStream::from_stream(reader, ctrlc.cloned()), None)
            }
            PipelineDataHeader::ExternalStream(info) => {
                let handle = self.stream_manager().get_handle();
                let span = info.span;
                let new_raw_stream = |raw_info: RawStreamInfo| {
                    let reader = handle.read_stream(raw_info.id, self.get_interface())?;
                    let mut stream =
                        RawStream::new(Box::new(reader), ctrlc.cloned(), span, raw_info.known_size);
                    stream.is_binary = raw_info.is_binary;
                    Ok::<_, ShellError>(stream)
                };
                PipelineData::ExternalStream {
                    stdout: info.stdout.map(new_raw_stream).transpose()?,
                    stderr: info.stderr.map(new_raw_stream).transpose()?,
                    exit_code: info
                        .exit_code
                        .map(|list_info| {
                            handle
                                .read_stream(list_info.id, self.get_interface())
                                .map(|reader| ListStream::from_stream(reader, ctrlc.cloned()))
                        })
                        .transpose()?,
                    span: info.span,
                    metadata: None,
                    trim_end_newline: info.trim_end_newline,
                }
            }
        })
    }
}

/// An interface provides an API for communicating with a plugin or the engine and facilitates
/// stream I/O. See [`PluginInterface`] for the API from the engine side to a plugin, or
/// [`EngineInterface`] for the API from the plugin side to the engine.
///
/// There can be multiple copies of the interface managed by a single [`InterfaceManager`].
///
/// This is not a public API.
#[doc(hidden)]
pub trait Interface: Clone + Send {
    /// The output message type, which must be capable of encapsulating a [`StreamMessage`].
    type Output: From<StreamMessage>;

    /// Any context required to construct [`PipelineData`]. Can be `()` if not needed.
    type DataContext;

    /// Write an output message.
    fn write(&self, output: Self::Output) -> Result<(), ShellError>;

    /// Flush the output buffer, so messages are visible to the other side.
    fn flush(&self) -> Result<(), ShellError>;

    /// Get the sequence for generating new [`StreamId`](crate::protocol::StreamId)s.
    fn stream_id_sequence(&self) -> &Sequence;

    /// Get the [`StreamManagerHandle`] for doing stream operations.
    fn stream_manager_handle(&self) -> &StreamManagerHandle;

    /// Prepare [`PipelineData`] to be written. This is called by `init_write_pipeline_data()` as
    /// a hook so that values that need special handling can be taken care of.
    fn prepare_pipeline_data(
        &self,
        data: PipelineData,
        context: &Self::DataContext,
    ) -> Result<PipelineData, ShellError>;

    /// Initialize a write for [`PipelineData`]. This returns two parts: the header, which can be
    /// embedded in the particular message that references the stream, and a writer, which will
    /// write out all of the data in the pipeline when `.write()` is called.
    ///
    /// Note that not all [`PipelineData`] starts a stream. You should call `write()` anyway, as
    /// it will automatically handle this case.
    ///
    /// This method is provided for implementors to use.
    fn init_write_pipeline_data(
        &self,
        data: PipelineData,
        context: &Self::DataContext,
    ) -> Result<(PipelineDataHeader, PipelineDataWriter<Self>), ShellError> {
        // Allocate a stream id and a writer
        let new_stream = |high_pressure_mark: i32| {
            // Get a free stream id
            let id = self.stream_id_sequence().next()?;
            // Create the writer
            let writer =
                self.stream_manager_handle()
                    .write_stream(id, self.clone(), high_pressure_mark)?;
            Ok::<_, ShellError>((id, writer))
        };
        match self.prepare_pipeline_data(data, context)? {
            PipelineData::Value(value, _) => {
                Ok((PipelineDataHeader::Value(value), PipelineDataWriter::None))
            }
            PipelineData::Empty => Ok((PipelineDataHeader::Empty, PipelineDataWriter::None)),
            PipelineData::ListStream(stream, _) => {
                let (id, writer) = new_stream(LIST_STREAM_HIGH_PRESSURE)?;
                Ok((
                    PipelineDataHeader::ListStream(ListStreamInfo { id }),
                    PipelineDataWriter::ListStream(writer, stream),
                ))
            }
            PipelineData::ExternalStream {
                stdout,
                stderr,
                exit_code,
                span,
                metadata: _,
                trim_end_newline,
            } => {
                // Create the writers and stream ids
                let stdout_stream = stdout
                    .is_some()
                    .then(|| new_stream(RAW_STREAM_HIGH_PRESSURE))
                    .transpose()?;
                let stderr_stream = stderr
                    .is_some()
                    .then(|| new_stream(RAW_STREAM_HIGH_PRESSURE))
                    .transpose()?;
                let exit_code_stream = exit_code
                    .is_some()
                    .then(|| new_stream(LIST_STREAM_HIGH_PRESSURE))
                    .transpose()?;
                // Generate the header, with the stream ids
                let header = PipelineDataHeader::ExternalStream(ExternalStreamInfo {
                    span,
                    stdout: stdout
                        .as_ref()
                        .zip(stdout_stream.as_ref())
                        .map(|(stream, (id, _))| RawStreamInfo::new(*id, stream)),
                    stderr: stderr
                        .as_ref()
                        .zip(stderr_stream.as_ref())
                        .map(|(stream, (id, _))| RawStreamInfo::new(*id, stream)),
                    exit_code: exit_code_stream
                        .as_ref()
                        .map(|&(id, _)| ListStreamInfo { id }),
                    trim_end_newline,
                });
                // Collect the writers
                let writer = PipelineDataWriter::ExternalStream {
                    stdout: stdout_stream.map(|(_, writer)| writer).zip(stdout),
                    stderr: stderr_stream.map(|(_, writer)| writer).zip(stderr),
                    exit_code: exit_code_stream.map(|(_, writer)| writer).zip(exit_code),
                };
                Ok((header, writer))
            }
        }
    }
}

impl<T> WriteStreamMessage for T
where
    T: Interface,
{
    fn write_stream_message(&mut self, msg: StreamMessage) -> Result<(), ShellError> {
        self.write(msg.into())
    }

    fn flush(&mut self) -> Result<(), ShellError> {
        <Self as Interface>::flush(self)
    }
}

/// Completes the write operation for a [`PipelineData`]. You must call
/// [`PipelineDataWriter::write()`] to write all of the data contained within the streams.
#[derive(Default)]
#[must_use]
pub enum PipelineDataWriter<W: WriteStreamMessage> {
    #[default]
    None,
    ListStream(StreamWriter<W>, ListStream),
    ExternalStream {
        stdout: Option<(StreamWriter<W>, RawStream)>,
        stderr: Option<(StreamWriter<W>, RawStream)>,
        exit_code: Option<(StreamWriter<W>, ListStream)>,
    },
}

impl<W> PipelineDataWriter<W>
where
    W: WriteStreamMessage + Send + 'static,
{
    /// Write all of the data in each of the streams. This method waits for completion.
    pub(crate) fn write(self) -> Result<(), ShellError> {
        match self {
            // If no stream was contained in the PipelineData, do nothing.
            PipelineDataWriter::None => Ok(()),
            // Write a list stream.
            PipelineDataWriter::ListStream(mut writer, stream) => {
                writer.write_all(stream)?;
                Ok(())
            }
            // Write all three possible streams of an ExternalStream on separate threads.
            PipelineDataWriter::ExternalStream {
                stdout,
                stderr,
                exit_code,
            } => {
                thread::scope(|scope| {
                    let stderr_thread = stderr
                        .map(|(mut writer, stream)| {
                            thread::Builder::new()
                                .name("plugin stderr writer".into())
                                .spawn_scoped(scope, move || {
                                    writer.write_all(raw_stream_iter(stream))
                                })
                        })
                        .transpose()?;
                    let exit_code_thread = exit_code
                        .map(|(mut writer, stream)| {
                            thread::Builder::new()
                                .name("plugin exit_code writer".into())
                                .spawn_scoped(scope, move || writer.write_all(stream))
                        })
                        .transpose()?;
                    // Optimize for stdout: if only stdout is present, don't spawn any other
                    // threads.
                    if let Some((mut writer, stream)) = stdout {
                        writer.write_all(raw_stream_iter(stream))?;
                    }
                    let panicked = |thread_name: &str| {
                        Err(ShellError::NushellFailed {
                            msg: format!(
                                "{thread_name} thread panicked in PipelineDataWriter::write"
                            ),
                        })
                    };
                    stderr_thread
                        .map(|t| t.join().unwrap_or_else(|_| panicked("stderr")))
                        .transpose()?;
                    exit_code_thread
                        .map(|t| t.join().unwrap_or_else(|_| panicked("exit_code")))
                        .transpose()?;
                    Ok(())
                })
            }
        }
    }

    /// Write all of the data in each of the streams. This method returns immediately; any necessary
    /// write will happen in the background. If a thread was spawned, its handle is returned.
    pub(crate) fn write_background(
        self,
    ) -> Result<Option<thread::JoinHandle<Result<(), ShellError>>>, ShellError> {
        match self {
            PipelineDataWriter::None => Ok(None),
            _ => Ok(Some(
                thread::Builder::new()
                    .name("plugin stream background writer".into())
                    .spawn(move || {
                        let result = self.write();
                        if let Err(ref err) = result {
                            // Assume that the background thread error probably won't be handled and log it
                            // here just in case.
                            log::warn!("Error while writing pipeline in background: {err}");
                        }
                        result
                    })?,
            )),
        }
    }
}

/// Custom iterator for [`RawStream`] that respects ctrlc, but still has binary chunks
fn raw_stream_iter(stream: RawStream) -> impl Iterator<Item = Result<Vec<u8>, ShellError>> {
    let ctrlc = stream.ctrlc;
    stream
        .stream
        .take_while(move |_| ctrlc.as_ref().map(|b| !b.load(Relaxed)).unwrap_or(true))
}
