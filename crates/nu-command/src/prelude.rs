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
pub(crate) use nu_engine::RawCommandArgs;
pub(crate) use nu_engine::{get_full_help, CommandArgs, Scope, WholeStreamCommand};
pub(crate) use nu_engine::{RunnableContext, RunnableContextWithoutInput};
pub(crate) use nu_parser::ParserScope;
pub(crate) use nu_protocol::{out, row};
pub(crate) use nu_source::{AnchorLocation, PrettyDebug, Span, SpannedItem, Tag, TaggedItem};
pub(crate) use nu_stream::{ActionStream, InputStream, Interruptible, OutputStream};
pub(crate) use nu_stream::{ToActionStream, ToInputStream, ToOutputStream};
pub(crate) use nu_value_ext::ValueExt;
pub(crate) use num_bigint::BigInt;
pub(crate) use num_traits::cast::ToPrimitive;
pub(crate) use serde::Deserialize;
pub(crate) use std::collections::VecDeque;
pub(crate) use std::sync::atomic::AtomicBool;
pub(crate) use std::sync::Arc;

#[allow(clippy::wrong_self_convention)]
pub trait FromInputStream {
    fn from_input_stream(self) -> ActionStream;
}

impl<T> FromInputStream for T
where
    T: Iterator<Item = nu_protocol::Value> + Send + Sync + 'static,
{
    fn from_input_stream(self) -> ActionStream {
        ActionStream {
            values: Box::new(self.map(nu_protocol::ReturnSuccess::value)),
        }
    }
}

// #[allow(clippy::wrong_self_convention)]
// pub trait ToOutputStream {
//     fn to_output_stream(self) -> OutputStream;
// }

// impl<T, U> ToOutputStream for T
// where
//     T: Iterator<Item = U> + Send + Sync + 'static,
//     U: Into<nu_protocol::ReturnValue>,
// {
//     fn to_output_stream(self) -> OutputStream {
//         OutputStream::from_stream(self)
//     }
// }

// #[allow(clippy::wrong_self_convention)]
// pub trait ToOutputStreamWithAction {
//     fn to_action_stream(self) -> ActionStream;
// }

// impl<T, U> ToOutputStreamWithAction for T
// where
//     T: Iterator<Item = U> + Send + Sync + 'static,
//     U: Into<nu_protocol::ReturnValue>,
// {
//     fn to_action_stream(self) -> ActionStream {
//         ActionStream {
//             values: Box::new(self.map(|item| item.into())),
//         }
//     }
// }
