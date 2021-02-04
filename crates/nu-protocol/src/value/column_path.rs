use derive_new::new;
use getset::Getters;
use nu_source::{
    span_for_spanned_list, DbgDocBldr, DebugDocBuilder, HasFallibleSpan, PrettyDebug, Span,
    Spanned, SpannedItem,
};
use num_bigint::BigInt;
use serde::{Deserialize, Serialize};

use crate::hir::{Expression, Literal, Member, SpannedExpression};
use nu_errors::ParseError;

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
            UnspannedPathMember::String(string) => DbgDocBldr::primitive(format!("{:?}", string)),
            UnspannedPathMember::Int(int) => DbgDocBldr::primitive(format!("{}", int)),
        }
    }
}

/// The fundamental path primitive to describe how to navigate through a table to get to a sub-item. A path member can be either a word or a number. Words/strings are taken to mean
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

    pub fn build(text: &Spanned<String>) -> ColumnPath {
        if let (
            SpannedExpression {
                expr: Expression::Literal(Literal::ColumnPath(path)),
                span: _,
            },
            _,
        ) = parse(&text)
        {
            ColumnPath {
                members: path.iter().map(|member| member.to_path_member()).collect(),
            }
        } else {
            ColumnPath { members: vec![] }
        }
    }
}

impl PrettyDebug for ColumnPath {
    /// Gets the ColumnPath ready to be pretty-printed
    fn pretty(&self) -> DebugDocBuilder {
        let members: Vec<DebugDocBuilder> =
            self.members.iter().map(|member| member.pretty()).collect();

        DbgDocBldr::delimit(
            "(",
            DbgDocBldr::description("path")
                + DbgDocBldr::equals()
                + DbgDocBldr::intersperse(members, DbgDocBldr::space()),
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

fn parse(raw_column_path: &Spanned<String>) -> (SpannedExpression, Option<ParseError>) {
    let mut delimiter = '.';
    let mut inside_delimiter = false;
    let mut output = vec![];
    let mut current_part = String::new();
    let mut start_index = 0;
    let mut last_index = 0;

    for (idx, c) in raw_column_path.item.char_indices() {
        last_index = idx;
        if inside_delimiter {
            if c == delimiter {
                inside_delimiter = false;
            }
        } else if c == '\'' || c == '"' || c == '`' {
            inside_delimiter = true;
            delimiter = c;
        } else if c == '.' {
            let part_span = Span::new(
                raw_column_path.span.start() + start_index,
                raw_column_path.span.start() + idx,
            );

            if let Ok(row_number) = current_part.parse::<u64>() {
                output.push(Member::Int(BigInt::from(row_number), part_span));
            } else {
                let trimmed = trim_quotes(&current_part);
                output.push(Member::Bare(trimmed.clone().spanned(part_span)));
            }
            current_part.clear();
            // Note: I believe this is safe because of the delimiter we're using, but if we get fancy with
            // unicode we'll need to change this
            start_index = idx + '.'.len_utf8();
            continue;
        }
        current_part.push(c);
    }

    if !current_part.is_empty() {
        let part_span = Span::new(
            raw_column_path.span.start() + start_index,
            raw_column_path.span.start() + last_index + 1,
        );
        if let Ok(row_number) = current_part.parse::<u64>() {
            output.push(Member::Int(BigInt::from(row_number), part_span));
        } else {
            let current_part = trim_quotes(&current_part);
            output.push(Member::Bare(current_part.spanned(part_span)));
        }
    }

    (
        SpannedExpression::new(Expression::simple_column_path(output), raw_column_path.span),
        None,
    )
}

fn trim_quotes(input: &str) -> String {
    let mut chars = input.chars();

    match (chars.next(), chars.next_back()) {
        (Some('\''), Some('\'')) => chars.collect(),
        (Some('"'), Some('"')) => chars.collect(),
        (Some('`'), Some('`')) => chars.collect(),
        _ => input.to_string(),
    }
}
