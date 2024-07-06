use crate::{ShellError, Span};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

/// Controls the execution of nushell code.
///
/// For now, the only purpose of this struct is to check for interruption (ctrl+c or SIGINT).
#[derive(Debug, Clone)]
pub struct Interrupt {
    interrupt: Option<Arc<AtomicBool>>,
}

impl Interrupt {
    /// An [`Interrupt`] that is not hooked up to any event/interrupt source.
    ///
    /// So, this [`Interrupt`] will never be triggered.
    pub const EMPTY: Self = Interrupt { interrupt: None };

    /// Create a new [`Interrupt`] with `ctrlc` as the interrupt source.
    ///
    /// Once `ctrlc` is set to `true`, [`check`](Self::check) will error
    /// and [`triggered`](Self::triggered) will return `true`.
    pub fn new(ctrlc: Arc<AtomicBool>) -> Self {
        Self {
            interrupt: Some(ctrlc),
        }
    }

    /// Create [`Interrupt`] that is not hooked up to any event/interrupt source.
    ///
    /// So, the returned [`Interrupt`] will never be triggered.
    ///
    /// This should only be used in test code, or if the stream/iterator being created
    /// already has an [`Interrupt`].
    pub const fn empty() -> Self {
        Self::EMPTY
    }

    /// Returns an `Err` if this [`Interrupt`] has been triggered.
    ///
    /// Otherwise, returns `Ok`.
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

    /// Returns whether this [`Interrupt`] has been triggered.
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
