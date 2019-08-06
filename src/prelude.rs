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
macro_rules! trace_stream {
    (target: $target:tt, $desc:tt = $expr:expr) => {{
        if log::log_enabled!(target: $target, log::Level::Trace) {
            use futures::stream::StreamExt;
            // Blocking is generally quite bad, but this is for debugging
            // let mut local = futures::executor::LocalPool::new();
            // let objects = local.run_until($expr.into_vec());
            // let objects: Vec<_> = futures::executor::block_on($expr.into_vec());

            let objects = $expr.values.inspect(|o| {
                trace!(target: $target, "{} = {:#?}", $desc, o.debug());
            });

            $crate::stream::InputStream::from_stream(objects.boxed())
        } else {
            $expr
        }
    }};
}

crate use crate::cli::MaybeOwned;
crate use crate::commands::command::{
    CommandAction, CommandArgs, ReturnSuccess, ReturnValue, RunnableContext,
};
crate use crate::context::{CommandRegistry, Context};
crate use crate::env::host::handle_unexpected;
crate use crate::env::{Environment, Host};
crate use crate::errors::ShellError;
crate use crate::object::base as value;
crate use crate::object::types::ExtractType;
crate use crate::object::{Primitive, Value};
crate use crate::parser::registry::Signature;
crate use crate::parser::{hir::SyntaxType, Span, Spanned, SpannedItem};
crate use crate::stream::{InputStream, OutputStream};
crate use crate::traits::{HasSpan, ToDebug};
crate use crate::Text;
crate use futures::stream::BoxStream;
crate use futures::{FutureExt, Stream, StreamExt};
crate use futures_async_stream::async_stream_block;
#[allow(unused)]
crate use serde::{Deserialize, Serialize};
crate use std::collections::VecDeque;
crate use std::future::Future;
crate use std::sync::{Arc, Mutex};

pub trait FromInputStream {
    fn from_input_stream(self) -> OutputStream;
}

impl<T> FromInputStream for T
where
    T: Stream<Item = Spanned<Value>> + Send + 'static,
{
    fn from_input_stream(self) -> OutputStream {
        OutputStream {
            values: self.map(ReturnSuccess::value).boxed(),
        }
    }
}

pub trait ToOutputStream {
    fn to_output_stream(self) -> OutputStream;
}

impl<T, U> ToOutputStream for T
where
    T: Stream<Item = U> + Send + 'static,
    U: Into<ReturnValue>,
{
    fn to_output_stream(self) -> OutputStream {
        OutputStream {
            values: self.map(|item| item.into()).boxed(),
        }
    }
}
