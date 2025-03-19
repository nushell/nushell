use std::{
    error::Error,
    fmt, io,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

#[derive(Debug, Copy, Clone)]
pub struct Interrupted(());

impl fmt::Display for Interrupted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "code execution interrupted")
    }
}

impl Error for Interrupted {}

impl From<Interrupted> for io::Error {
    fn from(interrupt: Interrupted) -> Self {
        io::Error::new(io::ErrorKind::Other, interrupt)
    }
}

/// Used to check for signals to suspend or terminate the execution of code.
///
/// For now, this struct only supports interruption (ctrl+c or SIGINT).
#[derive(Debug, Clone)]
pub struct Signals {
    interrupt: Option<Arc<AtomicBool>>,
}

impl Signals {
    /// A [`Signals`] that is not hooked up to any event source.
    ///
    /// So, this [`Signals`] will never be interrupted.
    pub const EMPTY: Self = Signals { interrupt: None };

    /// Create a new [`Signals`] with `interrupt` as the interrupt source.
    ///
    /// Once `interrupt` is set to `true`, [`check`](Self::check) will error
    /// and [`interrupted`](Self::interrupted) will return `true`.
    pub fn new(interrupt: Arc<AtomicBool>) -> Self {
        Self {
            interrupt: Some(interrupt),
        }
    }

    /// Create a [`Signals`] that is not hooked up to any interrupt source.
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
    pub fn check(&self) -> Result<(), Interrupted> {
        #[inline]
        #[cold]
        fn interrupt() -> Result<(), Interrupted> {
            Err(Interrupted(()))
        }

        if self.interrupted() {
            interrupt()
        } else {
            Ok(())
        }
    }

    /// Triggers an interrupt.
    pub fn trigger_interrupt(&self) {
        if let Some(signals) = &self.interrupt {
            signals.store(true, Ordering::Relaxed);
        }
    }

    /// Returns whether an interrupt has been triggered.
    #[inline]
    pub fn interrupted(&self) -> bool {
        self.interrupt
            .as_deref()
            .is_some_and(|b| b.load(Ordering::Relaxed))
    }

    /// Returns whether a [`Signals`] has an interrupt source.
    pub fn has_interrupt(&self) -> bool {
        self.interrupt.is_some()
    }

    pub fn reset(&self) {
        if let Some(interrupt) = &self.interrupt {
            interrupt.store(false, Ordering::Relaxed);
        }
    }
}
