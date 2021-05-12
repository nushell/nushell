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
    }
}
