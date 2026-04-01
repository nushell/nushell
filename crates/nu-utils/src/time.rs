#![allow(clippy::disallowed_types)]

use std::ops::Deref;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant as InnerInstant;

#[cfg(target_arch = "wasm32")]
pub use web_time::Instant as InnerInstant;

pub struct Instant(InnerInstant);

impl Instant {
    pub fn now() -> Self {
        Instant(InnerInstant::now())
    }
}

impl Deref for Instant {
    type Target = InnerInstant;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}


