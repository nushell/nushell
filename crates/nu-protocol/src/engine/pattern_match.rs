use crate::{
    IntRange, ParseError, Range, Record, ShellError, Span, Value, VarId,
    ast::{MatchPattern, Pattern},
};
use std::{cmp::Ordering, ops::Bound};

pub trait Matcher {
    fn match_value(&self, value: &Value, matches: &mut Vec<(VarId, Value)>) -> bool;
}

impl Matcher for MatchPattern {
    fn match_value(&self, value: &Value, matches: &mut Vec<(VarId, Value)>) -> bool {
        self.check(value, matches).is_ok()
    }
}

impl Matcher for Pattern {
    fn match_value(&self, value: &Value, matches: &mut Vec<(VarId, Value)>) -> bool {
        self.check(Span::unknown(), value, matches).is_ok()
    }
}

/// Why a pattern doesn't fit a value: the part of the pattern that failed, and the reason.
#[derive(Debug)]
pub struct PatternMismatch<'a> {
    /// The span of the sub-pattern that failed.
    pub span: Span,
    pub reason: MismatchReason<'a>,
}

/// The reason in a [`PatternMismatch`].
#[derive(Debug)]
pub enum MismatchReason<'a> {
    /// A record or list pattern met a value of another type.
    WrongType {
        expected: &'static str,
        actual: &'static str,
    },
    /// A record pattern names a field the record doesn't have.
    MissingField { field: &'a str, record: &'a Record },
    /// A list pattern takes a different number of items. `min` leaves out the optional items, and
    /// `max` is `None` when the pattern has a rest (`..` or `..$name`).
    ListLength {
        min: usize,
        max: Option<usize>,
        len: usize,
        /// What has the items: "list" or "range".
        of: &'static str,
    },
    /// A list pattern without a rest, or with items after it, met a range that never ends.
    UnboundedRange,
    /// The value doesn't equal a literal, isn't in a range, or fits no side of an or-pattern.
    /// Only `match` arms have these patterns.
    Unmatched,
}

impl MatchPattern {
    /// Match `value`, pushing a binding onto `matches` for each variable in the pattern.
    ///
    /// On failure, report the part of the pattern that didn't fit and why. `matches` then still
    /// holds the bindings made before the failure.
    pub fn check<'a>(
        &'a self,
        value: &'a Value,
        matches: &mut Vec<(VarId, Value)>,
    ) -> Result<(), PatternMismatch<'a>> {
        self.pattern.check(self.span, value, matches)
    }

    /// Bind every variable in the pattern to `null`, for an optional field or item that is
    /// missing.
    fn bind_nulls(&self, matches: &mut Vec<(VarId, Value)>) {
        let nothing = Value::nothing(self.span);
        matches.extend(
            self.variables()
                .into_iter()
                .map(|var_id| (var_id, nothing.clone())),
        );
    }
}

impl Pattern {
    /// [`MatchPattern::check`] for a pattern without its span. `span` is the pattern's span: it
    /// labels a mismatch of the pattern itself and is the span of the `null` values an or-pattern
    /// binds.
    pub fn check<'a>(
        &'a self,
        span: Span,
        value: &'a Value,
        matches: &mut Vec<(VarId, Value)>,
    ) -> Result<(), PatternMismatch<'a>> {
        let mismatch = |reason| Err(PatternMismatch { span, reason });
        match self {
            Pattern::Garbage => mismatch(MismatchReason::Unmatched),
            Pattern::IgnoreValue => Ok(()),
            // `..` and `..$foo` only match inside a list or record pattern, which handles them.
            Pattern::IgnoreRest | Pattern::Rest(_) => mismatch(MismatchReason::Unmatched),
            Pattern::Record(field_patterns) => {
                let Value::Record { val: record, .. } = value else {
                    return mismatch(MismatchReason::WrongType {
                        expected: "record",
                        actual: type_name(value),
                    });
                };
                let mut rest = None;
                for (field, pattern) in field_patterns {
                    if let Pattern::Rest(var_id) = pattern.pattern {
                        rest = Some((var_id, pattern.span));
                        continue;
                    }
                    match record.get(field) {
                        Some(field_value) => pattern.check(field_value, matches)?,
                        None if matches!(pattern.pattern, Pattern::Optional { .. }) => {
                            pattern.pattern.bind_missing(pattern.span, matches)?
                        }
                        None => {
                            return Err(PatternMismatch {
                                span: pattern.span,
                                reason: MismatchReason::MissingField { field, record },
                            });
                        }
                    }
                }
                // `..$rest` gets the fields no other entry names, in the record's order.
                if let Some((var_id, rest_span)) = rest {
                    let named = |col: &str| {
                        field_patterns.iter().any(|(field, pattern)| {
                            field == col && !matches!(pattern.pattern, Pattern::Rest(_))
                        })
                    };
                    let others = record
                        .iter()
                        .filter(|(col, _)| !named(col))
                        .map(|(col, val)| (col.clone(), val.clone()))
                        .collect();
                    matches.push((var_id, Value::record(others, rest_span)));
                }
                Ok(())
            }
            Pattern::Variable(var_id) => {
                matches.push((*var_id, value.clone()));
                Ok(())
            }
            Pattern::List(items) => {
                let shape = ListShape::new(items);
                match value {
                    Value::List { vals, .. } => shape.check_list(span, vals, matches),
                    Value::Range { val, .. } => match **val {
                        Range::IntRange(range) => {
                            shape.check_int_range(span, range, value.span(), matches)
                        }
                        // Float items aren't exact enough to count, so only int ranges qualify.
                        Range::FloatRange(_) => mismatch(MismatchReason::WrongType {
                            expected: "list or int range",
                            actual: "float range",
                        }),
                    },
                    _ => mismatch(MismatchReason::WrongType {
                        expected: "list",
                        actual: type_name(value),
                    }),
                }
            }
            Pattern::Value(pattern_value) => {
                if value == pattern_value {
                    Ok(())
                } else {
                    mismatch(MismatchReason::Unmatched)
                }
            }
            Pattern::Interval { low, high } => {
                let above = match low {
                    Bound::Included(low) => matches!(
                        compare_numbers(value, low),
                        Some(Ordering::Greater | Ordering::Equal)
                    ),
                    Bound::Excluded(low) => compare_numbers(value, low) == Some(Ordering::Greater),
                    Bound::Unbounded => matches!(value, Value::Int { .. } | Value::Float { .. }),
                };
                let below = match high {
                    Bound::Included(high) => matches!(
                        compare_numbers(value, high),
                        Some(Ordering::Less | Ordering::Equal)
                    ),
                    Bound::Excluded(high) => compare_numbers(value, high) == Some(Ordering::Less),
                    Bound::Unbounded => true,
                };
                if above && below {
                    Ok(())
                } else {
                    mismatch(MismatchReason::Unmatched)
                }
            }
            Pattern::Optional { pattern, .. } => match value {
                Value::Nothing { .. } => self.bind_missing(span, matches),
                _ => pattern.check(value, matches),
            },
            Pattern::Or(patterns) => {
                // The first side that fits binds its variables. Each variable of the other sides
                // that isn't bound yet is set to null, so the arm can still use it.
                let mut matched = false;
                for pattern in patterns {
                    if !matched {
                        let mut local_matches = vec![];
                        if pattern.check(value, &mut local_matches).is_ok() {
                            matches.append(&mut local_matches);
                            matched = true;
                            continue;
                        }
                    }
                    for var in pattern.variables() {
                        if !matches.iter().any(|(bound, _)| *bound == var) {
                            matches.push((var, Value::nothing(span)));
                        }
                    }
                }
                if matched {
                    Ok(())
                } else {
                    mismatch(MismatchReason::Unmatched)
                }
            }
        }
    }
}

impl Pattern {
    /// Bind an optional field or item that is missing or `null`: match its default, or bind each
    /// variable to `null`. Only optional parts can be missing, so other patterns bind `null` too.
    fn bind_missing<'a>(
        &'a self,
        span: Span,
        matches: &mut Vec<(VarId, Value)>,
    ) -> Result<(), PatternMismatch<'a>> {
        match self {
            Pattern::Optional {
                pattern,
                default: Some(default),
            } => pattern.check(default, matches),
            Pattern::Optional { pattern, .. } => {
                pattern.bind_nulls(matches);
                Ok(())
            }
            _ => {
                let nothing = Value::nothing(span);
                matches.extend(self.variables().into_iter().map(|id| (id, nothing.clone())));
                Ok(())
            }
        }
    }
}

/// A list pattern split around its rest, with the numbers of items it takes.
///
/// The parser allows at most one `..` or `..$name` per list pattern, at any position. Items
/// before it match the start of the list, items after it match the end, and the rest takes
/// whatever is left in between. Optional items come last, in a pattern without a rest, and may be
/// missing.
struct ListShape<'a> {
    before: &'a [MatchPattern],
    rest: Option<&'a MatchPattern>,
    after: &'a [MatchPattern],
    /// The fewest items, leaving out the optional ones.
    min: usize,
    /// The most items, or `None` with a rest.
    max: Option<usize>,
}

impl<'a> ListShape<'a> {
    fn new(items: &'a [MatchPattern]) -> Self {
        let rest_idx = items
            .iter()
            .position(|item| matches!(item.pattern, Pattern::IgnoreRest | Pattern::Rest(_)));
        let (before, rest, after) = match rest_idx {
            Some(idx) => (&items[..idx], Some(&items[idx]), &items[idx + 1..]),
            None => (items, None, &[][..]),
        };
        let optional = before
            .iter()
            .filter(|item| matches!(item.pattern, Pattern::Optional { .. }))
            .count();
        let fixed = before.len() + after.len();
        Self {
            before,
            rest,
            after,
            min: fixed - optional,
            max: rest.is_none().then_some(fixed),
        }
    }

    /// Check that `len` items fit, for the list or range `of`.
    fn check_len(
        &self,
        span: Span,
        len: usize,
        of: &'static str,
    ) -> Result<(), PatternMismatch<'a>> {
        if len < self.min || self.max.is_some_and(|max| len > max) {
            Err(PatternMismatch {
                span,
                reason: MismatchReason::ListLength {
                    min: self.min,
                    max: self.max,
                    len,
                    of,
                },
            })
        } else {
            Ok(())
        }
    }

    fn check_list(
        &self,
        span: Span,
        vals: &'a [Value],
        matches: &mut Vec<(VarId, Value)>,
    ) -> Result<(), PatternMismatch<'a>> {
        self.check_len(span, vals.len(), "list")?;
        let after_start = vals.len() - self.after.len();

        // Bind left to right, so a variable used twice keeps its rightmost value.
        for (idx, pattern) in self.before.iter().enumerate() {
            match vals.get(idx) {
                Some(val) => pattern.check(val, matches)?,
                None => pattern.pattern.bind_missing(pattern.span, matches)?,
            }
        }
        if let Some(MatchPattern {
            pattern: Pattern::Rest(var_id),
            span,
            ..
        }) = self.rest
        {
            let rest_vals = vals[self.before.len()..after_start].to_vec();
            matches.push((*var_id, Value::list(rest_vals, *span)));
        }
        for (pattern, val) in self.after.iter().zip(&vals[after_start..]) {
            pattern.check(val, matches)?;
        }
        Ok(())
    }

    /// Match an int range as the list of its items, without making that list. `..$rest` gets
    /// the items it covers as a range, so it works on a range without an end too.
    fn check_int_range(
        &self,
        span: Span,
        range: IntRange,
        item_span: Span,
        matches: &mut Vec<(VarId, Value)>,
    ) -> Result<(), PatternMismatch<'a>> {
        if range.is_unbounded() && (self.rest.is_none() || !self.after.is_empty()) {
            return Err(PatternMismatch {
                span,
                reason: MismatchReason::UnboundedRange,
            });
        }
        let start = i128::from(range.start());
        let step = i128::from(range.step());
        // The last item, or for a range without an end, the last one before `i64` overflows.
        let last = match range.end() {
            Bound::Included(end) => i128::from(end),
            Bound::Excluded(end) => i128::from(end) - step.signum(),
            Bound::Unbounded if step > 0 => i128::from(i64::MAX),
            Bound::Unbounded => i128::from(i64::MIN),
        };
        let distance = (last - start) * step.signum();
        let len = if distance < 0 {
            0
        } else {
            usize::try_from(distance / step.abs() + 1).unwrap_or(usize::MAX)
        };
        self.check_len(span, len, "range")?;
        let after_start = len - self.after.len();
        // Every index below `len` is an item between the start and `last`, so it fits an `i64`.
        let item_at = |index: usize| start + index as i128 * step;
        let item = |index: usize| Value::int(item_at(index) as i64, item_span);

        for (idx, pattern) in self.before.iter().enumerate() {
            if idx < len {
                check_range_item(pattern, item(idx), matches)?;
            } else {
                pattern.pattern.bind_missing(pattern.span, matches)?;
            }
        }
        if let Some(MatchPattern {
            pattern: Pattern::Rest(var_id),
            span: rest_span,
            ..
        }) = self.rest
        {
            let rest = if after_start > self.before.len() {
                let end = match range.end() {
                    Bound::Unbounded => Bound::Unbounded,
                    _ => Bound::Included(item_at(after_start - 1) as i64),
                };
                IntRange {
                    start: item_at(self.before.len()) as i64,
                    step: range.step(),
                    end,
                }
            } else {
                // No items are left for the rest: an empty range, `n..<n`.
                let at = i64::try_from(item_at(self.before.len())).unwrap_or(range.start());
                IntRange {
                    start: at,
                    step: 1,
                    end: Bound::Excluded(at),
                }
            };
            matches.push((*var_id, Value::range(Range::IntRange(rest), *rest_span)));
        }
        for (offset, pattern) in self.after.iter().enumerate() {
            check_range_item(pattern, item(after_start + offset), matches)?;
        }
        Ok(())
    }
}

/// Match an item of a range, an int made for the match. The detailed reason would borrow the
/// item, so a failure reports what an int can't fit: a record or list pattern, or a literal.
fn check_range_item<'a>(
    pattern: &'a MatchPattern,
    item: Value,
    matches: &mut Vec<(VarId, Value)>,
) -> Result<(), PatternMismatch<'a>> {
    let mut bindings = vec![];
    if pattern.check(&item, &mut bindings).is_ok() {
        matches.append(&mut bindings);
        return Ok(());
    }
    let failed = match &pattern.pattern {
        Pattern::Optional { pattern, .. } => pattern,
        _ => pattern,
    };
    let reason = match failed.pattern {
        Pattern::Record(_) => MismatchReason::WrongType {
            expected: "record",
            actual: "int",
        },
        Pattern::List(_) => MismatchReason::WrongType {
            expected: "list",
            actual: "int",
        },
        _ => MismatchReason::Unmatched,
    };
    Err(PatternMismatch {
        span: failed.span,
        reason,
    })
}

/// The name of a value's type for a mismatch message. A table is a list here: telling them apart
/// would scan the list on every failed `match` arm.
fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Bool { .. } => "bool",
        Value::Int { .. } => "int",
        Value::Float { .. } => "float",
        Value::Filesize { .. } => "filesize",
        Value::Duration { .. } => "duration",
        Value::Date { .. } => "datetime",
        Value::Range { .. } => "range",
        Value::String { .. } => "string",
        Value::Glob { .. } => "glob",
        Value::Record { .. } => "record",
        Value::List { .. } => "list",
        Value::Closure { .. } => "closure",
        Value::Error { .. } => "error",
        Value::Binary { .. } => "binary",
        Value::CellPath { .. } => "cell-path",
        Value::Custom { .. } => "custom",
        Value::Nothing { .. } => "nothing",
    }
}

/// Compare two numbers, ints as ints. `None` when either isn't a number, or for NaN.
fn compare_numbers(value: &Value, bound: &Value) -> Option<Ordering> {
    match (value, bound) {
        (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => Some(lhs.cmp(rhs)),
        (Value::Int { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
            (*lhs as f64).partial_cmp(rhs)
        }
        (Value::Float { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
            lhs.partial_cmp(&(*rhs as f64))
        }
        (Value::Float { val: lhs, .. }, Value::Float { val: rhs, .. }) => lhs.partial_cmp(rhs),
        _ => None,
    }
}

/// How many field names a missing-field error lists before it summarizes the rest.
const LISTED_FIELDS: usize = 10;

impl PatternMismatch<'_> {
    /// The error for a destructuring binding that doesn't fit its value.
    pub fn into_shell_error(self) -> ShellError {
        let (msg, span, help) = self.describe();
        ShellError::PatternMismatch { msg, span, help }
    }

    /// The error for a destructuring `const` that doesn't fit its value.
    pub fn into_parse_error(self) -> ParseError {
        let (msg, span, help) = self.describe();
        ParseError::PatternMismatch { msg, span, help }
    }

    /// The error's label, its span, and its help.
    fn describe(self) -> (String, Span, Option<String>) {
        let (msg, help) = match self.reason {
            MismatchReason::WrongType { expected, actual } => {
                (format!("expected a {expected}, got {actual}"), None)
            }
            MismatchReason::MissingField { field, record } => {
                let help = if record.is_empty() {
                    "The record is empty.".to_string()
                } else {
                    let names = record
                        .columns()
                        .take(LISTED_FIELDS)
                        .map(String::as_str)
                        .collect::<Vec<_>>()
                        .join(", ");
                    match record.len().checked_sub(LISTED_FIELDS) {
                        Some(more) if more > 0 => {
                            format!("The record has {names} and {more} more.")
                        }
                        _ => format!("The record has {names}."),
                    }
                };
                let help = format!("{help} Add `?` after the pattern if the field can be missing.");
                (format!("record has no field `{field}`"), Some(help))
            }
            MismatchReason::ListLength { min, max, len, of } => {
                let items = if len == 1 { "item" } else { "items" };
                let takes = match max {
                    Some(max) if max == min => format!("takes {max}"),
                    Some(max) => format!("takes {min} to {max}"),
                    None => format!("needs at least {min}"),
                };
                // `..` can't join optional items, so it's only a fix for an exact length.
                let help = (max == Some(min) && len > min)
                    .then(|| "Add `..` to ignore the rest.".to_string());
                (format!("{of} has {len} {items}, the pattern {takes}"), help)
            }
            MismatchReason::UnboundedRange => (
                "the range has no end".into(),
                Some(
                    "End the pattern with `..` or `..$rest` to take a range without an end.".into(),
                ),
            ),
            MismatchReason::Unmatched => ("the value doesn't fit this pattern".into(), None),
        };
        (msg, self.span, help)
    }
}
