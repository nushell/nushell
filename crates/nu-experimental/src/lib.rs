use crate::util::AtomicMaybe;
use std::{fmt::Debug, sync::atomic::Ordering};

mod options;
mod parse;
mod util;

pub use options::*;
pub use parse::*;

/// Where an experimental option sits in its life-cycle.
///
/// This shows how stable an experimental option is.
/// Highly unstable options should be marked as `Unstable`.
/// If the API is unlikely to change but still not quite right, use `StableOptIn`.
/// If the option seems correct, mark it as `StableOptOut` so all users get it by default.
/// That's usually the last step before full stabilization.
///
/// If we plan to remove an option, mark it as `Deprecated`.
/// It will trigger a warning when used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stability {
    /// Likely to change, disabled by default.
    Unstable,
    /// Final API, disabled by default.
    StableOptIn,
    /// Final API, enabled by default.
    StableOptOut,
    /// Deprecated, will be removed and prints a warning.
    Deprecated,
}

/// Experimental option (aka feature flag).
///
/// This struct holds one experimental option that can change some part of Nushell's behavior.
/// These options let users opt in or out of experimental changes while keeping the rest stable.
/// They're useful for testing new ideas and giving users a way to go back to older behavior if needed.
///
/// You can find all options in the statics of [`nu_experimental`](crate).
/// Everything there, except [`ALL`], is a toggleable option.
/// `ALL` gives a full list and can be used to check which options are set.
///
/// The [`Debug`] implementation shows the option's identifier, stability, and current value.
/// To also include the description in the output, use the
/// [plus sign](std::fmt::Formatter::sign_plus), e.g. `format!("{OPTION:+#?}")`.
pub struct ExperimentalOption {
    value: AtomicMaybe,
    marker: &'static (dyn DynExperimentalOptionMarker + Send + Sync),
}

impl ExperimentalOption {
    /// Construct a new `ExperimentalOption`.
    ///
    /// This should only be used to define a single static for a marker.
    pub(crate) const fn new(
        marker: &'static (dyn DynExperimentalOptionMarker + Send + Sync),
    ) -> Self {
        Self {
            value: AtomicMaybe::new(None),
            marker,
        }
    }

    pub fn identifier(&self) -> &'static str {
        self.marker.identifier()
    }

    pub fn description(&self) -> &'static str {
        self.marker.description()
    }

    pub fn stability(&self) -> Stability {
        self.marker.stability()
    }

    pub fn get(&self) -> bool {
        self.value
            .load(Ordering::Relaxed)
            .unwrap_or_else(|| match self.marker.stability() {
                Stability::Unstable => false,
                Stability::StableOptIn => false,
                Stability::StableOptOut => true,
                Stability::Deprecated => false,
            })
    }

    /// Sets the state of an experimental option.
    ///
    /// # Safety
    /// This method is unsafe to emphasize that experimental options are not designed to change
    /// dynamically at runtime.
    /// Changing their state at arbitrary points can lead to inconsistent behavior.
    /// You should set experimental options only during initialization, before the application fully
    /// starts.
    pub unsafe fn set(&self, value: bool) {
        self.value.store(value, Ordering::Relaxed);
    }

    /// Unsets an experimental option, resetting it to an uninitialized state.
    ///
    /// # Safety
    /// Like [`set`], this method is unsafe to highlight that experimental options should remain
    /// stable during runtime.
    /// Only unset options in controlled, initialization contexts to avoid unpredictable behavior.
    pub unsafe fn unset(&self) {
        self.value.store(None, Ordering::Relaxed);
    }
}

impl Debug for ExperimentalOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let add_description = f.sign_plus();
        let mut debug_struct = f.debug_struct("ExperimentalOption");
        debug_struct.field("identifier", &self.identifier());
        debug_struct.field("value", &self.get());
        debug_struct.field("stability", &self.stability());
        if add_description {
            debug_struct.field("description", &self.description());
        }
        debug_struct.finish()
    }
}

impl PartialEq for ExperimentalOption {
    fn eq(&self, other: &Self) -> bool {
        // if both underlying atomics point to the same value, we talk about the same option
        self.value.as_ptr() == other.value.as_ptr()
    }
}

impl Eq for ExperimentalOption {}

pub(crate) trait DynExperimentalOptionMarker {
    fn identifier(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn stability(&self) -> Stability;
}

impl<M: options::ExperimentalOptionMarker> DynExperimentalOptionMarker for M {
    fn identifier(&self) -> &'static str {
        M::IDENTIFIER
    }

    fn description(&self) -> &'static str {
        M::DESCRIPTION
    }

    fn stability(&self) -> Stability {
        M::STABILITY
    }
}
