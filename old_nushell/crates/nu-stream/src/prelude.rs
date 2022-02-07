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

pub trait IntoOutputStream {
    fn into_output_stream(self) -> OutputStream;
}

impl<T> IntoOutputStream for T
where
    T: Iterator<Item = Value> + Send + Sync + 'static,
{
    fn into_output_stream(self) -> OutputStream {
        OutputStream::from_stream(self)
    }
}

pub trait IntoActionStream {
    fn into_action_stream(self) -> ActionStream;
}

impl<T, U> IntoActionStream for T
where
    T: Iterator<Item = U> + Send + Sync + 'static,
    U: Into<nu_protocol::ReturnValue>,
{
    fn into_action_stream(self) -> ActionStream {
        ActionStream {
            values: Box::new(self.map(|item| item.into())),
        }
    }
}
