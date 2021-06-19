use derive_new::new;
use nu_protocol::hir;

#[derive(new, Debug)]
pub(crate) struct Command {
    pub(crate) args: hir::Call,
}
