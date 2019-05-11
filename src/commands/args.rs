use crate::object::Value;
use derive_new::new;
use std::collections::VecDeque;

#[derive(Debug, Default)]
pub struct ObjectStream {
    queue: VecDeque<Value>,
}

#[derive(Debug, Default)]
pub struct Streams {
    success: ObjectStream,
    error: ObjectStream,
    warning: ObjectStream,
    debug: ObjectStream,
    trace: ObjectStream,
    verbose: ObjectStream,
}

#[derive(Debug, new)]
pub struct Args {
    argv: Vec<Value>,
    #[new(default)]
    streams: Streams,
}

impl Args {
    crate fn first(&self) -> Option<&Value> {
        self.argv.first()
    }
}
