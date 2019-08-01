use crate::parser::{hir::Expression, Operator};
use crate::Tagged;
use derive_new::new;
use getset::Getters;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Getters, new)]
#[get = "crate"]
pub struct Binary {
    left: Expression,
    op: Tagged<Operator>,
    right: Expression,
}
