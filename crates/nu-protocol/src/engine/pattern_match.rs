use crate::{
    ast::{Expr, MatchPattern, Pattern, RangeInclusion},
    Value, VarId,
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
                        // We need more items in our pattern than are available in the Value
                        return false;
                    }

                    for (val_idx, val) in vals.iter().enumerate() {
                        // We require that the pattern and the value have the same number of items, or the pattern does not match
                        // The only exception is if the pattern includes a `..` pattern

                        if let Some(pattern) = items.get(val_idx) {
                            if !pattern.match_value(val, matches) {
                                return false;
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
        }
    }
}
