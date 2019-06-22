use crate::parser::{hir::Expression, Spanned};
use derive_new::new;
use getset::Getters;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Getters, new)]
#[get = "crate"]
pub struct Path {
    head: Expression,
    tail: Vec<Spanned<String>>,
}
