use crate::{ShellError, Span};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

/// Used to check for signals to suspend or terminate the execution of Nushell code.
///
/// For now, this struct only supports interruption (ctrl+c or SIGINT).
#[derive(Debug, Clone)]
pub struct Signals {
    signals: Option<Arc<AtomicBool>>,
}

impl Signals {
    /// A [`Signals`] that is not hooked up to any event/signals source.
    ///
    /// So, this [`Signals`] will never be interrupted.
    pub const EMPTY: Self = Signals { signals: None };

    /// Create a new [`Signals`] with `ctrlc` as the interrupt source.
    ///
    /// Once `ctrlc` is set to `true`, [`check`](Self::check) will error
    /// and [`interrupted`](Self::interrupted) will return `true`.
    pub fn new(ctrlc: Arc<AtomicBool>) -> Self {
        Self {
            signals: Some(ctrlc),
        }
    }

    /// Create a [`Signals`] that is not hooked up to any event/signals source.
    ///
    /// So, the returned [`Signals`] will never be interrupted.
    ///
    /// This should only be used in test code, or if the stream/iterator being created
    /// already has an underlying [`Signals`].
    pub const fn empty() -> Self {
        Self::EMPTY
    }

    /// Returns an `Err` if an interrupt has been triggered.
    ///
    /// Otherwise, returns `Ok`.
    #[inline]
    pub fn check(&self, span: Span) -> Result<(), ShellError> {
        #[inline]
        #[cold]
        fn interrupt_error(span: Span) -> Result<(), ShellError> {
            Err(ShellError::Interrupted { span })
        }

        if self.interrupted() {
            interrupt_error(span)
        } else {
            Ok(())
        }
    }

    /// Returns whether an interrupt has been triggered.
    #[inline]
    pub fn interrupted(&self) -> bool {
        self.signals
            .as_deref()
            .is_some_and(|b| b.load(Ordering::Relaxed))
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.signals.is_none()
    }

    pub(crate) fn reset(&self) {
        if let Some(signals) = &self.signals {
            signals.store(false, Ordering::Relaxed);
        }
    }
}
