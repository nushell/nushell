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
