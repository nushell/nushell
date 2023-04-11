use crate::{
    ast::{Expr, MatchPattern, Pattern, RangeInclusion},
    Span, Unit, Value, VarId,
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
                Value::Record { cols, vals, .. } => {
                    'top: for field_pattern in field_patterns {
                        for (col_idx, col) in cols.iter().enumerate() {
                            if col == &field_pattern.0 {
                                // We have found the field
                                let result = field_pattern.1.match_value(&vals[col_idx], matches);
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
                    if items.len() > vals.len() {
                        // The only we we allow this is to have a rest pattern in the n+1 position
                        if items.len() == (vals.len() + 1) {
                            match &items[vals.len()].pattern {
                                Pattern::IgnoreRest => {}
                                Pattern::Rest(var_id) => {
                                    matches.push((*var_id, Value::nothing(items[vals.len()].span)))
                                }
                                _ => {
                                    // There is a pattern which can't skip missing values, so we fail
                                    return false;
                                }
                            }
                        } else {
                            // There are patterns that can't be matches, so we fail
                            return false;
                        }
                    }
                    for (val_idx, val) in vals.iter().enumerate() {
                        // We require that the pattern and the value have the same number of items, or the pattern does not match
                        // The only exception is if the pattern includes a `..` pattern
                        if let Some(pattern) = items.get(val_idx) {
                            match &pattern.pattern {
                                Pattern::IgnoreRest => {
                                    break;
                                }
                                Pattern::Rest(var_id) => {
                                    let rest_vals = vals[val_idx..].to_vec();
                                    matches.push((
                                        *var_id,
                                        Value::List {
                                            vals: rest_vals,
                                            span: pattern.span,
                                        },
                                    ));
                                    break;
                                }
                                _ => {
                                    if !pattern.match_value(val, matches) {
                                        return false;
                                    }
                                }
                            }
                        } else {
                            return false;
                        }
                    }

                    true
                }
                _ => false,
            },
            Pattern::Value(pattern_value) => {
                // TODO: Fill this out with the rest of them
                match &pattern_value.expr {
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
                            x == val
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
                    Expr::String(x) => {
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
                    Expr::ValueWithUnit(amount, unit) => {
                        if let Value::Filesize { val, .. } = &value {
                            // FIXME: we probably want this math in one place that both the
                            // pattern matcher and the eval engine can get to it
                            match &amount.expr {
                                Expr::Int(amount) => match &unit.item {
                                    Unit::Byte => amount == val,
                                    Unit::Kilobyte => *val == amount * 1000,
                                    Unit::Megabyte => *val == amount * 1000 * 1000,
                                    Unit::Gigabyte => *val == amount * 1000 * 1000 * 1000,
                                    Unit::Petabyte => *val == amount * 1000 * 1000 * 1000 * 1000,
                                    Unit::Exabyte => {
                                        *val == amount * 1000 * 1000 * 1000 * 1000 * 1000
                                    }
                                    Unit::Zettabyte => {
                                        *val == amount * 1000 * 1000 * 1000 * 1000 * 1000 * 1000
                                    }
                                    Unit::Kibibyte => *val == amount * 1024,
                                    Unit::Mebibyte => *val == amount * 1024 * 1024,
                                    Unit::Gibibyte => *val == amount * 1024 * 1024 * 1024,
                                    Unit::Pebibyte => *val == amount * 1024 * 1024 * 1024 * 1024,
                                    Unit::Exbibyte => {
                                        *val == amount * 1024 * 1024 * 1024 * 1024 * 1024
                                    }
                                    Unit::Zebibyte => {
                                        *val == amount * 1024 * 1024 * 1024 * 1024 * 1024 * 1024
                                    }
                                    _ => false,
                                },
                                _ => false,
                            }
                        } else if let Value::Duration { val, .. } = &value {
                            // FIXME: we probably want this math in one place that both the
                            // pattern matcher and the eval engine can get to it
                            match &amount.expr {
                                Expr::Int(amount) => match &unit.item {
                                    Unit::Nanosecond => val == amount,
                                    Unit::Microsecond => *val == amount * 1000,
                                    Unit::Millisecond => *val == amount * 1000 * 1000,
                                    Unit::Second => *val == amount * 1000 * 1000 * 1000,
                                    Unit::Minute => *val == amount * 1000 * 1000 * 1000 * 60,
                                    Unit::Hour => *val == amount * 1000 * 1000 * 1000 * 60 * 60,
                                    Unit::Day => *val == amount * 1000 * 1000 * 1000 * 60 * 60 * 24,
                                    Unit::Week => {
                                        *val == amount * 1000 * 1000 * 1000 * 60 * 60 * 24 * 7
                                    }
                                    _ => false,
                                },
                                _ => false,
                            }
                        } else {
                            false
                        }
                    }
                    Expr::Range(start, step, end, inclusion) => {
                        // TODO: Add support for floats

                        let start = if let Some(start) = &start {
                            match &start.expr {
                                Expr::Int(start) => *start,
                                _ => return false,
                            }
                        } else {
                            0
                        };

                        let end = if let Some(end) = &end {
                            match &end.expr {
                                Expr::Int(end) => *end,
                                _ => return false,
                            }
                        } else {
                            i64::MAX
                        };

                        let step = if let Some(step) = step {
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
                            if matches!(inclusion.inclusion, RangeInclusion::RightExclusive) {
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
