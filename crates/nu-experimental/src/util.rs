use std::sync::atomic::{AtomicU8, Ordering};

/// Store an `Option<bool>` as an atomic.
///
/// # Implementation Detail
/// This stores the `Option<bool>` via its three states as `u8` representing:
/// - `None` as `0`
/// - `Some(true)` as `1`
/// - `Some(false)` as `2`
pub struct AtomicMaybe(AtomicU8);

impl AtomicMaybe {
    pub const fn new(initial: Option<bool>) -> Self {
        Self(AtomicU8::new(match initial {
            None => 0,
            Some(true) => 1,
            Some(false) => 2,
        }))
    }

    pub fn store(&self, value: impl Into<Option<bool>>, order: Ordering) {
        self.0.store(
            match value.into() {
                None => 0,
                Some(true) => 1,
                Some(false) => 2,
            },
            order,
        );
    }

    pub fn load(&self, order: Ordering) -> Option<bool> {
        match self.0.load(order) {
            0 => None,
            1 => Some(true),
            2 => Some(false),
            _ => unreachable!("inner atomic is not exposed and can only set 0 to 2"),
        }
    }

    pub fn as_ptr(&self) -> *mut u8 {
        self.0.as_ptr()
    }
}
