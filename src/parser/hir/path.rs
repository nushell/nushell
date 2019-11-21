use crate::parser::hir::Expression;
use crate::prelude::*;
use derive_new::new;
use getset::{Getters, MutGetters};
use nu_source::{b, span_for_spanned_list, PrettyDebug};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum UnspannedPathMember {
    String(String),
    Int(BigInt),
}

impl UnspannedPathMember {
    pub fn into_path_member(self, span: impl Into<Span>) -> PathMember {
        PathMember {
            unspanned: self,
            span: span.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct PathMember {
    pub unspanned: UnspannedPathMember,
    pub span: Span,
}

impl PrettyDebug for &PathMember {
    fn pretty(&self) -> DebugDocBuilder {
        match &self.unspanned {
            UnspannedPathMember::String(string) => b::primitive(format!("{:?}", string)),
            UnspannedPathMember::Int(int) => b::primitive(format!("{}", int)),
        }
    }
}

#[derive(
    Debug, Hash, Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Getters, Clone, new,
)]
pub struct ColumnPath {
    #[get = "pub"]
    members: Vec<PathMember>,
}

impl ColumnPath {
    pub fn iter(&self) -> impl Iterator<Item = &PathMember> {
        self.members.iter()
    }

    pub fn split_last(&self) -> (&PathMember, &[PathMember]) {
        self.members.split_last().unwrap()
    }
}

impl PrettyDebug for ColumnPath {
    fn pretty(&self) -> DebugDocBuilder {
        let members: Vec<DebugDocBuilder> =
            self.members.iter().map(|member| member.pretty()).collect();

        b::delimit(
            "(",
            b::description("path") + b::equals() + b::intersperse(members, b::space()),
            ")",
        )
        .nest()
    }
}

impl HasFallibleSpan for ColumnPath {
    fn maybe_span(&self) -> Option<Span> {
        if self.members.len() == 0 {
            None
        } else {
            Some(span_for_spanned_list(self.members.iter().map(|m| m.span)))
        }
    }
}

impl fmt::Display for UnspannedPathMember {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnspannedPathMember::String(string) => write!(f, "{}", string),
            UnspannedPathMember::Int(int) => write!(f, "{}", int),
        }
    }
}

impl PathMember {
    pub fn string(string: impl Into<String>, span: impl Into<Span>) -> PathMember {
        UnspannedPathMember::String(string.into()).into_path_member(span)
    }

    pub fn int(int: impl Into<BigInt>, span: impl Into<Span>) -> PathMember {
        UnspannedPathMember::Int(int.into()).into_path_member(span)
    }
}

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
#[get = "pub(crate)"]
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

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.head)?;

        for entry in &self.tail {
            write!(f, ".{}", entry.unspanned)?;
        }

        Ok(())
    }
}

impl Path {
    pub(crate) fn parts(self) -> (Expression, Vec<PathMember>) {
        (self.head, self.tail)
    }
}
