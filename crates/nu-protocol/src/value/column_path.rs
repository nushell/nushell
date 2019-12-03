use derive_new::new;
use getset::Getters;
use nu_source::{b, span_for_spanned_list, DebugDocBuilder, HasFallibleSpan, PrettyDebug, Span};
use num_bigint::BigInt;
use serde::{Deserialize, Serialize};

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

impl PathMember {
    pub fn string(string: impl Into<String>, span: impl Into<Span>) -> PathMember {
        UnspannedPathMember::String(string.into()).into_path_member(span)
    }

    pub fn int(int: impl Into<BigInt>, span: impl Into<Span>) -> PathMember {
        UnspannedPathMember::Int(int.into()).into_path_member(span)
    }
}
