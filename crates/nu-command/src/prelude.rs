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

pub(crate) use crate::commands::command::RunnableContext;
pub(crate) use async_trait::async_trait;
pub(crate) use bigdecimal::BigDecimal;
pub(crate) use futures::{Stream, StreamExt};
pub(crate) use indexmap::{indexmap, IndexMap};
pub(crate) use itertools::Itertools;
pub(crate) use nu_data::config;
pub(crate) use nu_data::value;
pub(crate) use nu_engine::EvaluationContext;
pub(crate) use nu_engine::Example;
pub(crate) use nu_engine::Host;
pub(crate) use nu_engine::RawCommandArgs;
pub(crate) use nu_engine::ShellManager;
pub(crate) use nu_engine::{get_help, CommandArgs, Scope, WholeStreamCommand};
pub(crate) use nu_parser::ParserScope;
pub(crate) use nu_protocol::{out, row};
pub(crate) use nu_source::{AnchorLocation, PrettyDebug, Span, SpannedItem, Tag, TaggedItem, Text};
pub(crate) use nu_stream::ToInputStream;
pub(crate) use nu_stream::{InputStream, Interruptible, OutputStream};
pub(crate) use nu_value_ext::ValueExt;
pub(crate) use num_bigint::BigInt;
pub(crate) use num_traits::cast::ToPrimitive;
pub(crate) use serde::Deserialize;
pub(crate) use std::collections::VecDeque;
pub(crate) use std::future::Future;
pub(crate) use std::sync::atomic::{AtomicBool, Ordering};
pub(crate) use std::sync::Arc;

#[allow(clippy::wrong_self_convention)]
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

#[allow(clippy::wrong_self_convention)]
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
