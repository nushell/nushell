#![allow(clippy::disallowed_types)]

use std::{ops::{Add, AddAssign, Deref, Sub, SubAssign}, time::Duration};

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant as InnerInstant;

#[cfg(target_arch = "wasm32")]
pub use web_time::Instant as InnerInstant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instant(InnerInstant);

impl Instant {
    pub fn now() -> Self {
        Instant(InnerInstant::now())
    }

    pub fn duration_since(&self, earlier: Instant) -> Duration {
        self.0.duration_since(earlier.0)
    }

    pub fn checked_duration_since(&self, earlier: Instant) -> Option<Duration> {
        self.0.checked_duration_since(earlier.0)
    }

    pub fn saturating_duration_since(&self, earlier: Instant) -> Duration {
        self.0.saturating_duration_since(earlier.0)
    }

    pub fn checked_add(&self, duration: Duration) -> Option<Instant> {
        self.0.checked_add(duration).map(Self)
    }

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
