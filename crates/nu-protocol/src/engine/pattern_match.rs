use crate::{
    Span, Value, VarId,
    ast::{Expr, MatchPattern, Pattern, RangeInclusion},
};

pub trait Matcher {
    fn match_value(&self, value: &Value, matches: &mut Vec<(VarId, Value)>) -> bool;
}

impl Matcher for MatchPattern {
    fn match_value(&self, value: &Value, matches: &mut Vec<(VarId, Value)>) -> bool {
        self.pattern.match_value(value, matches)
    }
}

impl Matcher for Pattern {
    fn match_value(&self, value: &Value, matches: &mut Vec<(VarId, Value)>) -> bool {
        match self {
            Pattern::Garbage => false,
            Pattern::IgnoreValue => true,
            Pattern::IgnoreRest => false, // `..` and `..$foo` only match in specific contexts
            Pattern::Rest(_) => false,    // so we return false here and handle them elsewhere
            Pattern::Record(field_patterns) => match value {
                Value::Record { val, .. } => {
                    'top: for field_pattern in field_patterns {
                        for (col, val) in &**val {
                            if col == &field_pattern.0 {
                                // We have found the field
                                let result = field_pattern.1.match_value(val, matches);
                                if !result {
                                    return false;
                                } else {
                                    continue 'top;
                                }
                            }
                        }
                        return false;
                    }
                    true
                }
                _ => false,
            },
            Pattern::Variable(var_id) => {
                // TODO: FIXME: This needs the span of this variable
                matches.push((*var_id, value.clone()));
                true
            }
            Pattern::List(items) => match &value {
                Value::List { vals, .. } => {
                    // The parser allows at most one `..` or `..$name` per list pattern, at any
                    // position. Items before it match the start of the list, items after it
                    // match the end, and the rest pattern takes whatever is left in between.
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
                        return false;
                    }
                    let after_start = vals.len() - after.len();

                    // Bind left to right, so a variable used twice keeps its rightmost value.
                    if !before
                        .iter()
                        .zip(vals)
                        .all(|(pattern, val)| pattern.match_value(val, matches))
                    {
                        return false;
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
                    after
                        .iter()
                        .zip(&vals[after_start..])
                        .all(|(pattern, val)| pattern.match_value(val, matches))
                }
                _ => false,
            },
            Pattern::Expression(pattern_value) => {
                // TODO: Fill this out with the rest of them
                match &pattern_value.expr {
                    Expr::Nothing => {
                        matches!(value, Value::Nothing { .. })
                    }
                    Expr::Int(x) => {
                        if let Value::Int { val, .. } = &value {
                            x == val
                        } else {
                            false
                        }
                    }
                    Expr::Float(x) => {
                        if let Value::Float { val, .. } = &value {
                            x == val
                        } else {
                            false
                        }
                    }
                    Expr::Binary(x) => {
                        if let Value::Binary { val, .. } = &value {
                            x.as_slice() == val.as_slice()
                        } else {
                            false
                        }
                    }
                    Expr::Bool(x) => {
                        if let Value::Bool { val, .. } = &value {
                            x == val
                        } else {
                            false
                        }
                    }
                    Expr::String(x) | Expr::RawString(x) => {
                        if let Value::String { val, .. } = &value {
                            x == val
                        } else {
                            false
                        }
                    }
                    Expr::DateTime(x) => {
                        if let Value::Date { val, .. } = &value {
                            x == val
                        } else {
                            false
                        }
                    }
                    Expr::ValueWithUnit(val) => {
                        let span = val.unit.span;

                        if let Expr::Int(size) = val.expr.expr {
                            match &val.unit.item.build_value(size, span) {
                                Ok(v) => v == value,
                                _ => false,
                            }
                        } else {
                            false
                        }
                    }
                    Expr::Range(range) => {
                        // TODO: Add support for floats

                        let start = if let Some(start) = &range.from {
                            match &start.expr {
                                Expr::Int(start) => *start,
                                _ => return false,
                            }
                        } else {
                            0
                        };

                        let end = if let Some(end) = &range.to {
                            match &end.expr {
                                Expr::Int(end) => *end,
                                _ => return false,
                            }
                        } else {
                            i64::MAX
                        };

                        let step = if let Some(step) = &range.next {
                            match &step.expr {
                                Expr::Int(step) => *step - start,
                                _ => return false,
                            }
                        } else if end < start {
                            -1
                        } else {
                            1
                        };

                        let (start, end) = if end < start {
                            (end, start)
                        } else {
                            (start, end)
                        };

                        if let Value::Int { val, .. } = &value {
                            if matches!(range.operator.inclusion, RangeInclusion::RightExclusive) {
                                *val >= start && *val < end && ((*val - start) % step) == 0
                            } else {
                                *val >= start && *val <= end && ((*val - start) % step) == 0
                            }
                        } else {
                            false
                        }
                    }
                    _ => false,
                }
            }
            Pattern::Value(pattern_value) => match pattern_value {
                // Ranges match by containment (same as `Pattern::Expression` + `Expr::Range`).
                Value::Range { val, .. } => val.contains(value),
                _ => value == pattern_value,
            },
            Pattern::Or(patterns) => {
                let mut result = false;

                for pattern in patterns {
                    let mut local_matches = vec![];
                    if !result {
                        if pattern.match_value(value, &mut local_matches) {
                            // TODO: do we need to replace previous variables that defaulted to nothing?
                            matches.append(&mut local_matches);
                            result = true;
                        } else {
                            // Create variables that don't match and assign them to null
                            let vars = pattern.variables();
                            for var in &vars {
                                let mut found = false;
                                for match_ in matches.iter() {
                                    if match_.0 == *var {
                                        found = true;
                                    }
                                }

                                if !found {
                                    // FIXME: don't use Span::unknown()
                                    matches.push((*var, Value::nothing(Span::unknown())))
                                }
                            }
                        }
                    } else {
                        // We already have a match, so ignore the remaining match variables
                        // And assign them to null
                        let vars = pattern.variables();
                        for var in &vars {
                            let mut found = false;
                            for match_ in matches.iter() {
                                if match_.0 == *var {
                                    found = true;
                                }
                            }

                            if !found {
                                // FIXME: don't use Span::unknown()
                                matches.push((*var, Value::nothing(Span::unknown())))
                            }
                        }
                    }
                }
                result
            }
        }
    }
}
