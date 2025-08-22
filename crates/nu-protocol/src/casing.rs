use std::cmp::Ordering;

use nu_utils::IgnoreCaseExt;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Casing {
    #[default]
    Sensitive,
    Insensitive,
}

pub(crate) mod private {
    pub trait Seal {}
}

pub trait CasingCmp: private::Seal + 'static {
    fn eq(lhs: &str, rhs: &str) -> bool;

    fn cmp(lhs: &str, rhs: &str) -> Ordering;
}

pub struct CaseSensitive;
pub struct CaseInsensitive;

impl private::Seal for CaseSensitive {}
impl private::Seal for CaseInsensitive {}

impl CasingCmp for CaseSensitive {
    #[inline]
    fn eq(lhs: &str, rhs: &str) -> bool {
        lhs == rhs
    }

    #[inline]
    fn cmp(lhs: &str, rhs: &str) -> Ordering {
        lhs.cmp(rhs)
    }
}

impl CasingCmp for CaseInsensitive {
    #[inline]
    fn eq(lhs: &str, rhs: &str) -> bool {
        lhs.eq_ignore_case(rhs)
    }

    #[inline]
    fn cmp(lhs: &str, rhs: &str) -> Ordering {
        lhs.cmp_ignore_case(rhs)
    }
}
