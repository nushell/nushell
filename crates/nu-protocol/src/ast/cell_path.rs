use super::Expression;
use crate::{Span, casing::Casing};
use nu_utils::{escape_quote_string, needs_quoting};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, fmt::Display, str::FromStr};
use winnow::Parser;

/// One level of access of a [`CellPath`]
#[derive(Debug, Clone)]
pub enum PathMember {
    /// Accessing a member by string (i.e. columns of a table or [`Record`](crate::Record))
    String {
        val: String,
        span: Span,
        /// If marked as optional don't throw an error if not found but perform default handling
        /// (e.g. return `Value::Nothing`)
        optional: bool,
        /// Affects column lookup
        casing: Casing,
    },
    /// Accessing a member by index (i.e. row of a table or item in a list)
    Int {
        val: usize,
        span: Span,
        /// If marked as optional don't throw an error if not found but perform default handling
        /// (e.g. return `Value::Nothing`)
        optional: bool,
    },
}

impl PathMember {
    pub fn int(val: usize, optional: bool, span: Span) -> Self {
        PathMember::Int {
            val,
            span,
            optional,
        }
    }

    pub fn string(val: String, optional: bool, casing: Casing, span: Span) -> Self {
        PathMember::String {
            val,
            span,
            optional,
            casing,
        }
    }

    pub fn test_int(val: usize, optional: bool) -> Self {
        PathMember::Int {
            val,
            optional,
            span: Span::test_data(),
        }
    }

    pub fn test_string(val: String, optional: bool, casing: Casing) -> Self {
        PathMember::String {
            val,
            optional,
            casing,
            span: Span::test_data(),
        }
    }

    pub fn make_optional(&mut self) {
        match self {
            PathMember::String { optional, .. } => *optional = true,
            PathMember::Int { optional, .. } => *optional = true,
        }
    }

    pub fn make_insensitive(&mut self) {
        match self {
            PathMember::String { casing, .. } => *casing = Casing::Insensitive,
            PathMember::Int { .. } => {}
        }
    }

    pub fn span(&self) -> Span {
        match self {
            PathMember::String { span, .. } => *span,
            PathMember::Int { span, .. } => *span,
        }
    }

    /// Returns an estimate of the memory size used by this PathMember in bytes
    pub fn memory_size(&self) -> usize {
        match self {
            PathMember::String { val, .. } => std::mem::size_of::<Self>() + val.capacity(),
            PathMember::Int { .. } => std::mem::size_of::<Self>(),
        }
    }
}

impl PartialEq for PathMember {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::String {
                    val: l_val,
                    optional: l_opt,
                    ..
                },
                Self::String {
                    val: r_val,
                    optional: r_opt,
                    ..
                },
            ) => l_val == r_val && l_opt == r_opt,
            (
                Self::Int {
                    val: l_val,
                    optional: l_opt,
                    ..
                },
                Self::Int {
                    val: r_val,
                    optional: r_opt,
                    ..
                },
            ) => l_val == r_val && l_opt == r_opt,
            _ => false,
        }
    }
}

impl PartialOrd for PathMember {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (
                PathMember::String {
                    val: l_val,
                    optional: l_opt,
                    ..
                },
                PathMember::String {
                    val: r_val,
                    optional: r_opt,
                    ..
                },
            ) => {
                let val_ord = Some(l_val.cmp(r_val));

                if let Some(Ordering::Equal) = val_ord {
                    Some(l_opt.cmp(r_opt))
                } else {
                    val_ord
                }
            }
            (
                PathMember::Int {
                    val: l_val,
                    optional: l_opt,
                    ..
                },
                PathMember::Int {
                    val: r_val,
                    optional: r_opt,
                    ..
                },
            ) => {
                let val_ord = Some(l_val.cmp(r_val));

                if let Some(Ordering::Equal) = val_ord {
                    Some(l_opt.cmp(r_opt))
                } else {
                    val_ord
                }
            }
            (PathMember::Int { .. }, PathMember::String { .. }) => Some(Ordering::Greater),
            (PathMember::String { .. }, PathMember::Int { .. }) => Some(Ordering::Less),
        }
    }
}

impl Display for PathMember {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PathMember::Int { val, optional, .. } => {
                let question_mark = if *optional { "?" } else { "" };
                write!(f, "{val}{question_mark}")
            }
            PathMember::String {
                val,
                optional,
                casing,
                ..
            } => {
                let question_mark = if *optional { "?" } else { "" };
                let exclamation_mark = if *casing == Casing::Insensitive {
                    "!"
                } else {
                    ""
                };
                let val = if needs_quoting(val) {
                    &escape_quote_string(val)
                } else {
                    val
                };
                write!(f, "{val}{exclamation_mark}{question_mark}")
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("could not parse path member {attempted:?}")]
pub struct PathMemberParseError {
    attempted: String,
}

impl FromStr for PathMember {
    type Err = PathMemberParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse::path_member
            .parse(s)
            .map_err(|_| PathMemberParseError {
                attempted: s.to_owned(),
            })
    }
}

impl Serialize for PathMember {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PathMember {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_str(&s).map_err(serde::de::Error::custom)
    }
}

/// [`PathMember`] for testing purposes.
///
/// This path member may be converted via [`into_path_member`](Self::into_path_member) into a
/// [`PathMember`] that is using a [`Span::test_data()`](crate::Span::test_data) span.
#[doc(hidden)]
pub struct TestPathMember<T>(T);

impl<S: Into<String>> From<S> for TestPathMember<String> {
    fn from(value: S) -> Self {
        Self(value.into())
    }
}

impl TestPathMember<String> {
    pub fn into_path_member(self) -> PathMember {
        PathMember::test_string(self.0, false, Casing::Sensitive)
    }
}

impl From<usize> for TestPathMember<usize> {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl TestPathMember<usize> {
    pub fn into_path_member(self) -> PathMember {
        PathMember::test_int(self.0, false)
    }
}

/// Represents the potentially nested access to fields/cells of a container type
///
/// In our current implementation for table access the order of row/column is commutative.
/// This limits the number of possible rows to select in one [`CellPath`] to 1 as it could
/// otherwise be ambiguous
///
/// ```nushell
/// col1.0
/// 0.col1
/// col2
/// 42
/// ```
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct CellPath {
    pub members: Vec<PathMember>,
}

impl CellPath {
    pub fn empty() -> Self {
        Self {
            members: Vec::new(),
        }
    }

    pub fn make_optional(&mut self) {
        for member in &mut self.members {
            member.make_optional();
        }
    }

    pub fn make_insensitive(&mut self) {
        for member in &mut self.members {
            member.make_insensitive();
        }
    }

    // Formats the cell-path as a column name, i.e. without quoting and optional markers ('?').
    pub fn to_column_name(&self) -> String {
        let mut s = String::new();

        for member in &self.members {
            match member {
                PathMember::Int { val, .. } => {
                    s += &val.to_string();
                }
                PathMember::String { val, .. } => {
                    s += val;
                }
            }

            s.push('.');
        }

        s.pop(); // Easier than checking whether to insert the '.' on every iteration.
        s
    }

    /// Returns an estimate of the memory size used by this CellPath in bytes
    pub fn memory_size(&self) -> usize {
        std::mem::size_of::<Self>() + self.members.iter().map(|m| m.memory_size()).sum::<usize>()
    }
}

impl Display for CellPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "$")?;
        for member in self.members.iter() {
            write!(f, ".{member}")?;
        }
        // Empty cell-paths are `$.` not `$`
        if self.members.is_empty() {
            write!(f, ".")?;
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
#[error("could not parse cell path {attempted:?}")]
pub struct CellPathParseError {
    attempted: String,
}

impl FromStr for CellPath {
    type Err = CellPathParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse::cell_path.parse(s).map_err(|_| CellPathParseError {
            attempted: s.to_owned(),
        })
    }
}

impl Serialize for CellPath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CellPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_str(&s).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FullCellPath {
    pub head: Expression,
    pub tail: Vec<PathMember>,
}

mod parse {
    use super::*;
    use winnow::{
        Result, Str, combinator::*, error::*, prelude::*, stream::ContainsToken, token::*,
    };

    pub fn cell_path(input: &mut &str) -> Result<CellPath> {
        preceded(opt("$."), repeat(0.., terminated(path_member, opt('.'))))
            .parse_next(input)
            .map(|members| CellPath { members })
    }

    pub fn path_member(input: &mut &str) -> Result<PathMember> {
        if input.is_empty() {
            return Err(ParserError::from_input(input));
        }

        let member = alt((int_path_member, string_path_member)).parse_next(input)?;

        // ensure there's no more content after a member
        peek(alt((".", eof))).parse_next(input)?;

        Ok(member)
    }

    fn int_path_member(input: &mut &str) -> Result<PathMember> {
        let int = digits.parse_next(input)?;
        let modifier = modifier.parse_next(input)?;
        Ok(PathMember::Int {
            val: int,
            span: Span::unknown(),
            optional: modifier.optional,
        })
    }

    fn digits(input: &mut &str) -> Result<usize> {
        let start = input.checkpoint();
        if let Ok(prefix) = digit_prefix.parse_next(input) {
            return match prefix {
                DigitPrefix::Bin => bin_digits.parse_next(input),
                DigitPrefix::Oct => oct_digits.parse_next(input),
                DigitPrefix::Hex => hex_digits.parse_next(input),
            };
        }

        input.reset(&start);
        dec_digits.parse_next(input)
    }

    enum DigitPrefix {
        Bin,
        Oct,
        Hex,
    }

    fn digit_prefix(input: &mut &str) -> Result<DigitPrefix> {
        let prefix = take(2usize).parse_next(input)?;
        Ok(match prefix {
            "0b" => DigitPrefix::Bin,
            "0o" => DigitPrefix::Oct,
            "Ox" => DigitPrefix::Hex,
            _ => return fail(input),
        })
    }

    fn bin_digits(input: &mut &str) -> Result<usize> {
        any_radix_digits(2, ('_', '0', '1')).parse_next(input)
    }

    fn oct_digits(input: &mut &str) -> Result<usize> {
        any_radix_digits(8, ('_', '0'..='7')).parse_next(input)
    }

    fn dec_digits(input: &mut &str) -> Result<usize> {
        any_radix_digits(10, ('_', '0'..='9')).parse_next(input)
    }

    fn hex_digits(input: &mut &str) -> Result<usize> {
        any_radix_digits(16, ('_', '0'..='9', 'a'..='f', 'A'..='Z')).parse_next(input)
    }

    fn any_radix_digits<'i>(
        radix: u32,
        tokens: impl ContainsToken<char>,
    ) -> impl Parser<Str<'i>, usize, ContextError> {
        take_while(1.., tokens)
            .map(|d: &str| d.replace('_', ""))
            .verify(|d: &str| !d.is_empty())
            .try_map(move |d| usize::from_str_radix(&d, radix))
    }

    fn string_path_member(input: &mut &str) -> Result<PathMember> {
        let string = alt((
            single_quoted_string,
            bare_word_string,
            double_quoted_string,
            unquoted_string,
        ))
        .parse_next(input)?;

        let modifier = modifier.parse_next(input)?;

        Ok(PathMember::String {
            val: string,
            span: Span::unknown(),
            optional: modifier.optional,
            casing: match modifier.case_insensitive {
                true => Casing::Insensitive,
                false => Default::default(),
            },
        })
    }

    fn unquoted_string(input: &mut &str) -> Result<String> {
        struct UnquotedTokens;

        impl ContainsToken<char> for UnquotedTokens {
            fn contains_token(&self, token: char) -> bool {
                match token {
                    // spaces and tabs
                    ' ' | '\n' | '\t' => false,

                    // syntax characters
                    '!' | '?' | '.' => false,

                    // brackets
                    '(' | ')' => false,

                    _ => true,
                }
            }
        }

        take_while(0.., UnquotedTokens)
            .parse_next(input)
            .map(|s| s.to_owned())
    }

    fn single_quoted_string(input: &mut &str) -> Result<String> {
        delimited("'", take_while(0.., |c| c != '\''), "'")
            .parse_next(input)
            .map(|s| s.to_owned())
    }

    fn bare_word_string(input: &mut &str) -> Result<String> {
        delimited("`", take_while(0.., |c| c != '`'), "`")
            .parse_next(input)
            .map(|s| s.to_owned())
    }

    fn double_quoted_string(input: &mut &str) -> Result<String> {
        fn escaped(input: &mut &str) -> Result<char> {
            preceded(
                '\\',
                alt((
                    'n'.value('\n'),
                    'r'.value('\r'),
                    't'.value('\t'),
                    '\\'.value('\\'),
                    '/'.value('/'),
                    '"'.value('"'),
                )),
            )
            .parse_next(input)
        }

        fn char(input: &mut &str) -> Result<char> {
            any.verify(|c| *c != '"').parse_next(input)
        }

        let content = repeat(0.., alt((escaped, char))).fold(String::new, |mut string, char| {
            string.push(char);
            string
        });

        delimited('"', content, '"').parse_next(input)
    }

    #[derive(Default)]
    struct Modifier {
        optional: bool,
        case_insensitive: bool,
    }

    fn modifier(input: &mut &str) -> Result<Modifier> {
        let mut modifier = Modifier::default();

        loop {
            let Some(next) = opt(alt(('!', '?'))).parse_next(input)? else {
                break;
            };

            let expected = match (next, modifier.optional, modifier.case_insensitive) {
                ('!', _, false) => {
                    modifier.case_insensitive = true;
                    continue;
                }
                ('?', false, _) => {
                    modifier.optional = true;
                    continue;
                }
                ('!', false, true) => "'?' or '.'",
                ('!', true, true) => "'.'",
                ('?', true, false) => "'!' or '.'",
                ('?', true, true) => "'.'",
                (c, _, _) => unreachable!("parser only returns with '!' or '?', got {c:?}"),
            };

            fail.context(StrContext::Expected(StrContextValue::Description(expected)))
                .parse_next(input)?
        }

        Ok(modifier)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::cmp::Ordering::Greater;

    #[test]
    fn path_member_partial_ord() {
        assert_eq!(
            Some(Greater),
            PathMember::test_int(5, true).partial_cmp(&PathMember::test_string(
                "e".into(),
                true,
                Casing::Sensitive
            ))
        );

        assert_eq!(
            Some(Greater),
            PathMember::test_int(5, true).partial_cmp(&PathMember::test_int(5, false))
        );

        assert_eq!(
            Some(Greater),
            PathMember::test_int(6, true).partial_cmp(&PathMember::test_int(5, true))
        );

        assert_eq!(
            Some(Greater),
            PathMember::test_string("e".into(), true, Casing::Sensitive).partial_cmp(
                &PathMember::test_string("e".into(), false, Casing::Sensitive)
            )
        );

        assert_eq!(
            Some(Greater),
            PathMember::test_string("f".into(), true, Casing::Sensitive).partial_cmp(
                &PathMember::test_string("e".into(), true, Casing::Sensitive)
            )
        );
    }
}
