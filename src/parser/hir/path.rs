use crate::parser::hir::Expression;
use crate::Tagged;
use derive_new::new;
use getset::Getters;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Getters, new)]
#[get = "crate"]
pub struct Path {
    head: Expression,
    tail: Vec<Tagged<String>>,
}
