//! A Range is an iterator over integers or floats.

use crate::{ShellError, Signals, Span, Value, ast::RangeInclusion};
use core::ops::Bound;
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, fmt::Display};

mod int_range {
    use crate::{FromValue, ShellError, Signals, Span, Value, ast::RangeInclusion};
    use serde::{Deserialize, Serialize};
    use std::{cmp::Ordering, fmt::Display, ops::Bound};

    use super::Range;

    #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
    pub struct IntRange {
        pub(crate) start: i64,
        pub(crate) step: i64,
        pub(crate) end: Bound<i64>,
    }

    impl IntRange {
        pub fn new(
            start: Value,
            next: Value,
            end: Value,
            inclusion: RangeInclusion,
            span: Span,
        ) -> Result<Self, ShellError> {
            fn to_int(value: Value) -> Result<Option<i64>, ShellError> {
                match value {
                    Value::Int { val, .. } => Ok(Some(val)),
                    Value::Nothing { .. } => Ok(None),
                    val => Err(ShellError::CantConvert {
                        to_type: "int".into(),
                        from_type: val.get_type().to_string(),
                        span: val.span(),
                        help: None,
                    }),
                }
            }

            let start = to_int(start)?.unwrap_or(0);

            let next_span = next.span();
            let next = to_int(next)?;
            if next.is_some_and(|next| next == start) {
                return Err(ShellError::CannotCreateRange { span: next_span });
            }

            let end = to_int(end)?;

            let step = match (next, end) {
                (Some(next), Some(end)) => {
                    if (next < start) != (end < start) {
                        return Err(ShellError::CannotCreateRange { span });
                    }
                    next - start
                }
                (Some(next), None) => next - start,
                (None, Some(end)) => {
                    if end < start {
                        -1
                    } else {
                        1
                    }
                }
                (None, None) => 1,
            };

            let end = if let Some(end) = end {
                match inclusion {
                    RangeInclusion::Inclusive => Bound::Included(end),
                    RangeInclusion::RightExclusive => Bound::Excluded(end),
                }
            } else {
                Bound::Unbounded
            };

            Ok(Self { start, step, end })
        }

        pub fn start(&self) -> i64 {
            self.start
        }

        // Resolves the absolute start position given the length of the input value
        pub fn absolute_start(&self, len: u64) -> u64 {
            match self.start {
                start if start < 0 => len.saturating_sub(start.unsigned_abs()),
                start => len.min(start.unsigned_abs()),
            }
        }

        /// Returns the distance between the start and end of the range
        /// The result will always be 0 or positive
        pub fn distance(&self) -> Bound<u64> {
            match self.end {
                Bound::Unbounded => Bound::Unbounded,
                Bound::Included(end) | Bound::Excluded(end) if self.start > end => {
                    Bound::Excluded(0)
                }
                Bound::Included(end) => Bound::Included((end - self.start) as u64),
                Bound::Excluded(end) => Bound::Excluded((end - self.start) as u64),
            }
        }

        pub fn end(&self) -> Bound<i64> {
            self.end
        }

        pub fn absolute_end(&self, len: u64) -> Bound<u64> {
            match self.end {
                Bound::Unbounded => Bound::Unbounded,
                Bound::Included(i) => match i {
                    _ if len == 0 => Bound::Excluded(0),
                    i if i < 0 => Bound::Excluded(len.saturating_sub((i + 1).unsigned_abs())),
                    i => Bound::Included((len.saturating_sub(1)).min(i.unsigned_abs())),
                },
                Bound::Excluded(i) => Bound::Excluded(match i {
                    i if i < 0 => len.saturating_sub(i.unsigned_abs()),
                    i => len.min(i.unsigned_abs()),
                }),
            }
        }

        pub fn absolute_bounds(&self, len: usize) -> (usize, Bound<usize>) {
            let start = self.absolute_start(len as u64) as usize;
            let end = self.absolute_end(len as u64).map(|e| e as usize);
            match end {
                Bound::Excluded(end) | Bound::Included(end) if end < start => {
                    (start, Bound::Excluded(start))
                }
                Bound::Excluded(end) => (start, Bound::Excluded(end)),
                Bound::Included(end) => (start, Bound::Included(end)),
                Bound::Unbounded => (start, Bound::Unbounded),
            }
        }

        pub fn step(&self) -> i64 {
            self.step
        }

        pub fn is_unbounded(&self) -> bool {
            self.end == Bound::Unbounded
        }

        pub fn is_relative(&self) -> bool {
            self.is_start_relative() || self.is_end_relative()
        }

        pub fn is_start_relative(&self) -> bool {
            self.start < 0
        }

        pub fn is_end_relative(&self) -> bool {
            match self.end {
                Bound::Included(end) | Bound::Excluded(end) => end < 0,
                _ => false,
            }
        }

        pub fn contains(&self, value: i64) -> bool {
            if self.step < 0 {
                // Decreasing range
                if value > self.start {
                    return false;
                }
                match self.end {
                    Bound::Included(end) if value < end => return false,
                    Bound::Excluded(end) if value <= end => return false,
                    _ => {}
                };
            } else {
                // Increasing range
                if value < self.start {
                    return false;
                }
                match self.end {
                    Bound::Included(end) if value > end => return false,
                    Bound::Excluded(end) if value >= end => return false,
                    _ => {}
                };
            }
            (value - self.start) % self.step == 0
        }

        pub fn into_range_iter(self, signals: Signals) -> Iter {
            Iter {
                current: Some(self.start),
                step: self.step,
                end: self.end,
                signals,
            }
        }
    }

    impl Ord for IntRange {
        fn cmp(&self, other: &Self) -> Ordering {
            // Ranges are compared roughly according to their list representation.
            // Compare in order:
            // - the head element (start)
            // - the tail elements (step)
            // - the length (end)
            self.start
                .cmp(&other.start)
                .then(self.step.cmp(&other.step))
                .then_with(|| match (self.end, other.end) {
                    (Bound::Included(l), Bound::Included(r))
                    | (Bound::Excluded(l), Bound::Excluded(r)) => {
                        let ord = l.cmp(&r);
                        if self.step < 0 { ord.reverse() } else { ord }
                    }
                    (Bound::Included(l), Bound::Excluded(r)) => match l.cmp(&r) {
                        Ordering::Equal => Ordering::Greater,
                        ord if self.step < 0 => ord.reverse(),
                        ord => ord,
                    },
                    (Bound::Excluded(l), Bound::Included(r)) => match l.cmp(&r) {
                        Ordering::Equal => Ordering::Less,
                        ord if self.step < 0 => ord.reverse(),
                        ord => ord,
                    },
                    (Bound::Included(_), Bound::Unbounded) => Ordering::Less,
                    (Bound::Excluded(_), Bound::Unbounded) => Ordering::Less,
                    (Bound::Unbounded, Bound::Included(_)) => Ordering::Greater,
                    (Bound::Unbounded, Bound::Excluded(_)) => Ordering::Greater,
                    (Bound::Unbounded, Bound::Unbounded) => Ordering::Equal,
                })
        }
    }

    impl PartialOrd for IntRange {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    impl PartialEq for IntRange {
        fn eq(&self, other: &Self) -> bool {
            self.start == other.start && self.step == other.step && self.end == other.end
        }
    }

    impl Eq for IntRange {}

    impl Display for IntRange {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}..", self.start)?;
            if self.step != 1 {
                write!(f, "{}..", self.start + self.step)?;
            }
            match self.end {
                Bound::Included(end) => write!(f, "{end}"),
                Bound::Excluded(end) => write!(f, "<{end}"),
                Bound::Unbounded => Ok(()),
            }
        }
    }

    impl FromValue for IntRange {
        fn from_value(v: Value) -> Result<Self, ShellError> {
            let span = v.span();
            let range = Range::from_value(v)?;
            match range {
                Range::IntRange(v) => Ok(v),
                Range::FloatRange(_) => Err(ShellError::TypeMismatch {
                    err_message: "expected an int range".into(),
                    span,
                }),
            }
        }
    }

    pub struct Iter {
        current: Option<i64>,
        step: i64,
        end: Bound<i64>,
        signals: Signals,
    }

    impl Iterator for Iter {
        type Item = i64;

        fn next(&mut self) -> Option<Self::Item> {
            if let Some(current) = self.current {
                let not_end = match (self.step < 0, self.end) {
                    (true, Bound::Included(end)) => current >= end,
                    (true, Bound::Excluded(end)) => current > end,
                    (false, Bound::Included(end)) => current <= end,
                    (false, Bound::Excluded(end)) => current < end,
                    (_, Bound::Unbounded) => true, // will stop once integer overflows
                };

                if not_end && !self.signals.interrupted() {
                    self.current = current.checked_add(self.step);
                    Some(current)
                } else {
                    self.current = None;
                    None
                }
            } else {
                None
            }
        }
    }
}

mod float_range {
    use crate::{IntRange, Range, ShellError, Signals, Span, Value, ast::RangeInclusion};
    use nu_utils::ObviousFloat;
    use serde::{Deserialize, Serialize};
    use std::{cmp::Ordering, fmt::Display, ops::Bound};

    #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
    pub struct FloatRange {
        pub(crate) start: f64,
        pub(crate) step: f64,
        pub(crate) end: Bound<f64>,
    }

    impl FloatRange {
        pub fn new(
            start: Value,
            next: Value,
            end: Value,
            inclusion: RangeInclusion,
            span: Span,
        ) -> Result<Self, ShellError> {
            fn to_float(value: Value) -> Result<Option<f64>, ShellError> {
                match value {
                    Value::Float { val, .. } => Ok(Some(val)),
                    Value::Int { val, .. } => Ok(Some(val as f64)),
                    Value::Nothing { .. } => Ok(None),
                    val => Err(ShellError::CantConvert {
                        to_type: "float".into(),
                        from_type: val.get_type().to_string(),
                        span: val.span(),
                        help: None,
                    }),
                }
            }

            // `start` must be finite (not NaN or infinity).
            // `next` must be finite and not equal to `start`.
            // `end` must not be NaN (but can be infinite).
            //
            // TODO: better error messages for the restrictions above

            let start_span = start.span();
            let start = to_float(start)?.unwrap_or(0.0);
            if !start.is_finite() {
                return Err(ShellError::CannotCreateRange { span: start_span });
            }

            let end_span = end.span();
            let end = to_float(end)?;
            if end.is_some_and(f64::is_nan) {
                return Err(ShellError::CannotCreateRange { span: end_span });
            }

            let next_span = next.span();
            let next = to_float(next)?;
            if next.is_some_and(|next| next == start || !next.is_finite()) {
                return Err(ShellError::CannotCreateRange { span: next_span });
            }

            let step = match (next, end) {
                (Some(next), Some(end)) => {
                    if (next < start) != (end < start) {
                        return Err(ShellError::CannotCreateRange { span });
                    }
                    next - start
                }
                (Some(next), None) => next - start,
                (None, Some(end)) => {
                    if end < start {
                        -1.0
                    } else {
                        1.0
                    }
                }
                (None, None) => 1.0,
            };

            let end = if let Some(end) = end {
                if end.is_infinite() {
                    Bound::Unbounded
                } else {
                    match inclusion {
                        RangeInclusion::Inclusive => Bound::Included(end),
                        RangeInclusion::RightExclusive => Bound::Excluded(end),
                    }
                }
            } else {
                Bound::Unbounded
            };

            Ok(Self { start, step, end })
        }

        pub fn start(&self) -> f64 {
            self.start
        }

        pub fn end(&self) -> Bound<f64> {
            self.end
        }

        pub fn step(&self) -> f64 {
            self.step
        }

        pub fn is_unbounded(&self) -> bool {
            self.end == Bound::Unbounded
        }

        pub fn contains(&self, value: f64) -> bool {
            if self.step < 0.0 {
                // Decreasing range
                if value > self.start {
                    return false;
                }
                match self.end {
                    Bound::Included(end) if value <= end => return false,
                    Bound::Excluded(end) if value < end => return false,
                    _ => {}
                };
            } else {
                // Increasing range
                if value < self.start {
                    return false;
                }
                match self.end {
                    Bound::Included(end) if value >= end => return false,
                    Bound::Excluded(end) if value > end => return false,
                    _ => {}
                };
            }
            ((value - self.start) % self.step).abs() < f64::EPSILON
        }

        pub fn into_range_iter(self, signals: Signals) -> Iter {
            Iter {
                start: self.start,
                step: self.step,
                end: self.end,
                iter: Some(0),
                signals,
            }
        }
    }

    impl Ord for FloatRange {
        fn cmp(&self, other: &Self) -> Ordering {
            fn float_cmp(a: f64, b: f64) -> Ordering {
                // There is no way a `FloatRange` can have NaN values:
                // - `FloatRange::new` ensures no values are NaN.
                // - `From<IntRange> for FloatRange` cannot give NaNs either.
                // - There are no other ways to create a `FloatRange`.
                // - There is no way to modify values of a `FloatRange`.
                a.partial_cmp(&b).expect("not NaN")
            }

            // Ranges are compared roughly according to their list representation.
            // Compare in order:
            // - the head element (start)
            // - the tail elements (step)
            // - the length (end)
            float_cmp(self.start, other.start)
                .then(float_cmp(self.step, other.step))
                .then_with(|| match (self.end, other.end) {
                    (Bound::Included(l), Bound::Included(r))
                    | (Bound::Excluded(l), Bound::Excluded(r)) => {
                        let ord = float_cmp(l, r);
                        if self.step < 0.0 { ord.reverse() } else { ord }
                    }
                    (Bound::Included(l), Bound::Excluded(r)) => match float_cmp(l, r) {
                        Ordering::Equal => Ordering::Greater,
                        ord if self.step < 0.0 => ord.reverse(),
                        ord => ord,
                    },
                    (Bound::Excluded(l), Bound::Included(r)) => match float_cmp(l, r) {
                        Ordering::Equal => Ordering::Less,
                        ord if self.step < 0.0 => ord.reverse(),
                        ord => ord,
                    },
                    (Bound::Included(_), Bound::Unbounded) => Ordering::Less,
                    (Bound::Excluded(_), Bound::Unbounded) => Ordering::Less,
                    (Bound::Unbounded, Bound::Included(_)) => Ordering::Greater,
                    (Bound::Unbounded, Bound::Excluded(_)) => Ordering::Greater,
                    (Bound::Unbounded, Bound::Unbounded) => Ordering::Equal,
                })
        }
    }

    impl PartialOrd for FloatRange {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    impl PartialEq for FloatRange {
        fn eq(&self, other: &Self) -> bool {
            self.start == other.start && self.step == other.step && self.end == other.end
        }
    }

    impl Eq for FloatRange {}

    impl Display for FloatRange {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}..", ObviousFloat(self.start))?;
            if self.step != 1f64 {
                write!(f, "{}..", ObviousFloat(self.start + self.step))?;
            }
            match self.end {
                Bound::Included(end) => write!(f, "{}", ObviousFloat(end)),
                Bound::Excluded(end) => write!(f, "<{}", ObviousFloat(end)),
                Bound::Unbounded => Ok(()),
            }
        }
    }

    impl From<IntRange> for FloatRange {
        fn from(range: IntRange) -> Self {
            Self {
                start: range.start as f64,
                step: range.step as f64,
                end: match range.end {
                    Bound::Included(b) => Bound::Included(b as f64),
                    Bound::Excluded(b) => Bound::Excluded(b as f64),
                    Bound::Unbounded => Bound::Unbounded,
                },
            }
        }
    }

    impl From<Range> for FloatRange {
        fn from(range: Range) -> Self {
            match range {
                Range::IntRange(range) => range.into(),
                Range::FloatRange(range) => range,
            }
        }
    }

    pub struct Iter {
        start: f64,
        step: f64,
        end: Bound<f64>,
        iter: Option<u64>,
        signals: Signals,
    }

    impl Iterator for Iter {
        type Item = f64;

        fn next(&mut self) -> Option<Self::Item> {
            if let Some(iter) = self.iter {
                let current = self.start + self.step * iter as f64;

                let not_end = match (self.step < 0.0, self.end) {
                    (true, Bound::Included(end)) => current >= end,
                    (true, Bound::Excluded(end)) => current > end,
                    (false, Bound::Included(end)) => current <= end,
                    (false, Bound::Excluded(end)) => current < end,
                    (_, Bound::Unbounded) => current.is_finite(),
                };

                if not_end && !self.signals.interrupted() {
                    self.iter = iter.checked_add(1);
                    Some(current)
                } else {
                    self.iter = None;
                    None
                }
            } else {
                None
            }
        }
    }
}

pub use float_range::FloatRange;
pub use int_range::IntRange;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Range {
    IntRange(IntRange),
    FloatRange(FloatRange),
}

impl Range {
    pub fn new(
        start: Value,
        next: Value,
        end: Value,
        inclusion: RangeInclusion,
        span: Span,
    ) -> Result<Self, ShellError> {
        // promote to float range if any Value is float
        if matches!(start, Value::Float { .. })
            || matches!(next, Value::Float { .. })
            || matches!(end, Value::Float { .. })
        {
            FloatRange::new(start, next, end, inclusion, span).map(Self::FloatRange)
        } else {
            IntRange::new(start, next, end, inclusion, span).map(Self::IntRange)
        }
    }

    pub fn contains(&self, value: &Value) -> bool {
        match (self, value) {
            (Self::IntRange(range), Value::Int { val, .. }) => range.contains(*val),
            (Self::IntRange(range), Value::Float { val, .. }) => {
                FloatRange::from(*range).contains(*val)
            }
            (Self::FloatRange(range), Value::Int { val, .. }) => range.contains(*val as f64),
            (Self::FloatRange(range), Value::Float { val, .. }) => range.contains(*val),
            _ => false,
        }
    }

    pub fn is_bounded(&self) -> bool {
        match self {
            Range::IntRange(range) => range.end() != Bound::<i64>::Unbounded,
            Range::FloatRange(range) => range.end() != Bound::<f64>::Unbounded,
        }
    }

    pub fn into_range_iter(self, span: Span, signals: Signals) -> Iter {
        match self {
            Range::IntRange(range) => Iter::IntIter(range.into_range_iter(signals), span),
            Range::FloatRange(range) => Iter::FloatIter(range.into_range_iter(signals), span),
        }
    }
}

impl Ord for Range {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Range::IntRange(l), Range::IntRange(r)) => l.cmp(r),
            (Range::FloatRange(l), Range::FloatRange(r)) => l.cmp(r),
            (Range::IntRange(int), Range::FloatRange(float)) => FloatRange::from(*int).cmp(float),
            (Range::FloatRange(float), Range::IntRange(int)) => float.cmp(&FloatRange::from(*int)),
        }
    }
}

impl PartialOrd for Range {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Range {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Range::IntRange(l), Range::IntRange(r)) => l == r,
            (Range::FloatRange(l), Range::FloatRange(r)) => l == r,
            (Range::IntRange(int), Range::FloatRange(float)) => FloatRange::from(*int) == *float,
            (Range::FloatRange(float), Range::IntRange(int)) => *float == FloatRange::from(*int),
        }
    }
}

impl Eq for Range {}

impl Display for Range {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Range::IntRange(range) => write!(f, "{range}"),
            Range::FloatRange(range) => write!(f, "{range}"),
        }
    }
}

impl From<IntRange> for Range {
    fn from(range: IntRange) -> Self {
        Self::IntRange(range)
    }
}

impl From<FloatRange> for Range {
    fn from(range: FloatRange) -> Self {
        Self::FloatRange(range)
    }
}

pub enum Iter {
    IntIter(int_range::Iter, Span),
    FloatIter(float_range::Iter, Span),
}

impl Iterator for Iter {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Iter::IntIter(iter, span) => iter.next().map(|val| Value::int(val, *span)),
            Iter::FloatIter(iter, span) => iter.next().map(|val| Value::float(val, *span)),
        }
    }
}
