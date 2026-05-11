#![allow(
    clippy::disallowed_types,
    reason = "only allow std::time::Instant here when it's not WASM"
)]
#![allow(clippy::unchecked_time_subtraction, reason = "just forwarded")]

use std::{
    ops::{Add, AddAssign, Deref, Sub, SubAssign},
    time::Duration,
};

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant as InnerInstant;

#[cfg(target_arch = "wasm32")]
pub use web_time::Instant as InnerInstant;

/// WASM-safe alternative to [`std::time::Instant`].
///
/// This is a thin wrapper around either `std::time::Instant` or `web_time::Instant` to allow
/// compiling for WASM without issues.
/// `web_time::Instant` usually re-exports `std::time::Instant` on non-WASM targets but this does
/// not allow usage of `clippy::disallowed-types` properly.
/// This wrapper fixes that.
///
/// For any reference, see [`std::time::Instant`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instant(InnerInstant);

impl Instant {
    /// Returns an instant corresponding to "now".
    ///
    /// See [`Instant::now`](InnerInstant::now).
    pub fn now() -> Self {
        Instant(InnerInstant::now())
    }

    /// Returns the amount of time elapsed from another instant to this one, or zero duration if
    /// that instant is later than this one.
    ///
    /// See [`Instant::duration_since`](InnerInstant::duration_since).
    pub fn duration_since(&self, earlier: Instant) -> Duration {
        self.0.duration_since(earlier.0)
    }

    /// Returns the amount of time elapsed from another instant to this one, or None if that instant
    /// is later than this one.
    ///
    /// See [`Instant::checked_duration_since`](InnerInstant::checked_duration_since).
    pub fn checked_duration_since(&self, earlier: Instant) -> Option<Duration> {
        self.0.checked_duration_since(earlier.0)
    }

    /// Returns the amount of time elapsed from another instant to this one, or zero duration if
    /// that instant is later than this one.
    ///
    /// See [`Instant::saturating_duration_since`](InnerInstant::saturating_duration_since).
    pub fn saturating_duration_since(&self, earlier: Instant) -> Duration {
        self.0.saturating_duration_since(earlier.0)
    }

    /// Returns `Some(t)` where `t` is the time `self + duration` if `t` can be represented as
    /// `Instant` (which means it's inside the bounds of the underlying data structure), `None`
    /// otherwise.
    ///
    /// See [`Instant::checked_add`](InnerInstant::checked_add).
    pub fn checked_add(&self, duration: Duration) -> Option<Instant> {
        self.0.checked_add(duration).map(Self)
    }

    /// Returns `Some(t)` where `t` is the time `self - duration` if `t` can be represented as
    /// `Instant` (which means it's inside the bounds of the underlying data structure), `None`
    /// otherwise.
    ///
    /// See [`Instant::checked_sub`](InnerInstant::checked_sub).
    pub fn checked_sub(&self, duration: Duration) -> Option<Instant> {
        self.0.checked_sub(duration).map(Self)
    }
}

impl Deref for Instant {
    type Target = InnerInstant;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Add<Duration> for Instant {
    type Output = Instant;

    fn add(self, rhs: Duration) -> Self::Output {
        Instant(self.0.add(rhs))
    }
}

impl AddAssign<Duration> for Instant {
    fn add_assign(&mut self, rhs: Duration) {
        self.0.add_assign(rhs)
    }
}

impl Sub<Duration> for Instant {
    type Output = Instant;

    fn sub(self, rhs: Duration) -> Self::Output {
        Instant(self.0.sub(rhs))
    }
}

impl SubAssign<Duration> for Instant {
    fn sub_assign(&mut self, rhs: Duration) {
        self.0.sub_assign(rhs)
    }
}

impl Sub<Instant> for Instant {
    type Output = Duration;

    fn sub(self, rhs: Instant) -> Self::Output {
        self.0.sub(rhs.0)
    }
}

impl From<InnerInstant> for Instant {
    fn from(value: InnerInstant) -> Self {
        Self(value)
    }
}
