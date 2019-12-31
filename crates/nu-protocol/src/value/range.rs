use crate::value::Primitive;
use derive_new::new;
use nu_source::{b, DebugDocBuilder, Spanned};
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub enum RangeInclusion {
    Inclusive,
    Exclusive,
}

impl RangeInclusion {
    pub fn debug_left_bracket(self) -> DebugDocBuilder {
        b::delimiter(match self {
            RangeInclusion::Exclusive => "(",
            RangeInclusion::Inclusive => "[",
        })
    }

    pub fn debug_right_bracket(self) -> DebugDocBuilder {
        b::delimiter(match self {
            RangeInclusion::Exclusive => ")",
            RangeInclusion::Inclusive => "]",
        })
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize, new)]
pub struct Range {
    pub from: (Spanned<Primitive>, RangeInclusion),
    pub to: (Spanned<Primitive>, RangeInclusion),
}
