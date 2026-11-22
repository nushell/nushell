use crate::{
    Record, ShellError, Span, Value, VarId,
    ast::{MatchPattern, Pattern},
};

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
        value: &'a Value,
    },
    /// A record pattern names a field the record doesn't have.
    MissingField { field: &'a str, record: &'a Record },
    /// A list pattern takes a different number of items. `max` is `None` when the pattern has a
    /// rest (`..` or `..$name`).
    ListLength {
        min: usize,
        max: Option<usize>,
        len: usize,
    },
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
            // `..` and `..$foo` only match inside a list pattern, which handles them itself.
            Pattern::IgnoreRest | Pattern::Rest(_) => mismatch(MismatchReason::Unmatched),
            Pattern::Record(field_patterns) => {
                let Value::Record { val: record, .. } = value else {
                    return mismatch(MismatchReason::WrongType {
                        expected: "record",
                        value,
                    });
                };
                for (field, pattern) in field_patterns {
                    let Some(field_value) = record.get(field) else {
                        return Err(PatternMismatch {
                            span: pattern.span,
                            reason: MismatchReason::MissingField { field, record },
                        });
                    };
                    pattern.check(field_value, matches)?;
                }
                Ok(())
            }
            Pattern::Variable(var_id) => {
                matches.push((*var_id, value.clone()));
                Ok(())
            }
            Pattern::List(items) => {
                let Value::List { vals, .. } = value else {
                    return mismatch(MismatchReason::WrongType {
                        expected: "list",
                        value,
                    });
                };
                // The parser allows at most one `..` or `..$name` per list pattern, at any
                // position. Items before it match the start of the list, items after it match
                // the end, and the rest pattern takes whatever is left in between.
                let rest_idx = items.iter().position(|item| {
                    matches!(item.pattern, Pattern::IgnoreRest | Pattern::Rest(_))
                });
                let (before, rest, after) = match rest_idx {
                    Some(idx) => (&items[..idx], Some(&items[idx]), &items[idx + 1..]),
                    None => (&items[..], None, &[][..]),
                };

                // Without a rest pattern the lengths must be equal; with one the list may be longer.
                let fixed_len = before.len() + after.len();
                if vals.len() < fixed_len || (rest.is_none() && vals.len() > fixed_len) {
                    return mismatch(MismatchReason::ListLength {
                        min: fixed_len,
                        max: rest.is_none().then_some(fixed_len),
                        len: vals.len(),
                    });
                }
                let after_start = vals.len() - after.len();

                // Bind left to right, so a variable used twice keeps its rightmost value.
                for (pattern, val) in before.iter().zip(vals) {
                    pattern.check(val, matches)?;
                }
                if let Some(MatchPattern {
                    pattern: Pattern::Rest(var_id),
                    span,
                    ..
                }) = rest
                {
                    let rest_vals = vals[before.len()..after_start].to_vec();
                    matches.push((*var_id, Value::list(rest_vals, *span)));
                }
                for (pattern, val) in after.iter().zip(&vals[after_start..]) {
                    pattern.check(val, matches)?;
                }
                Ok(())
            }
            Pattern::Value(pattern_value) => {
                let fits = match pattern_value {
                    // Ranges match by containment.
                    Value::Range { val, .. } => val.contains(value),
                    _ => value == pattern_value,
                };
                if fits {
                    Ok(())
                } else {
                    mismatch(MismatchReason::Unmatched)
                }
            }
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

/// How many field names a missing-field error lists before it summarizes the rest.
const LISTED_FIELDS: usize = 10;

impl PatternMismatch<'_> {
    /// The error for a destructuring binding that doesn't fit its value.
    pub fn into_shell_error(self) -> ShellError {
        let (msg, help) = match self.reason {
            MismatchReason::WrongType { expected, value } => (
                format!(
                    "expected a {expected}, got {}",
                    value.get_type().get_non_specified_string()
                ),
                None,
            ),
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
                (format!("record has no field `{field}`"), Some(help))
            }
            MismatchReason::ListLength { min, max, len } => {
                let items = if len == 1 { "item" } else { "items" };
                match max {
                    Some(max) => (
                        format!("list has {len} {items}, the pattern takes {max}"),
                        (len > max).then(|| "Add `..` to ignore the rest.".to_string()),
                    ),
                    None => (
                        format!("list has {len} {items}, the pattern needs at least {min}"),
                        None,
                    ),
                }
            }
            MismatchReason::Unmatched => ("the value doesn't fit this pattern".into(), None),
        };
        ShellError::PatternMismatch {
            msg,
            span: self.span,
            help,
        }
    }
}
