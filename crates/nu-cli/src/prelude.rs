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
macro_rules! stream {
    ($($expr:expr),*) => {{
        let mut v = VecDeque::new();

        $(
            v.push_back($expr);
        )*

        v
    }}
}

#[macro_export]
macro_rules! trace_out_stream {
    (target: $target:tt, $desc:tt = $expr:expr) => {{
        if log::log_enabled!(target: $target, log::Level::Trace) {
            use futures::stream::StreamExt;

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

            nu_stream::OutputStream::new(objects)
        } else {
            $expr
        }
    }};
}

pub(crate) use futures::{Stream, StreamExt};
pub(crate) use nu_engine::Host;
#[allow(unused_imports)]
pub(crate) use nu_errors::ShellError;
#[allow(unused_imports)]
pub(crate) use nu_protocol::outln;
pub(crate) use nu_stream::OutputStream;
#[allow(unused_imports)]
pub(crate) use nu_value_ext::ValueExt;
#[allow(unused_imports)]
pub(crate) use std::sync::atomic::Ordering;

#[allow(clippy::clippy::wrong_self_convention)]
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

#[allow(clippy::clippy::wrong_self_convention)]
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
