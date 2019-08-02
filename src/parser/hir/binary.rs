use crate::parser::{hir::Expression, Operator, Spanned};
use crate::prelude::*;
use derive_new::new;
use getset::Getters;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Getters, Serialize, Deserialize, new,
)]
#[get = "crate"]
pub struct Binary {
    left: Expression,
    op: Spanned<Operator>,
    right: Expression,
}

impl ToDebug for Binary {
    fn fmt_debug(&self, f: &mut fmt::Formatter, source: &str) -> fmt::Result {
        write!(f, "{}", self.left.debug(source))?;
        write!(f, " {} ", self.op.debug(source))?;
        write!(f, "{}", self.right.debug(source))?;

        Ok(())
    }
}
