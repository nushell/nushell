use derive_new::new;
use nu_parser::hir;

#[derive(new, Debug, Eq, PartialEq)]
pub(crate) struct Command {
    pub(crate) args: hir::Call,
}
