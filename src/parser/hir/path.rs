use crate::parser::hir::Expression;
use crate::prelude::*;
use derive_new::new;
use getset::{Getters, MutGetters};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(
    Debug,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Getters,
    MutGetters,
    Serialize,
    Deserialize,
    new,
)]
#[get = "pub(crate)"]
pub struct Path {
    head: Expression,
    #[get_mut = "pub(crate)"]
    tail: Vec<Spanned<String>>,
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.head)?;

        for entry in &self.tail {
            write!(f, ".{}", entry.item)?;
        }

        Ok(())
    }
}

impl Path {
    pub(crate) fn parts(self) -> (Expression, Vec<Spanned<String>>) {
        (self.head, self.tail)
    }
}

impl ToDebug for Path {
    fn fmt_debug(&self, f: &mut fmt::Formatter, source: &str) -> fmt::Result {
        write!(f, "{}", self.head.debug(source))?;

        for part in &self.tail {
            write!(f, ".{}", part.item)?;
        }

        Ok(())
    }
}
