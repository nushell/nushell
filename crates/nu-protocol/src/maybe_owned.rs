#![allow(clippy::should_implement_trait)]

/// Helper type to allow passing something that may potentially be owned, but could also be borrowed
#[derive(Debug)]
pub enum MaybeOwned<'a, T> {
    Owned(T),
    Borrowed(&'a T),
}

impl<T> MaybeOwned<'_, T> {
    /// Allows the borrowing of an owned value or passes out the borrowed value
    pub fn borrow(&self) -> &T {
        match self {
            MaybeOwned::Owned(v) => v,
            MaybeOwned::Borrowed(v) => v,
        }
    }
}
