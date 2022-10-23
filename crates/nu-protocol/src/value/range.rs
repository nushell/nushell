use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    sync::{atomic::AtomicBool, Arc},
};

/// A Range is an iterator over integers.
use crate::{
    ast::{RangeInclusion, RangeOperator},
    *,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Range {
    pub from: Value,
    pub incr: Value,
    pub to: Value,
    pub inclusion: RangeInclusion,
}

impl Range {
    pub fn new(
        expr_span: Span,
        from: Value,
        next: Value,
        to: Value,
        operator: &RangeOperator,
    ) -> Result<Range, ShellError> {
        // Select from & to values if they're not specified
        // TODO: Replace the placeholder values with proper min/max for range based on data type
        let from = if let Value::Nothing { .. } = from {
            Value::Int {
                val: 0i64,
                span: expr_span,
            }
        } else {
            from
        };

        let to = if let Value::Nothing { .. } = to {
            if let Ok(Value::Bool { val: true, .. }) = next.lt(expr_span, &from, expr_span) {
                Value::Int {
                    val: i64::MIN,
                    span: expr_span,
                }
            } else {
                Value::Int {
                    val: i64::MAX,
                    span: expr_span,
                }
            }
        } else {
            to
        };

        // Check if the range counts up or down
        let moves_up = matches!(
            from.lte(expr_span, &to, expr_span),
            Ok(Value::Bool { val: true, .. })
        );

        // Convert the next value into the inctement
        let incr = if let Value::Nothing { .. } = next {
            if moves_up {
                Value::Int {
                    val: 1i64,
                    span: expr_span,
                }
            } else {
                Value::Int {
                    val: -1i64,
                    span: expr_span,
                }
            }
        } else {
            next.sub(operator.next_op_span, &from, expr_span)?
        };

        let zero = Value::Int {
            val: 0i64,
            span: expr_span,
        };

        // Increment must be non-zero, otherwise we iterate forever
        if matches!(
            incr.eq(expr_span, &zero, expr_span),
            Ok(Value::Bool { val: true, .. })
        ) {
            return Err(ShellError::CannotCreateRange(expr_span));
        }

        // If to > from, then incr > 0, otherwise we iterate forever
        if let (Value::Bool { val: true, .. }, Value::Bool { val: false, .. }) = (
            to.gt(operator.span, &from, expr_span)?,
            incr.gt(operator.next_op_span, &zero, expr_span)?,
        ) {
            return Err(ShellError::CannotCreateRange(expr_span));
        }

        // If to < from, then incr < 0, otherwise we iterate forever
        if let (Value::Bool { val: true, .. }, Value::Bool { val: false, .. }) = (
            to.lt(operator.span, &from, expr_span)?,
            incr.lt(operator.next_op_span, &zero, expr_span)?,
        ) {
            return Err(ShellError::CannotCreateRange(expr_span));
        }

        Ok(Range {
            from,
            incr,
            to,
            inclusion: operator.inclusion,
        })
    }

    #[inline]
    fn moves_up(&self) -> bool {
        self.from <= self.to
    }

    pub fn is_end_inclusive(&self) -> bool {
        matches!(self.inclusion, RangeInclusion::Inclusive)
    }

    pub fn from(&self) -> Result<i64, ShellError> {
        self.from.as_integer()
    }

    pub fn to(&self) -> Result<i64, ShellError> {
        let to = self.to.as_integer()?;
        if self.is_end_inclusive() {
            Ok(to)
        } else {
            Ok(to - 1)
        }
    }

    pub fn contains(&self, item: &Value) -> bool {
        match (item.partial_cmp(&self.from), item.partial_cmp(&self.to)) {
            (Some(Ordering::Greater | Ordering::Equal), Some(Ordering::Less)) => self.moves_up(),
            (Some(Ordering::Less | Ordering::Equal), Some(Ordering::Greater)) => !self.moves_up(),
            (Some(_), Some(Ordering::Equal)) => self.is_end_inclusive(),
            (_, _) => false,
        }
    }

    pub fn into_range_iter(
        self,
        ctrlc: Option<Arc<AtomicBool>>,
    ) -> Result<RangeIterator, ShellError> {
        let span = self.from.span()?;

        Ok(RangeIterator::new(self, ctrlc, span))
    }
}

impl PartialOrd for Range {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.from.partial_cmp(&other.from) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        match self.incr.partial_cmp(&other.incr) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        match self.to.partial_cmp(&other.to) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.inclusion.partial_cmp(&other.inclusion)
    }
}

pub struct RangeIterator {
    curr: Value,
    end: Value,
    span: Span,
    is_end_inclusive: bool,
    moves_up: bool,
    incr: Value,
    done: bool,
    ctrlc: Option<Arc<AtomicBool>>,
}

impl RangeIterator {
    pub fn new(range: Range, ctrlc: Option<Arc<AtomicBool>>, span: Span) -> RangeIterator {
        let moves_up = range.moves_up();
        let is_end_inclusive = range.is_end_inclusive();

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
            moves_up,
            curr: start,
            end,
            span,
            is_end_inclusive,
            done: false,
            incr: range.incr,
            ctrlc,
        }
    }
}

impl Iterator for RangeIterator {
    type Item = Value;
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        if let Some(ctrlc) = &self.ctrlc {
            if ctrlc.load(core::sync::atomic::Ordering::SeqCst) {
                return None;
            }
        }

        let ordering = if matches!(self.end, Value::Nothing { .. }) {
            Some(Ordering::Less)
        } else {
            self.curr.partial_cmp(&self.end)
        };

        let ordering = if let Some(ord) = ordering {
            ord
        } else {
            self.done = true;
            return Some(Value::Error {
                error: ShellError::CannotCreateRange(self.span),
            });
        };

        let desired_ordering = if self.moves_up {
            Ordering::Less
        } else {
            Ordering::Greater
        };

        if (ordering == desired_ordering) || (self.is_end_inclusive && ordering == Ordering::Equal)
        {
            let next_value = self.curr.add(self.span, &self.incr, self.span);

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
