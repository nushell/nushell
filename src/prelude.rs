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

            let objects = $expr.values.inspect(|o| {
                trace!(target: $target, "{} = {:#?}", $desc, o.debug());
            });

            $crate::stream::InputStream::from_stream(objects.boxed())
        } else {
            $expr
        }
    }};
}

#[macro_export]
macro_rules! trace_out_stream {
    (target: $target:tt, source: $source:expr, $desc:tt = $expr:expr) => {{
        if log::log_enabled!(target: $target, log::Level::Trace) {
            use futures::stream::StreamExt;

            let source = $source.clone();

            let objects = $expr.values.inspect(move |o| {
                trace!(target: $target, "{} = {}", $desc, o.debug(&source));
            });

            $crate::stream::OutputStream::new(objects)
        } else {
            $expr
        }
    }};
}

crate use crate::cli::MaybeOwned;
crate use crate::commands::command::{
    CallInfo, CommandAction, CommandArgs, ReturnSuccess, ReturnValue, RunnableContext,
};
crate use crate::commands::{PerItemCommand, RawCommandArgs};
crate use crate::context::CommandRegistry;
crate use crate::context::{Context, SpanSource};
crate use crate::env::host::handle_unexpected;
crate use crate::env::Host;
crate use crate::errors::ShellError;
crate use crate::object::base as value;
crate use crate::object::meta::{Tag, Tagged, TaggedItem};
crate use crate::object::types::ExtractType;
crate use crate::object::{Primitive, Value};
crate use crate::parser::hir::SyntaxType;
crate use crate::parser::registry::Signature;
crate use crate::shell::filesystem_shell::FilesystemShell;
crate use crate::shell::shell_manager::ShellManager;
crate use crate::shell::value_shell::ValueShell;
crate use crate::stream::{InputStream, OutputStream};
crate use crate::traits::{HasSpan, ToDebug};
crate use crate::Span;
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
    T: Stream<Item = Tagged<Value>> + Send + 'static,
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
