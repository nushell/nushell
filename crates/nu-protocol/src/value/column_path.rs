use crate::Value;
use derive_new::new;
use getset::Getters;
use nu_source::{b, span_for_spanned_list, DebugDocBuilder, HasFallibleSpan, PrettyDebug, Span};
use num_bigint::BigInt;
use serde::{Deserialize, Serialize};

/// A PathMember that has yet to be spanned so that it can be used in later processing
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum UnspannedPathMember {
    String(String),
    Int(BigInt),
}

impl UnspannedPathMember {
    /// Add the span information and get a full PathMember
    pub fn into_path_member(self, span: impl Into<Span>) -> PathMember {
        PathMember {
            unspanned: self,
            span: span.into(),
        }
    }
}

/// A basic piece of a ColumnPath, which describes the steps to take through a table to arrive a cell, row, or inner table
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct PathMember {
    pub unspanned: UnspannedPathMember,
    pub span: Span,
}

impl PrettyDebug for &PathMember {
    /// Gets the PathMember ready to be pretty-printed
    fn pretty(&self) -> DebugDocBuilder {
        match &self.unspanned {
            UnspannedPathMember::String(string) => b::primitive(format!("{:?}", string)),
            UnspannedPathMember::Int(int) => b::primitive(format!("{}", int)),
        }
    }
}

/// The fundamental path primitive to descrive how to navigate through a table to get to a sub-item. A path member can be either a word or a number. Words/strings are taken to mean
/// a column name, and numbers are the row number. Taken together they describe which column or row to narrow to in order to get data.
///
/// Rows must follow column names, they can't come first. eg) `foo.1` is valid where `1.foo` is not.
#[derive(
    Debug, Hash, Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Getters, Clone, new,
)]
pub struct ColumnPath {
    #[get = "pub"]
    members: Vec<PathMember>,
}

impl ColumnPath {
    /// Iterate over the members of the column path
    pub fn iter(&self) -> impl Iterator<Item = &PathMember> {
        self.members.iter()
    }

    /// Returns the last member and a slice of the remaining members
    pub fn split_last(&self) -> Option<(&PathMember, &[PathMember])> {
        self.members.split_last()
    }

    /// Returns the last member
    pub fn last(&self) -> Option<&PathMember> {
        self.iter().last()
    }
}

impl PrettyDebug for ColumnPath {
    /// Gets the ColumnPath ready to be pretty-printed
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
    /// Creates a span that will cover the column path, if possible
    fn maybe_span(&self) -> Option<Span> {
        if self.members.is_empty() {
            None
        } else {
            Some(span_for_spanned_list(self.members.iter().map(|m| m.span)))
        }
    }
}

impl PathMember {
    /// Create a string path member
    pub fn string(string: impl Into<String>, span: impl Into<Span>) -> PathMember {
        UnspannedPathMember::String(string.into()).into_path_member(span)
    }

    /// Create a numeric path member
    pub fn int(int: impl Into<BigInt>, span: impl Into<Span>) -> PathMember {
        UnspannedPathMember::Int(int.into()).into_path_member(span)
    }

    pub fn as_string(&self) -> String {
        match &self.unspanned {
            UnspannedPathMember::String(string) => string.clone(),
            UnspannedPathMember::Int(int) => format!("{}", int),
        }
    }
}

/// Prepares a list of "sounds like" matches for the string you're trying to find
pub fn did_you_mean(obj_source: &Value, field_tried: &PathMember) -> Option<Vec<(usize, String)>> {
    let field_tried = match &field_tried.unspanned {
        UnspannedPathMember::String(string) => string.clone(),
        UnspannedPathMember::Int(int) => format!("{}", int),
    };

    let possibilities = obj_source.data_descriptors();

    let mut possible_matches: Vec<_> = possibilities
        .into_iter()
        .map(|x| {
            let word = x;
            let distance = natural::distance::levenshtein_distance(&word, &field_tried);

            (distance, word)
        })
        .collect();

    if !possible_matches.is_empty() {
        possible_matches.sort();
        Some(possible_matches)
    } else {
        None
    }
}
