use crate::hir::Expression;
use derive_new::new;
use getset::{Getters, MutGetters};
use nu_protocol::PathMember;
use nu_source::{b, DebugDocBuilder, PrettyDebug, PrettyDebugWithSource};
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Getters,
    MutGetters,
    Serialize,
    Deserialize,
    new,
)]
#[get = "pub"]
pub struct Path {
    head: Expression,
    #[get_mut = "pub(crate)"]
    tail: Vec<PathMember>,
}

impl PrettyDebugWithSource for Path {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        self.head.pretty_debug(source)
            + b::operator(".")
            + b::intersperse(self.tail.iter().map(|m| m.pretty()), b::operator("."))
    }
}

impl Path {
    pub(crate) fn parts(self) -> (Expression, Vec<PathMember>) {
        (self.head, self.tail)
    }
}
