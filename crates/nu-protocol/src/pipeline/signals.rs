use crate::{ShellError, Span};
use nu_glob::Interruptible;
use nu_system::SuspendState;
use serde::{Deserialize, Serialize};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// Inner state shared across [`Signals`] clones.
///
/// Stored in an `Arc` so that `Signals` stays exactly 8 bytes (a nullable pointer),
/// preserving the `size_of::<Value>() <= 56` invariant in `value/mod.rs`.
#[derive(Debug)]
struct SignalsInner {
    interrupt: Arc<AtomicBool>,
    suspend: Option<Arc<SuspendState>>,
}

/// Used to check for signals to suspend or terminate the execution of Nushell code.
///
/// Supports both interruption (ctrl+c or SIGINT) and cooperative suspension for internal pipelines.
///
/// `size_of::<Signals>() == 8` — fits in `Option<Signals>` with niche optimization.
#[derive(Debug, Clone)]
pub struct Signals {
    inner: Option<Arc<SignalsInner>>,
}

impl Signals {
    /// A [`Signals`] that is not hooked up to any event/signals source.
    ///
    /// So, this [`Signals`] will never be interrupted.
    pub const EMPTY: Self = Signals { inner: None };

    /// Create a new [`Signals`] with `ctrlc` as the interrupt source.
    ///
    /// Once `ctrlc` is set to `true`, [`check`](Self::check) will error
    /// and [`interrupted`](Self::interrupted) will return `true`.
    pub fn new(ctrlc: Arc<AtomicBool>) -> Self {
        Self {
            inner: Some(Arc::new(SignalsInner {
                interrupt: ctrlc,
                suspend: None,
            })),
        }
    }

    /// Create a [`Signals`] with both interrupt and cooperative suspend support.
    pub fn with_suspend(ctrlc: Arc<AtomicBool>, suspend: Arc<SuspendState>) -> Self {
        Self {
            inner: Some(Arc::new(SignalsInner {
                interrupt: ctrlc,
                suspend: Some(suspend),
            })),
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
    /// Also cooperatively parks if suspended before checking for interrupts.
    ///
    /// Otherwise, returns `Ok`.
    #[inline]
    pub fn check(&self, span: &Span) -> Result<(), ShellError> {
        #[inline]
        #[cold]
        fn interrupt_error(span: &Span) -> Result<(), ShellError> {
            Err(ShellError::Interrupted { span: *span })
        }

        self.wait_if_suspended();
        if self.interrupted() {
            interrupt_error(span)
        } else {
            Ok(())
        }
    }

    /// Triggers an interrupt.
    pub fn trigger(&self) {
        if let Some(inner) = &self.inner {
            inner.interrupt.store(true, Ordering::Relaxed);
        }
    }

    /// Returns whether an interrupt has been triggered.
    #[inline]
    pub fn interrupted(&self) -> bool {
        self.inner
            .as_ref()
            .is_some_and(|i| i.interrupt.load(Ordering::Relaxed))
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.inner.is_none()
    }

    pub fn reset(&self) {
        if let Some(inner) = &self.inner {
            inner.interrupt.store(false, Ordering::Relaxed);
        }
    }

    /// Signal the pipeline to suspend cooperatively.
    pub fn suspend(&self) {
        if let Some(inner) = &self.inner
            && let Some(s) = &inner.suspend
        {
            s.suspend();
        }
    }

    /// Resume a cooperatively suspended pipeline.
    pub fn resume(&self) {
        if let Some(inner) = &self.inner
            && let Some(s) = &inner.suspend
        {
            s.resume();
        }
    }

    /// Returns whether the pipeline is currently suspended.
    pub fn is_suspended(&self) -> bool {
        self.inner
            .as_ref()
            .is_some_and(|i| i.suspend.as_ref().is_some_and(|s| s.is_suspended()))
    }

    /// Cooperative yield point. Blocks if suspended; no-op if suspend is `None`.
    #[inline]
    pub fn wait_if_suspended(&self) {
        if let Some(inner) = &self.inner
            && let Some(s) = &inner.suspend
        {
            s.wait_if_suspended();
        }
    }

    pub fn suspend_state(&self) -> Option<&Arc<SuspendState>> {
        self.inner.as_ref().and_then(|i| i.suspend.as_ref())
    }

    /// Returns the shared interrupt `Arc<AtomicBool>`, if any.
    ///
    /// Used by `CommandThread` to share the interrupt flag between the main thread
    /// and the pipeline worker thread.
    pub fn interrupt_arc(&self) -> Option<Arc<AtomicBool>> {
        self.inner.as_ref().map(|i| i.interrupt.clone())
    }
}

impl Interruptible for Signals {
    #[inline]
    fn interrupted(&self) -> bool {
        self.interrupted()
    }
}

/// The types of things that can be signaled. It's anticipated this will change as we learn more
/// about how we'd like signals to be handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalAction {
    Interrupt,
    Reset,
}
