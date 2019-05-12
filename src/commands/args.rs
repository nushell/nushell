use crate::object::Value;
use derive_new::new;
use std::cell::Cell;
use std::collections::VecDeque;

#[derive(Debug, Default)]
pub struct ObjectStream {
    queue: VecDeque<Value>,
}

pub struct Streams {
    success: Cell<ObjectStream>,
    // error: ObjectStream,
    // warning: ObjectStream,
    // debug: ObjectStream,
    // trace: ObjectStream,
    // verbose: ObjectStream,
}

impl std::fmt::Debug for Streams {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Streams")
    }
}

impl Streams {
    crate fn new() -> Streams {
        Streams {
            success: Cell::new(ObjectStream::default()),
        }
    }

    crate fn take_success(&mut self) -> Cell<ObjectStream> {
        let new_stream = Cell::new(ObjectStream::default());
        self.success.swap(&new_stream);
        new_stream
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
