use crate::*;
use std::{
    fmt::Debug,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

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
    pub fn into_string(self) -> String {
        format!(
            "[{}]",
            self.map(|x: Value| x.into_string())
                .collect::<Vec<String>>()
                .join(", ")
        )
    }

    pub fn collect_string(self) -> String {
        self.map(|x: Value| x.collect_string())
            .collect::<Vec<String>>()
            .join("\n")
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
