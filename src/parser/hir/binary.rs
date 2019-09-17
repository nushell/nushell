use crate::parser::{hir::Expression, Operator};
use crate::prelude::*;
use crate::Tagged;
use derive_new::new;
use getset::Getters;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Getters, Serialize, Deserialize, new,
)]
#[get = "pub(crate)"]
pub struct Binary {
    left: Expression,
    op: Tagged<Operator>,
    right: Expression,
}

impl fmt::Display for Binary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({} {} {})", self.op.as_str(), self.left, self.right)
    }
}

impl ToDebug for Binary {
    fn fmt_debug(&self, f: &mut fmt::Formatter, source: &str) -> fmt::Result {
        write!(f, "{}", self.left.debug(source))?;
        write!(f, " {} ", self.op.debug(source))?;
        write!(f, "{}", self.right.debug(source))?;

        Ok(())
    }
}
