use crate::{ast::RangeInclusion, *};

#[derive(Debug, Clone, PartialEq)]
pub struct Range {
    pub from: Value,
    pub to: Value,
    pub inclusion: RangeInclusion,
}

impl IntoIterator for Range {
    type Item = Value;

    type IntoIter = RangeIterator;

    fn into_iter(self) -> Self::IntoIter {
        let span = self.from.span();

        RangeIterator::new(self, span)
    }
}

pub struct RangeIterator {
    curr: Value,
    end: Value,
    span: Span,
    is_end_inclusive: bool,
    moves_up: bool,
    one: Value,
    negative_one: Value,
    done: bool,
}

impl RangeIterator {
    pub fn new(range: Range, span: Span) -> RangeIterator {
        let start = match range.from {
            Value::Nothing { .. } => Value::Int { val: 0, span },
            x => x,
        };

        let end = match range.to {
            Value::Nothing { .. } => Value::Int {
                val: i64::MAX,
                span,
            },
            x => x,
        };

        RangeIterator {
            moves_up: matches!(start.lte(span, &end), Ok(Value::Bool { val: true, .. })),
            curr: start,
            end,
            span,
            is_end_inclusive: matches!(range.inclusion, RangeInclusion::Inclusive),
            done: false,
            one: Value::Int { val: 1, span },
            negative_one: Value::Int { val: -1, span },
        }
    }
}

impl Iterator for RangeIterator {
    type Item = Value;
    fn next(&mut self) -> Option<Self::Item> {
        use std::cmp::Ordering;
        if self.done {
            return None;
        }

        let ordering = if matches!(self.end, Value::Nothing { .. }) {
            Ordering::Less
        } else {
            match (&self.curr, &self.end) {
                (Value::Int { val: x, .. }, Value::Int { val: y, .. }) => x.cmp(y),
                // (Value::Float { val: x, .. }, Value::Float { val: y, .. }) => x.cmp(y),
                // (Value::Float { val: x, .. }, Value::Int { val: y, .. }) => x.cmp(y),
                // (Value::Int { val: x, .. }, Value::Float { val: y, .. }) => x.cmp(y),
                _ => {
                    self.done = true;
                    return Some(Value::Error {
                        error: ShellError::CannotCreateRange(self.span),
                    });
                }
            }
        };

        if self.moves_up
            && (ordering == Ordering::Less || self.is_end_inclusive && ordering == Ordering::Equal)
        {
            let next_value = self.curr.add(self.span, &self.one);

            let mut next = match next_value {
                Ok(result) => result,

                Err(error) => {
                    self.done = true;
                    return Some(Value::Error { error });
                }
            };
            std::mem::swap(&mut self.curr, &mut next);

            Some(next)
        } else if !self.moves_up
            && (ordering == Ordering::Greater
                || self.is_end_inclusive && ordering == Ordering::Equal)
        {
            let next_value = self.curr.add(self.span, &self.negative_one);

            let mut next = match next_value {
                Ok(result) => result,
                Err(error) => {
                    self.done = true;
                    return Some(Value::Error { error });
                }
            };
            std::mem::swap(&mut self.curr, &mut next);

            Some(next)
        } else {
            None
        }
    }
}
