use crate::*;
use std::{
    fmt::Debug,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

/// A single buffer of binary data streamed over multiple parts. Optionally contains ctrl-c that can be used
/// to break the stream.
pub struct ByteStream {
    pub stream: Box<dyn Iterator<Item = Result<Vec<u8>, ShellError>> + Send + 'static>,
    pub ctrlc: Option<Arc<AtomicBool>>,
}
impl ByteStream {
    pub fn into_vec(self) -> Result<Vec<u8>, ShellError> {
        let mut output = vec![];
        for item in self.stream {
            output.append(&mut item?);
        }

        Ok(output)
    }
}
impl Debug for ByteStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ByteStream").finish()
    }
}

impl Iterator for ByteStream {
    type Item = Result<Vec<u8>, ShellError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(ctrlc) = &self.ctrlc {
            if ctrlc.load(Ordering::SeqCst) {
                None
            } else {
                self.stream.next()
            }
        } else {
            self.stream.next()
        }
    }
}

/// A single string streamed over multiple parts. Optionally contains ctrl-c that can be used
/// to break the stream.
pub struct StringStream {
    pub stream: Box<dyn Iterator<Item = Result<String, ShellError>> + Send + 'static>,
    pub ctrlc: Option<Arc<AtomicBool>>,
}
impl StringStream {
    pub fn into_string(self, separator: &str) -> Result<String, ShellError> {
        let mut output = String::new();

        let mut first = true;
        for s in self.stream {
            output.push_str(&s?);

            if !first {
                output.push_str(separator);
            } else {
                first = false;
            }
        }
        Ok(output)
    }

    pub fn from_stream(
        input: impl Iterator<Item = Result<String, ShellError>> + Send + 'static,
        ctrlc: Option<Arc<AtomicBool>>,
    ) -> StringStream {
        StringStream {
            stream: Box::new(input),
            ctrlc,
        }
    }
}
impl Debug for StringStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StringStream").finish()
    }
}

impl Iterator for StringStream {
    type Item = Result<String, ShellError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(ctrlc) = &self.ctrlc {
            if ctrlc.load(Ordering::SeqCst) {
                None
            } else {
                self.stream.next()
            }
        } else {
            self.stream.next()
        }
    }
}

/// A potentially infinite stream of values, optinally with a mean to send a Ctrl-C signal to stop
/// the stream from continuing.
///
/// In practice, a "stream" here means anything which can be iterated and produce Values as it iterates.
/// Like other iterators in Rust, observing values from this stream will drain the items as you view them
/// and the stream cannot be replayed.
pub struct ValueStream {
    pub stream: Box<dyn Iterator<Item = Value> + Send + 'static>,
    pub ctrlc: Option<Arc<AtomicBool>>,
}

impl ValueStream {
    pub fn into_string(self, separator: &str, config: &Config) -> String {
        self.map(|x: Value| x.into_string(", ", config))
            .collect::<Vec<String>>()
            .join(separator)
    }

    pub fn from_stream(
        input: impl Iterator<Item = Value> + Send + 'static,
        ctrlc: Option<Arc<AtomicBool>>,
    ) -> ValueStream {
        ValueStream {
            stream: Box::new(input),
            ctrlc,
        }
    }
}

impl Debug for ValueStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValueStream").finish()
    }
}

impl Iterator for ValueStream {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(ctrlc) = &self.ctrlc {
            if ctrlc.load(Ordering::SeqCst) {
                None
            } else {
                self.stream.next()
            }
        } else {
            self.stream.next()
        }
    }
}
