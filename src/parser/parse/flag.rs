use crate::Span;
use derive_new::new;
use getset::Getters;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum FlagKind {
    Shorthand,
    Longhand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Getters, new)]
#[get = "pub(crate)"]
pub struct Flag {
    kind: FlagKind,
    name: Span,
}
