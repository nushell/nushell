use std::collections::VecDeque;

use nu_protocol::{ShellError, Value};

use crate::protocol::StreamData;

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) use tests::gen_stream_data_tests;

/// Methods for reading and writing [crate::protocol::StreamData] contents on an interface.
///
/// This trait must be object safe.
pub(crate) trait StreamDataIo: Send + Sync {
    /// Read a value for a `ListStream`, returning `Ok(None)` at end of stream.
    ///
    /// Other streams will be transparently handled or stored for concurrent readers.
    fn read_list(&self) -> Result<Option<Value>, ShellError>;

    /// Read some bytes for an `ExternalStream`'s `stdout` stream, returning `Ok(None)` at end
    /// of stream.
    ///
    /// Other streams will be transparently handled or stored for concurrent readers.
    fn read_external_stdout(&self) -> Result<Option<Vec<u8>>, ShellError>;

    /// Read some bytes for an `ExternalStream`'s `stderr` stream, returning `Ok(None)` at end
    /// of stream.
    ///
    /// Other streams will be transparently handled or stored for concurrent readers.
    fn read_external_stderr(&self) -> Result<Option<Vec<u8>>, ShellError>;

    /// Read a value for an `ExternalStream`'s `exit_code` stream, returning `Ok(None)` at end
    /// of stream.
    ///
    /// Other streams will be transparently handled or stored for concurrent readers.
    fn read_external_exit_code(&self) -> Result<Option<Value>, ShellError>;

    /// Signal that no more values are desired from a `ListStream` and further messages should
    /// be ignored.
    fn drop_list(&self);

    /// Signal that no more bytes are desired from an `ExternalStream`'s `stdout` and further
    /// messages should be ignored.
    fn drop_external_stdout(&self);

    /// Signal that no more bytes are desired from an `ExternalStream`'s `stderr` and further
    /// messages should be ignored.
    fn drop_external_stderr(&self);

    /// Signal that no more values are desired from an `ExternalStream`'s `exit_code` and further
    /// messages should be ignored.
    fn drop_external_exit_code(&self);

    /// Write a value for a `ListStream`, or `None` to signal end of stream.
    fn write_list(&self, value: Option<Value>) -> Result<(), ShellError>;

    /// Write some bytes for an `ExternalStream`'s `stdout` stream, or `None` to signal end of
    /// stream.
    fn write_external_stdout(
        &self,
        bytes: Option<Result<Vec<u8>, ShellError>>,
    ) -> Result<(), ShellError>;

    /// Write some bytes for an `ExternalStream`'s `stderr` stream, or `None` to signal end of
    /// stream.
    fn write_external_stderr(
        &self,
        bytes: Option<Result<Vec<u8>, ShellError>>,
    ) -> Result<(), ShellError>;

    /// Write a value for an `ExternalStream`'s `exit_code` stream, or `None` to signal end of
    /// stream.
    fn write_external_exit_code(&self, code: Option<Value>) -> Result<(), ShellError>;
}

/// Implement [StreamDataIo] for the given type. The type is expected to have a shape similar to
/// `EngineInterfaceImpl` or `PluginInterfaceImpl`. The following struct fields must be defined:
///
/// * `read: Mutex<(R, StreamBuffers)>` where `R` implements [`PluginRead`](super::PluginRead)
/// * `write: Mutex<W>` where `W` implements [`PluginWrite`](super::PluginWrite)
macro_rules! impl_stream_data_io {
    (
        $type:ident,
        $read_type:ident ($read_method:ident),
        $write_type:ident ($write_method:ident)
    ) => {
        impl<R, W> StreamDataIo for $type<R, W>
        where
            R: $crate::plugin::interface::PluginRead,
            W: $crate::plugin::interface::PluginWrite,
        {
            fn read_list(&self) -> Result<Option<Value>, ShellError> {
                let mut read = self.read.lock().expect("read mutex poisoned");
                // Read from the buffer first
                if let Some(value) = read.1.list.pop_front()? {
                    Ok(value)
                } else {
                    // If we are expecting list stream data, there aren't any other simultaneous
                    // streams, so we don't need to loop. Just try to read and reject it
                    // otherwise
                    match read.0.$read_method()? {
                        Some($read_type::StreamData(StreamData::List(value))) => Ok(value),
                        _ => Err(ShellError::PluginFailedToDecode {
                            msg: "Expected list stream data".into(),
                        }),
                    }
                }
            }

            fn read_external_stdout(&self) -> Result<Option<Vec<u8>>, ShellError> {
                // Loop on the outside of the lock to allow other streams to make progress
                loop {
                    let mut read = self.read.lock().expect("read mutex poisoned");
                    // Read from the buffer first
                    if let Some(bytes) = read.1.external_stdout.pop_front()? {
                        return bytes.transpose();
                    } else {
                        // Skip messages from other streams until we get what we want
                        match read.0.$read_method()? {
                            Some($read_type::StreamData(StreamData::ExternalStdout(bytes))) => {
                                return bytes.transpose()
                            }
                            Some($read_type::StreamData(other)) => read.1.skip(other)?,
                            _ => {
                                return Err(ShellError::PluginFailedToDecode {
                                    msg: "Expected external stream data".into(),
                                })
                            }
                        }
                    }
                }
            }

            fn read_external_stderr(&self) -> Result<Option<Vec<u8>>, ShellError> {
                // Loop on the outside of the lock to allow other streams to make progress
                loop {
                    let mut read = self.read.lock().expect("read mutex poisoned");
                    // Read from the buffer first
                    if let Some(bytes) = read.1.external_stderr.pop_front()? {
                        return bytes.transpose();
                    } else {
                        // Skip messages from other streams until we get what we want
                        match read.0.$read_method()? {
                            Some($read_type::StreamData(StreamData::ExternalStderr(bytes))) => {
                                return bytes.transpose()
                            }
                            Some($read_type::StreamData(other)) => read.1.skip(other)?,
                            _ => {
                                return Err(ShellError::PluginFailedToDecode {
                                    msg: "Expected external stream data".into(),
                                })
                            }
                        }
                    }
                }
            }

            fn read_external_exit_code(&self) -> Result<Option<Value>, ShellError> {
                // Loop on the outside of the lock to allow other streams to make progress
                loop {
                    let mut read = self.read.lock().expect("read mutex poisoned");
                    // Read from the buffer first
                    if let Some(code) = read.1.external_exit_code.pop_front()? {
                        return Ok(code);
                    } else {
                        // Skip messages from other streams until we get what we want
                        match read.0.$read_method()? {
                            Some($read_type::StreamData(StreamData::ExternalExitCode(code))) => {
                                return Ok(code)
                            }
                            Some($read_type::StreamData(other)) => read.1.skip(other)?,
                            _ => {
                                return Err(ShellError::PluginFailedToDecode {
                                    msg: "Expected external stream data".into(),
                                })
                            }
                        }
                    }
                }
            }

            fn drop_list(&self) {
                let mut read = self.read.lock().expect("read mutex poisoned");
                if !matches!(read.1.list, StreamBuffer::NotPresent) {
                    read.1.list = StreamBuffer::Dropped;
                } else {
                    panic!("Tried to drop list stream but it's not present");
                }
            }

            fn drop_external_stdout(&self) {
                let mut read = self.read.lock().expect("read mutex poisoned");
                if !matches!(read.1.external_stdout, StreamBuffer::NotPresent) {
                    read.1.external_stdout = StreamBuffer::Dropped;
                } else {
                    panic!("Tried to drop external_stdout stream but it's not present");
                }
            }

            fn drop_external_stderr(&self) {
                let mut read = self.read.lock().expect("read mutex poisoned");
                if !matches!(read.1.external_stderr, StreamBuffer::NotPresent) {
                    read.1.external_stderr = StreamBuffer::Dropped;
                } else {
                    panic!("Tried to drop external_stderr stream but it's not present");
                }
            }

            fn drop_external_exit_code(&self) {
                let mut read = self.read.lock().expect("read mutex poisoned");
                if !matches!(read.1.external_exit_code, StreamBuffer::NotPresent) {
                    read.1.external_exit_code = StreamBuffer::Dropped;
                } else {
                    panic!("Tried to drop external_exit_code stream but it's not present");
                }
            }

            fn write_list(&self, value: Option<Value>) -> Result<(), ShellError> {
                let mut write = self.write.lock().expect("write mutex poisoned");
                let is_final = value.is_none();
                write.$write_method(&$write_type::StreamData(StreamData::List(value)))?;
                // Try to flush final value
                if is_final {
                    write.flush()?;
                }
                Ok(())
            }

            fn write_external_stdout(
                &self,
                bytes: Option<Result<Vec<u8>, ShellError>>,
            ) -> Result<(), ShellError> {
                let mut write = self.write.lock().expect("write mutex poisoned");
                let is_final = bytes.is_none();
                write.$write_method(&$write_type::StreamData(StreamData::ExternalStdout(bytes)))?;
                // Try to flush final value
                if is_final {
                    write.flush()?;
                }
                Ok(())
            }

            fn write_external_stderr(
                &self,
                bytes: Option<Result<Vec<u8>, ShellError>>,
            ) -> Result<(), ShellError> {
                let mut write = self.write.lock().expect("write mutex poisoned");
                let is_final = bytes.is_none();
                write.$write_method(&$write_type::StreamData(StreamData::ExternalStderr(bytes)))?;
                // Try to flush final value
                if is_final {
                    write.flush()?;
                }
                Ok(())
            }

            fn write_external_exit_code(&self, code: Option<Value>) -> Result<(), ShellError> {
                let mut write = self.write.lock().expect("write mutex poisoned");
                let is_final = code.is_none();
                write
                    .$write_method(&$write_type::StreamData(StreamData::ExternalExitCode(code)))?;
                // Try to flush final value
                if is_final {
                    write.flush()?;
                }
                Ok(())
            }
        }
    };
}

pub(crate) use impl_stream_data_io;

/// Buffers for stream messages that temporarily can't be handled
#[derive(Debug, Default)]
pub(crate) struct StreamBuffers {
    pub list: StreamBuffer<Option<Value>>,
    pub external_stdout: StreamBuffer<Option<Result<Vec<u8>, ShellError>>>,
    pub external_stderr: StreamBuffer<Option<Result<Vec<u8>, ShellError>>>,
    pub external_exit_code: StreamBuffer<Option<Value>>,
}

/// A buffer for stream messages that need to be stored temporarily to allow a different stream
/// to be consumed.
///
/// The buffer is a FIFO queue.
#[derive(Debug, Default)]
pub(crate) enum StreamBuffer<T> {
    /// The default state: this stream was not specified for use, so there is no buffer available
    /// and reading a message directed for this stream will cause an error.
    #[default]
    NotPresent,
    /// This stream was specified for use, but the reader was dropped, so no further messages are
    /// desired. Any messages read that were directed for this stream will just be silently
    /// discarded.
    Dropped,
    /// This stream was specified for use, and there is still a living reader that expects messages
    /// from it. We store messages temporarily to allow another stream to proceed out-of-order.
    Present(VecDeque<T>),
}

impl<T> StreamBuffer<T> {
    /// Returns a [StreamBuffer::Present] with a new empty buffer if the `condition` is true, or
    /// else returns [StreamBuffer::NotPresent].
    pub fn present_if(condition: bool) -> StreamBuffer<T> {
        if condition {
            StreamBuffer::Present(VecDeque::new())
        } else {
            StreamBuffer::NotPresent
        }
    }

    /// Push a message onto the back of the buffer.
    ///
    /// Returns an error if this buffer is `NotPresent`. Discards the message if it is `Dropped`.
    pub fn push_back(&mut self, value: T) -> Result<(), ShellError> {
        match self {
            StreamBuffer::NotPresent => Err(ShellError::PluginFailedToDecode {
                msg: "Tried to read into a stream that is not present".into(),
            }),
            StreamBuffer::Dropped => Ok(()), // just silently drop the message
            StreamBuffer::Present(ref mut buf) => {
                buf.push_back(value);
                Ok(())
            }
        }
    }

    /// Try to pop a message from the front of the buffer.
    ///
    /// Returns `Ok(None)` if there are no messages waiting in the buffer, or an error if this
    /// buffer is either `NotPresent` or `Dropped`.
    pub fn pop_front(&mut self) -> Result<Option<T>, ShellError> {
        match self {
            StreamBuffer::Present(ref mut buf) => Ok(buf.pop_front()),

            StreamBuffer::NotPresent => Err(ShellError::PluginFailedToDecode {
                msg: "Tried to read from a stream that is not present".into(),
            }),
            StreamBuffer::Dropped => Err(ShellError::PluginFailedToDecode {
                msg: "Tried to read from a stream that is already dropped".into(),
            }),
        }
    }

    /// True if the buffer is [Present].
    #[allow(dead_code)]
    pub fn is_present(&self) -> bool {
        matches!(self, StreamBuffer::Present(..))
    }

    /// True if the buffer is [NotPresent].
    #[allow(dead_code)]
    pub fn is_not_present(&self) -> bool {
        matches!(self, StreamBuffer::NotPresent)
    }

    /// True if the buffer is [Dropped].
    pub fn is_dropped(&self) -> bool {
        matches!(self, StreamBuffer::Dropped)
    }
}

impl StreamBuffers {
    /// Create a new [StreamBuffers] with an empty buffer for a `ListStream`.
    ///
    /// Other stream messages will be rejected with an error.
    pub fn new_list() -> StreamBuffers {
        StreamBuffers {
            list: StreamBuffer::Present(VecDeque::new()),
            external_stdout: StreamBuffer::NotPresent,
            external_stderr: StreamBuffer::NotPresent,
            external_exit_code: StreamBuffer::NotPresent,
        }
    }

    /// Create a new [StreamBuffers] with empty buffers for an `ExternalStream`.
    ///
    /// The buffers will be `Present` according to the values of the parameters. Any stream messages
    /// that do not belong to streams specified as present here will be rejected with an error.
    pub fn new_external(has_stdout: bool, has_stderr: bool, has_exit_code: bool) -> StreamBuffers {
        StreamBuffers {
            list: StreamBuffer::NotPresent,
            external_stdout: StreamBuffer::present_if(has_stdout),
            external_stderr: StreamBuffer::present_if(has_stderr),
            external_exit_code: StreamBuffer::present_if(has_exit_code),
        }
    }

    /// Temporarily store [StreamData] for a stream. Use this if a message was received that belongs
    /// to a stream other than the one actively being read.
    pub fn skip(&mut self, data: StreamData) -> Result<(), ShellError> {
        match data {
            StreamData::List(val) => self.list.push_back(val),
            StreamData::ExternalStdout(bytes) => self.external_stdout.push_back(bytes),
            StreamData::ExternalStderr(bytes) => self.external_stderr.push_back(bytes),
            StreamData::ExternalExitCode(code) => self.external_exit_code.push_back(code),
        }
    }
}
