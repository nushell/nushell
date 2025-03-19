use crate::{ShellError, Spanned};
use serde::{Deserialize, Serialize};

pub use nu_utils::{Interrupted, Signals};

impl From<Spanned<Interrupted>> for ShellError {
    fn from(value: Spanned<Interrupted>) -> Self {
        Self::Interrupted { span: value.span }
    }
}

/// The types of things that can be signaled. It's anticipated this will change as we learn more
/// about how we'd like signals to be handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalAction {
    Interrupt,
    Reset,
}
