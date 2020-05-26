use futures::stream::{Stream, StreamExt};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::input::InputStream;
use crate::interruptible::InterruptibleStream;
use crate::output::OutputStream;

pub trait FromInputStream {
    fn from_input_stream(self) -> OutputStream;
}

impl<T> FromInputStream for T
where
    T: Stream<Item = nu_protocol::Value> + Send + 'static,
{
    fn from_input_stream(self) -> OutputStream {
        OutputStream {
            values: self.map(nu_protocol::ReturnSuccess::value).boxed(),
        }
    }
}

pub trait ToInputStream {
    fn to_input_stream(self) -> InputStream;
}

impl<T, U> ToInputStream for T
where
    T: Stream<Item = U> + Send + 'static,
    U: Into<Result<nu_protocol::Value, nu_errors::ShellError>>,
{
    fn to_input_stream(self) -> InputStream {
        InputStream::from_stream(self.map(|item| match item.into() {
            Ok(result) => result,
            Err(err) => match nu_source::HasFallibleSpan::maybe_span(&err) {
                Some(span) => nu_protocol::UntaggedValue::Error(err).into_value(span),
                None => nu_protocol::UntaggedValue::Error(err).into_untagged_value(),
            },
        }))
    }
}

pub trait ToOutputStream {
    fn to_output_stream(self) -> OutputStream;
}

impl<T, U> ToOutputStream for T
where
    T: Stream<Item = U> + Send + 'static,
    U: Into<nu_protocol::ReturnValue>,
{
    fn to_output_stream(self) -> OutputStream {
        OutputStream {
            values: self.map(|item| item.into()).boxed(),
        }
    }
}

pub trait Interruptible<V> {
    fn interruptible(self, ctrl_c: Arc<AtomicBool>) -> InterruptibleStream<V>;
}

impl<S, V> Interruptible<V> for S
where
    S: Stream<Item = V> + Send + 'static,
{
    fn interruptible(self, ctrl_c: Arc<AtomicBool>) -> InterruptibleStream<V> {
        InterruptibleStream::new(self, ctrl_c)
    }
}
