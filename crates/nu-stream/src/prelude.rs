#[macro_export]
macro_rules! return_err {
    ($expr:expr) => {
        match $expr {
            Err(_) => return,
            Ok(expr) => expr,
        };
    };
}

#[macro_export]
macro_rules! trace_out_stream {
    (target: $target:tt, $desc:tt = $expr:expr) => {{
        if log::log_enabled!(target: $target, log::Level::Trace) {
            let objects = $expr.inspect(move |o| {
                trace!(
                    target: $target,
                    "{} = {}",
                    $desc,
                    match o {
                        Err(err) => format!("{:?}", err),
                        Ok(value) => value.display(),
                    }
                );
            });

            $crate::stream::OutputStream::new(objects)
        } else {
            $expr
        }
    }};
}

pub(crate) use std::collections::VecDeque;
pub(crate) use std::sync::Arc;

use nu_protocol::Value;

pub(crate) use crate::{ActionStream, InputStream, OutputStream};

#[allow(clippy::wrong_self_convention)]
pub trait ToOutputStream {
    fn to_output_stream(self) -> OutputStream;
}

impl<T> ToOutputStream for T
where
    T: Iterator<Item = Value> + Send + Sync + 'static,
{
    fn to_output_stream(self) -> OutputStream {
        OutputStream::from_stream(self)
    }
}

#[allow(clippy::wrong_self_convention)]
pub trait ToOutputStreamWithActions {
    fn to_output_stream_with_actions(self) -> ActionStream;
}

impl<T, U> ToOutputStreamWithActions for T
where
    T: Iterator<Item = U> + Send + Sync + 'static,
    U: Into<nu_protocol::ReturnValue>,
{
    fn to_output_stream_with_actions(self) -> ActionStream {
        ActionStream {
            values: Box::new(self.map(|item| item.into())),
        }
    }
}
