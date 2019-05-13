use crate::object::Value;
use crate::ShellError;
use derive_new::new;
use std::cell::Cell;
use std::collections::VecDeque;

#[derive(Debug)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

#[derive(Debug)]
pub struct LogItem {
    level: LogLevel,
    value: Value,
}

#[derive(Debug, Default)]
pub struct ObjectStream<T> {
    queue: VecDeque<T>,
}

impl<T> ObjectStream<T> {
    crate fn empty() -> ObjectStream<T> {
        ObjectStream {
            queue: VecDeque::new(),
        }
    }

    crate fn iter(&self) -> impl Iterator<Item = &T> {
        self.queue.iter()
    }

    crate fn take(&mut self) -> Option<T> {
        self.queue.pop_front()
    }

    crate fn add(&mut self, value: T) {
        self.queue.push_back(value);
    }
}

#[derive(new)]
pub struct Streams {
    #[new(value = "ObjectStream::empty()")]
    success: ObjectStream<Value>,

    #[new(value = "ObjectStream::empty()")]
    errors: ObjectStream<ShellError>,

    #[new(value = "ObjectStream::empty()")]
    log: ObjectStream<LogItem>,
}

impl std::fmt::Debug for Streams {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Streams")
    }
}

impl Streams {
    crate fn read(&mut self) -> Option<Value> {
        self.success.take()
    }

    crate fn add(&mut self, value: Value) {
        self.success.add(value);
    }

    // fn take_stream(&mut self, stream: &mut ObjectStream) -> ObjectStream {
    //     let mut new_stream = Cell::new(ObjectStream::default());
    //     new_stream.swap()
    //     std::mem::swap(stream, &mut new_stream);
    //     new_stream
    // }
}

#[derive(Debug, new)]
pub struct Args {
    argv: Vec<Value>,
    #[new(value = "Streams::new()")]
    streams: Streams,
}

impl Args {
    crate fn first(&self) -> Option<&Value> {
        self.argv.first()
    }
}
