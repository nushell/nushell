use crate::parser::{hir::Expression, Spanned};
use crate::prelude::*;
use derive_new::new;
use getset::Getters;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Getters, Serialize, Deserialize, new,
)]
#[get = "crate"]
pub struct Path {
    head: Expression,
    tail: Vec<Spanned<String>>,
}

impl ToDebug for Path {
    fn fmt_debug(&self, f: &mut fmt::Formatter, source: &str) -> fmt::Result {
        write!(f, "{}", self.head.debug(source))?;

        for part in &self.tail {
            write!(f, ".{}", part.item())?;
        }

        Ok(())
    }
}
