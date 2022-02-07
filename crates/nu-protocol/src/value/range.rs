<<<<<<< HEAD
use crate::value::Primitive;
use derive_new::new;
use nu_errors::ShellError;
use nu_source::{DbgDocBldr, DebugDocBuilder, Spanned};
use serde::{Deserialize, Serialize};

/// The two types of ways to include a range end. Inclusive means to include the value (eg 1..3 inclusive would include the 3 value).
/// Exclusive excludes the value (eg 1..3 exclusive does not include 3 value)
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub enum RangeInclusion {
    Inclusive,
    Exclusive,
}

impl RangeInclusion {
    /// Get a RangeInclusion left bracket ready for pretty printing
    pub fn debug_left_bracket(self) -> DebugDocBuilder {
        DbgDocBldr::delimiter(match self {
            RangeInclusion::Exclusive => "(",
            RangeInclusion::Inclusive => "[",
        })
    }

    /// Get a RangeInclusion right bracket ready for pretty printing
    pub fn debug_right_bracket(self) -> DebugDocBuilder {
        DbgDocBldr::delimiter(match self {
            RangeInclusion::Exclusive => ")",
            RangeInclusion::Inclusive => "]",
        })
    }
}

/// The range definition, holding the starting and end point of the range
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize, new)]
pub struct Range {
    pub from: (Spanned<Primitive>, RangeInclusion),
    pub to: (Spanned<Primitive>, RangeInclusion),
}

impl Range {
    pub fn min_u64(&self) -> Result<u64, ShellError> {
        let (from, range_incl) = &self.from;

        let minval = if let Primitive::Nothing = from.item {
            u64::MIN
        } else {
            from.item.as_u64(from.span)?
        };

        match range_incl {
            RangeInclusion::Inclusive => Ok(minval),
            RangeInclusion::Exclusive => Ok(minval.saturating_add(1)),
        }
    }

    pub fn max_u64(&self) -> Result<u64, ShellError> {
        let (to, range_incl) = &self.to;

        let maxval = if let Primitive::Nothing = to.item {
            u64::MAX
        } else {
            to.item.as_u64(to.span)?
        };

        match range_incl {
            RangeInclusion::Inclusive => Ok(maxval),
            RangeInclusion::Exclusive => Ok(maxval.saturating_sub(1)),
        }
    }

    pub fn min_i64(&self) -> Result<i64, ShellError> {
        let (from, range_incl) = &self.from;

        let minval = if let Primitive::Nothing = from.item {
            0
        } else {
            from.item.as_i64(from.span)?
        };

        match range_incl {
            RangeInclusion::Inclusive => Ok(minval),
            RangeInclusion::Exclusive => Ok(minval.saturating_add(1)),
        }
    }

    pub fn max_i64(&self) -> Result<i64, ShellError> {
        let (to, range_incl) = &self.to;

        let maxval = if let Primitive::Nothing = to.item {
            i64::MAX
        } else {
            to.item.as_i64(to.span)?
        };

        match range_incl {
            RangeInclusion::Inclusive => Ok(maxval),
            RangeInclusion::Exclusive => Ok(maxval.saturating_sub(1)),
        }
    }

    pub fn min_usize(&self) -> Result<usize, ShellError> {
        let (from, range_incl) = &self.from;

        let minval = if let Primitive::Nothing = from.item {
            usize::MIN
        } else {
            from.item.as_usize(from.span)?
        };

        match range_incl {
            RangeInclusion::Inclusive => Ok(minval),
            RangeInclusion::Exclusive => Ok(minval.saturating_add(1)),
        }
    }

    pub fn max_usize(&self) -> Result<usize, ShellError> {
        let (to, range_incl) = &self.to;

        let maxval = if let Primitive::Nothing = to.item {
            usize::MAX
        } else {
            to.item.as_usize(to.span)?
        };

        match range_incl {
            RangeInclusion::Inclusive => Ok(maxval),
            RangeInclusion::Exclusive => Ok(maxval.saturating_sub(1)),
        }
    }

    pub fn min_f64(&self) -> Result<f64, ShellError> {
        let from = &self.from.0;

        if let Primitive::Nothing = from.item {
            Ok(f64::MIN)
        } else {
            Ok(from.item.as_f64(from.span)?)
        }

        // How would inclusive vs. exclusive range work here?
    }

    pub fn max_f64(&self) -> Result<f64, ShellError> {
        let to = &self.to.0;

        if let Primitive::Nothing = to.item {
            Ok(f64::MAX)
        } else {
            Ok(to.item.as_f64(to.span)?)
        }

        // How would inclusive vs. exclusive range work here?
=======
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

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
            if let Ok(Value::Bool { val: true, .. }) = next.lt(expr_span, &from) {
                Value::Int {
                    val: -100i64,
                    span: expr_span,
                }
            } else {
                Value::Int {
                    val: 100i64,
                    span: expr_span,
                }
            }
        } else {
            to
        };

        // Check if the range counts up or down
        let moves_up = matches!(from.lte(expr_span, &to), Ok(Value::Bool { val: true, .. }));

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
            next.sub(operator.next_op_span, &from)?
        };

        let zero = Value::Int {
            val: 0i64,
            span: expr_span,
        };

        // Increment must be non-zero, otherwise we iterate forever
        if matches!(incr.eq(expr_span, &zero), Ok(Value::Bool { val: true, .. })) {
            return Err(ShellError::CannotCreateRange(expr_span));
        }

        // If to > from, then incr > 0, otherwise we iterate forever
        if let (Value::Bool { val: true, .. }, Value::Bool { val: false, .. }) = (
            to.gt(operator.span, &from)?,
            incr.gt(operator.next_op_span, &zero)?,
        ) {
            return Err(ShellError::CannotCreateRange(expr_span));
        }

        // If to < from, then incr < 0, otherwise we iterate forever
        if let (Value::Bool { val: true, .. }, Value::Bool { val: false, .. }) = (
            to.lt(operator.span, &from)?,
            incr.lt(operator.next_op_span, &zero)?,
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

    #[inline]
    fn is_end_inclusive(&self) -> bool {
        matches!(self.inclusion, RangeInclusion::Inclusive)
    }

    pub fn contains(&self, item: &Value) -> bool {
        match (item.partial_cmp(&self.from), item.partial_cmp(&self.to)) {
            (Some(Ordering::Greater | Ordering::Equal), Some(Ordering::Less)) => self.moves_up(),
            (Some(Ordering::Less | Ordering::Equal), Some(Ordering::Greater)) => !self.moves_up(),
            (Some(_), Some(Ordering::Equal)) => self.is_end_inclusive(),
            (_, _) => false,
        }
    }

    pub fn into_range_iter(self) -> Result<RangeIterator, ShellError> {
        let span = self.from.span()?;

        Ok(RangeIterator::new(self, span))
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
}

impl RangeIterator {
    pub fn new(range: Range, span: Span) -> RangeIterator {
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
        }
    }
}

impl Iterator for RangeIterator {
    type Item = Value;
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
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
            let next_value = self.curr.add(self.span, &self.incr);

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
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
    }
}
