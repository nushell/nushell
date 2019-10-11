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

pub(crate) use crate::cli::MaybeOwned;
pub(crate) use crate::commands::command::{
    CallInfo, CommandAction, CommandArgs, ReturnSuccess, ReturnValue, RunnableContext,
};
pub(crate) use crate::commands::PerItemCommand;
pub(crate) use crate::commands::RawCommandArgs;
pub(crate) use crate::context::CommandRegistry;
pub(crate) use crate::context::{AnchorLocation, Context};
pub(crate) use crate::data::base as value;
pub(crate) use crate::data::meta::{Tag, Tagged, TaggedItem};
pub(crate) use crate::data::types::ExtractType;
pub(crate) use crate::data::{Primitive, Value};
pub(crate) use crate::env::host::handle_unexpected;
pub(crate) use crate::env::Host;
pub(crate) use crate::errors::{CoerceInto, ShellError};
pub(crate) use crate::parser::hir::SyntaxShape;
pub(crate) use crate::parser::parse::parser::Number;
pub(crate) use crate::parser::registry::Signature;
pub(crate) use crate::shell::filesystem_shell::FilesystemShell;
pub(crate) use crate::shell::help_shell::HelpShell;
pub(crate) use crate::shell::shell_manager::ShellManager;
pub(crate) use crate::shell::value_shell::ValueShell;
pub(crate) use crate::stream::{InputStream, OutputStream};
pub(crate) use crate::traits::{HasTag, ToDebug};
pub(crate) use crate::Text;
pub(crate) use async_stream::stream as async_stream;
pub(crate) use bigdecimal::BigDecimal;
pub(crate) use futures::stream::BoxStream;
pub(crate) use futures::{FutureExt, Stream, StreamExt};
pub(crate) use num_bigint::BigInt;
pub(crate) use num_traits::cast::{FromPrimitive, ToPrimitive};
pub(crate) use num_traits::identities::Zero;
pub(crate) use serde::Deserialize;
pub(crate) use std::collections::VecDeque;
pub(crate) use std::future::Future;
pub(crate) use std::sync::{Arc, Mutex};

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
