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

            nu_streams::InputStream::from_stream(objects.boxed())
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

            nu_streams::OutputStream::new(objects)
        } else {
            $expr
        }
    }};
}

pub(crate) use crate::commands::command::{CommandArgs, RawCommandArgs, RunnableContext};
pub(crate) use crate::commands::Example;
pub(crate) use crate::context::CommandRegistry;
pub(crate) use crate::context::Context;
pub(crate) use crate::data::config;
pub(crate) use crate::data::value;
pub(crate) use crate::env::host::handle_unexpected;
pub(crate) use crate::env::Host;
pub(crate) use crate::shell::filesystem_shell::FilesystemShell;
pub(crate) use crate::shell::help_shell::HelpShell;
pub(crate) use crate::shell::shell_manager::ShellManager;
pub(crate) use crate::shell::value_shell::ValueShell;

pub(crate) use nu_protocol::{errln, out, outln, MaybeOwned};
pub(crate) use nu_source::{
    b, AnchorLocation, DebugDocBuilder, PrettyDebug, PrettyDebugWithSource, Span, SpannedItem, Tag,
    TaggedItem, Text,
};
pub(crate) use nu_streams::{
    InputStream, Interruptible, OutputStream, ToInputStream, ToOutputStream,
};
pub(crate) use nu_value_ext::ValueExt;

pub(crate) use async_stream::stream as async_stream;
pub(crate) use bigdecimal::BigDecimal;
pub(crate) use futures::stream::BoxStream;
pub(crate) use futures::StreamExt;
pub(crate) use itertools::Itertools;
pub(crate) use num_bigint::BigInt;
pub(crate) use num_traits::cast::ToPrimitive;
pub(crate) use serde::Deserialize;
pub(crate) use std::collections::VecDeque;
pub(crate) use std::sync::atomic::AtomicBool;
pub(crate) use std::sync::Arc;
