use crate::{ShellError, Span};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[derive(Debug, Clone)]
pub struct Interrupt {
    interrupt: Option<Arc<AtomicBool>>,
}

impl Interrupt {
    pub fn new(ctrlc: Arc<AtomicBool>) -> Self {
        Self {
            interrupt: Some(ctrlc),
        }
    }

    pub const fn empty() -> Self {
        Self { interrupt: None }
    }

    #[inline]
    pub fn poll(&self, span: Span) -> Result<(), ShellError> {
        if self
            .interrupt
            .as_deref()
            .is_some_and(|b| b.load(Ordering::Relaxed))
        {
            Err(ShellError::Interrupted { span })
        } else {
            Ok(())
        }
    }
}
