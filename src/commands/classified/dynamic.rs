use crate::parser::hir;
use derive_new::new;

#[derive(new, Debug, Eq, PartialEq)]
pub(crate) struct Command {
    pub(crate) args: hir::Call,
}
