use crate::parser::{hir::Expression, Operator, Spanned};
use derive_new::new;
use getset::Getters;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Getters, new)]
#[get = "crate"]
pub struct Binary {
    left: Expression,
    op: Spanned<Operator>,
    right: Expression,
}
