use crate::value::Primitive;
use derive_new::new;
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
    pub fn min_u64(&self) -> u64 {
        self.from
            .0
            .item
            .as_u64(self.from.0.span)
            .unwrap_or(u64::MIN)
            .saturating_add(match self.from.1 {
                RangeInclusion::Inclusive => 0,
                RangeInclusion::Exclusive => 1,
            })
    }

    pub fn max_u64(&self) -> u64 {
        self.to
            .0
            .item
            .as_u64(self.to.0.span)
            .unwrap_or(u64::MAX)
            .saturating_sub(match self.to.1 {
                RangeInclusion::Inclusive => 0,
                RangeInclusion::Exclusive => 1,
            })
    }

    pub fn min_usize(&self) -> usize {
        self.from
            .0
            .item
            .as_usize(self.from.0.span)
            .unwrap_or(usize::MIN)
            .saturating_add(match self.from.1 {
                RangeInclusion::Inclusive => 0,
                RangeInclusion::Exclusive => 1,
            })
    }

    pub fn max_usize(&self) -> usize {
        self.to
            .0
            .item
            .as_usize(self.to.0.span)
            .unwrap_or(usize::MAX)
            .saturating_sub(match self.to.1 {
                RangeInclusion::Inclusive => 0,
                RangeInclusion::Exclusive => 1,
            })
    }
}
