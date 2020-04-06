use derive_new::new;
use nu_parser::hir;

#[derive(new, Debug)]
pub(crate) struct Command {
    pub(crate) args: hir::Call,
}
