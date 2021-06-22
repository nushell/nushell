#[macro_export]
macro_rules! return_err {
    ($expr:expr) => {
        match $expr {
            Err(_) => return,
            Ok(expr) => expr,
        };
    };
}

pub(crate) use bigdecimal::BigDecimal;
pub(crate) use indexmap::{indexmap, IndexMap};
pub(crate) use itertools::Itertools;
pub(crate) use nu_data::config;
pub(crate) use nu_data::value;
pub(crate) use nu_engine::EvaluationContext;
pub(crate) use nu_engine::Example;
pub(crate) use nu_engine::Host;
pub(crate) use nu_engine::RunnableContext;
pub(crate) use nu_engine::{get_full_help, CommandArgs, Scope, WholeStreamCommand};
pub(crate) use nu_parser::ParserScope;
pub(crate) use nu_protocol::{out, row};
pub(crate) use nu_source::{AnchorLocation, PrettyDebug, Span, SpannedItem, Tag, TaggedItem};
pub(crate) use nu_stream::{ActionStream, InputStream, Interruptible, OutputStream};
pub(crate) use nu_stream::{IntoActionStream, IntoInputStream, IntoOutputStream};
pub(crate) use nu_value_ext::ValueExt;
pub(crate) use num_bigint::BigInt;
pub(crate) use num_traits::cast::ToPrimitive;
pub(crate) use serde::Deserialize;
pub(crate) use std::collections::VecDeque;
pub(crate) use std::sync::atomic::AtomicBool;
pub(crate) use std::sync::Arc;
