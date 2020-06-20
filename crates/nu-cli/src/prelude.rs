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
macro_rules! trace_stream {
    (target: $target:tt, $desc:tt = $expr:expr) => {{
        if log::log_enabled!(target: $target, log::Level::Trace) {
            use futures::stream::StreamExt;

            let objects = $expr.inspect(move |o| {
                trace!(
                    target: $target,
                    "{} = {}",
                    $desc,
                    nu_source::PrettyDebug::plain_string(o, 70)
                );
            });

            $crate::stream::InputStream::from_stream(objects.boxed())
        } else {
            $expr
        }
    }};
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

            $crate::stream::OutputStream::new(objects)
        } else {
            $expr
        }
    }};
}

pub(crate) use nu_protocol::{errln, out, outln};
use nu_source::HasFallibleSpan;

pub(crate) use crate::commands::command::{CommandArgs, RawCommandArgs, RunnableContext};
pub(crate) use crate::commands::Example;
pub(crate) use crate::context::CommandRegistry;
pub(crate) use crate::context::Context;
pub(crate) use crate::data::config;
pub(crate) use crate::data::value;
// pub(crate) use crate::env::host::handle_unexpected;
pub(crate) use crate::env::Host;
pub(crate) use crate::shell::filesystem_shell::FilesystemShell;
pub(crate) use crate::shell::help_shell::HelpShell;
pub(crate) use crate::shell::shell_manager::ShellManager;
pub(crate) use crate::shell::value_shell::ValueShell;
pub(crate) use crate::stream::{InputStream, InterruptibleStream, OutputStream};
pub(crate) use bigdecimal::BigDecimal;
pub(crate) use futures::stream::BoxStream;
pub(crate) use futures::{Stream, StreamExt};
pub(crate) use nu_protocol::MaybeOwned;
pub(crate) use nu_source::{
    b, AnchorLocation, DebugDocBuilder, PrettyDebug, PrettyDebugWithSource, Span, SpannedItem, Tag,
    TaggedItem, Text,
};
pub(crate) use nu_value_ext::ValueExt;
pub(crate) use num_bigint::BigInt;
pub(crate) use num_traits::cast::ToPrimitive;
pub(crate) use serde::Deserialize;
pub(crate) use std::collections::VecDeque;
pub(crate) use std::future::Future;
pub(crate) use std::sync::atomic::AtomicBool;
pub(crate) use std::sync::Arc;

pub(crate) use async_trait::async_trait;
pub(crate) use indexmap::IndexMap;
pub(crate) use itertools::Itertools;

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
            Err(err) => match HasFallibleSpan::maybe_span(&err) {
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
