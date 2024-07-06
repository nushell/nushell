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
    pub const EMPTY: Self = Interrupt { interrupt: None };

    pub fn new(ctrlc: Arc<AtomicBool>) -> Self {
        Self {
            interrupt: Some(ctrlc),
        }
    }

    pub const fn empty() -> Self {
        Self::EMPTY
    }

    #[inline]
    pub fn check(&self, span: Span) -> Result<(), ShellError> {
        #[inline]
        #[cold]
        fn interrupt_error(span: Span) -> Result<(), ShellError> {
            Err(ShellError::Interrupted { span })
        }

        if self.triggered() {
            interrupt_error(span)
        } else {
            Ok(())
        }
    }

    #[inline]
    pub fn triggered(&self) -> bool {
        self.interrupt
            .as_deref()
            .is_some_and(|b| b.load(Ordering::Relaxed))
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.interrupt.is_none()
    }

    pub(crate) fn reset(&self) {
        if let Some(interrupt) = &self.interrupt {
            interrupt.store(false, Ordering::Relaxed);
        }
    }
}
