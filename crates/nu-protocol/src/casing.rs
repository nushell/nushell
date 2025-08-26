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

pub trait CaseSensitivity: private::Seal + 'static {
    fn eq(lhs: &str, rhs: &str) -> bool;

    fn cmp(lhs: &str, rhs: &str) -> Ordering;
}

pub struct CaseSensitive;
pub struct CaseInsensitive;

impl private::Seal for CaseSensitive {}
impl private::Seal for CaseInsensitive {}

impl CaseSensitivity for CaseSensitive {
    #[inline]
    fn eq(lhs: &str, rhs: &str) -> bool {
        lhs == rhs
    }

    #[inline]
    fn cmp(lhs: &str, rhs: &str) -> Ordering {
        lhs.cmp(rhs)
    }
}

impl CaseSensitivity for CaseInsensitive {
    #[inline]
    fn eq(lhs: &str, rhs: &str) -> bool {
        lhs.eq_ignore_case(rhs)
    }

    #[inline]
    fn cmp(lhs: &str, rhs: &str) -> Ordering {
        lhs.cmp_ignore_case(rhs)
    }
}

/// Wraps `Self` in a type that affects the case sensitivity of operations
///
/// Using methods of [`CaseSensitivity`] in `Wrapper` implementations are not mandotary.
/// They are provided mostly for convenience and to have a common implementation for comparisons
pub trait WrapCased {
    /// Wrapper type generic over case sensitivity.
    type Wrapper<S: CaseSensitivity>;

    fn case_sensitive(self) -> Self::Wrapper<CaseSensitive>;
    fn case_insensitive(self) -> Self::Wrapper<CaseInsensitive>;
}
