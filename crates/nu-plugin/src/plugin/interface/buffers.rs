use std::collections::VecDeque;

use nu_protocol::{ShellError, Value};

use crate::protocol::{PipelineDataHeader, StreamId, StreamData};

#[cfg(test)]
mod tests;

/// Buffers for stream messages that temporarily can't be handled
#[derive(Debug, Default)]
pub(crate) struct StreamBuffers {
    streams: Vec<(StreamId, Box<TypedStreamBuffer>)>,
}

impl StreamBuffers {
    /// Get the buffers for a stream by id. Returns an error if the stream is not present.
    pub fn get(&mut self, id: StreamId) -> Result<&mut TypedStreamBuffer, ShellError> {
        self.streams
            .iter_mut()
            .find(|(found_id, _)| *found_id == id)
            .map(|(_, bufs)| &mut **bufs)
            .ok_or_else(|| ShellError::PluginFailedToDecode {
                msg: format!("Tried to write to a non-existent stream: {id}"),
            })
    }

    /// Insert a new stream by id. Returns an error if the stream is already present.
    pub fn insert(&mut self, id: StreamId, buffer: TypedStreamBuffer) -> Result<(), ShellError> {
        // Ensure the stream doesn't already exist
        if self.streams.iter().any(|(found_id, _)| *found_id == id) {
            Err(ShellError::PluginFailedToDecode {
                msg: format!("Tried to initialize already existing stream: {id}"),
            })
        } else {
            self.streams.push((id, Box::new(buffer)));
            Ok(())
        }
    }

    /// Remove any streams that were fully consumed.
    pub fn cleanup(&mut self) {
        self.streams.retain(|(id, bufs)| {
            if bufs.is_fully_consumed() {
                log::trace!("Cleaning up stream id={id}");
                false
            } else {
                true
            }
        });
    }

    /// Create buffers for the given stream header.
    ///
    /// Returns an error if the specified stream id already existed.
    pub fn init_stream(&mut self, header: &PipelineDataHeader) -> Result<(), ShellError> {
        match header {
            PipelineDataHeader::ListStream(info) => {
                log::trace!("New list stream id={}", info.id);
                self.insert(info.id, TypedStreamBuffer::new_list())
            }

            PipelineDataHeader::ExternalStream(info) => {
                log::trace!(
                    "New external stream stdout.id={:?}, stderr.id={:?}, exit_code.id={:?}",
                    info.stdout.as_ref().map(|info| info.id),
                    info.stderr.as_ref().map(|info| info.id),
                    info.exit_code.as_ref().map(|info| info.id)
                );

                if let Some(ref stdout_info) = info.stdout {
                    self.insert(stdout_info.id, TypedStreamBuffer::new_raw())?;
                }
                if let Some(ref stderr_info) = info.stderr {
                    self.insert(stderr_info.id, TypedStreamBuffer::new_raw())?;
                }
                if let Some(ref exit_code_info) = info.exit_code {
                    self.insert(exit_code_info.id, TypedStreamBuffer::new_list())?;
                }
                Ok(())
            }

            // Don't have to do anything for these
            PipelineDataHeader::Empty
            | PipelineDataHeader::Value(_)
            | PipelineDataHeader::PluginData(_) => Ok(()),
        }
    }

    /// Temporarily store [StreamData] for a stream. Use this if a message was received that belongs
    /// to a stream other than the one actively being read.
    pub fn skip(&mut self, id: StreamId, data: StreamData) -> Result<(), ShellError> {
        self.get(id)?.push_back(data)
    }
}

/// Different types of stream buffers.
#[derive(Debug)]
pub(crate) enum TypedStreamBuffer {
    /// List buffers accept values.
    List(StreamBuffer<Value>),
    /// Raw buffers accept byte vectors, or errors, and are conceptually a fallible byte stream.
    Raw(StreamBuffer<Result<Vec<u8>, ShellError>>),
}

impl TypedStreamBuffer {
    /// Create a new [TypedStreamBuffer] with an empty buffer for a `ListStream`.
    ///
    /// Other stream messages will be rejected with an error.
    pub const fn new_list() -> TypedStreamBuffer {
        TypedStreamBuffer::List(StreamBuffer::new())
    }

    /// Create a new [TypedStreamBuffer] with an empty buffer for a `RawStream`.
    pub const fn new_raw() -> TypedStreamBuffer {
        TypedStreamBuffer::Raw(StreamBuffer::new())
    }

    /// Push a stream message onto the correct buffer. Returns an error if the message is not
    /// accepted.
    pub fn push_back(&mut self, message: StreamData) -> Result<(), ShellError> {
        match self {
            TypedStreamBuffer::List(buf) => match message {
                StreamData::List(value) => buf.push_back(value),
                StreamData::Raw(..) => Err(ShellError::PluginFailedToDecode {
                    msg: "Tried to send a raw stream's data to a list stream".into(),
                }),
            },
            TypedStreamBuffer::Raw(buf) => match message {
                StreamData::List(..) => Err(ShellError::PluginFailedToDecode {
                    msg: "Tried to send a list stream's data to a raw stream".into(),
                }),
                StreamData::Raw(bytes) => buf.push_back(bytes),
            },
        }
    }

    /// Pop a list value. Error if this is not a list stream.
    pub fn pop_list(&mut self) -> Result<Option<Option<Value>>, ShellError> {
        match self {
            TypedStreamBuffer::List(buf) => buf.pop_front(),
            _ => Err(ShellError::NushellFailed {
                msg: "tried to read list message from non-list stream".into(),
            }),
        }
    }

    /// Pop some raw bytes. Error if this is not an raw stream.
    pub fn pop_raw(&mut self) -> Result<Option<Option<Result<Vec<u8>, ShellError>>>, ShellError> {
        match self {
            TypedStreamBuffer::Raw(buf) => buf.pop_front(),
            _ => Err(ShellError::NushellFailed {
                msg: "tried to read raw message from non-raw stream".into(),
            }),
        }
    }

    /// End the list stream.
    pub fn end_list(&mut self) {
        match self {
            TypedStreamBuffer::List(buf) => buf.set_ended(),
            _ => (),
        }
    }

    /// End the raw stream.
    pub fn end_raw(&mut self) {
        match self {
            TypedStreamBuffer::Raw(buf) => buf.set_ended(),
            _ => (),
        }
    }

    /// Drop the list stream.
    pub fn drop_list(&mut self) {
        match self {
            TypedStreamBuffer::List(buf) => buf.set_dropped(),
            _ => (),
        }
    }

    /// Drop the raw stream.
    pub fn drop_raw(&mut self) {
        match self {
            TypedStreamBuffer::Raw(buf) => buf.set_dropped(),
            _ => (),
        }
    }

    /// True if the stream has been fully consumed.
    pub fn is_fully_consumed(&self) -> bool {
        match self {
            TypedStreamBuffer::List(buf) => buf.is_fully_consumed(),
            TypedStreamBuffer::Raw(buf) => buf.is_fully_consumed(),
        }
    }
}

/// A buffer for stream messages that need to be stored temporarily to allow a different stream
/// to be consumed.
///
/// The buffer is a FIFO queue.
#[derive(Debug)]
pub(crate) struct StreamBuffer<T> {
    /// Queue is `None` if the reader was dropped, and the messages are no longer desired.
    queue: Option<VecDeque<T>>,

    /// True if no more messages are expected
    ended: bool,
}

impl<T> StreamBuffer<T> {
    /// Returns a [StreamBuffer] with a new empty buffer.
    pub const fn new() -> StreamBuffer<T> {
        StreamBuffer {
            queue: Some(VecDeque::new()),
            ended: false,
        }
    }

    /// Push a message onto the back of the buffer.
    ///
    /// Discards the message if it is `Dropped`.
    pub fn push_back(&mut self, value: Option<T>) -> Result<(), ShellError> {
        if let Some(value) = value {
            if !self.ended {
                if let Some(ref mut queue) = self.queue {
                    queue.push_back(value);
                }
                Ok(())
            } else {
                Err(ShellError::PluginFailedToDecode {
                    msg: "Tried to write into a stream after it was closed".into(),
                })
            }
        } else {
            self.ended = true;
            Ok(())
        }
    }

    /// Try to pop a message from the front of the buffer.
    ///
    /// Returns `Ok(None)` if there are no messages waiting in the buffer, `Ok(Some(None))` at end
    /// of stream, or an error if this buffer is dropped.
    pub fn pop_front(&mut self) -> Result<Option<Option<T>>, ShellError> {
        if let Some(ref mut queue) = self.queue {
            if self.ended && queue.is_empty() {
                Ok(Some(None))
            } else {
                Ok(queue.pop_front().map(Some))
            }
        } else {
            Err(ShellError::PluginFailedToDecode {
                msg: "Tried to read from a stream that is already dropped".into(),
            })
        }
    }

    /// True if the buffer is present/dropped, has ended, and contains no messages.
    pub fn is_fully_consumed(&self) -> bool {
        self.ended && self.queue.as_ref().map(|q| q.is_empty()).unwrap_or(true)
    }

    /// True if the buffer is dropped.
    #[cfg(test)]
    pub fn is_dropped(&self) -> bool {
        self.queue.is_none()
    }

    /// Set the stream to ended
    pub fn set_ended(&mut self) {
        self.ended = true;
    }

    /// Set the stream to dropped
    pub fn set_dropped(&mut self) {
        self.queue = None;
    }
}
