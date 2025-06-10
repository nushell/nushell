/// Like [`Cow`][std::borrow::Cow] but with a mutable reference instead. So not exactly
/// clone-on-write, but can be made owned.
pub enum MutableCow<'a, T> {
    Borrowed(&'a mut T),
    Owned(T),
}

impl<T: Clone> MutableCow<'_, T> {
    pub fn owned(&self) -> MutableCow<'static, T> {
        match self {
            MutableCow::Borrowed(r) => MutableCow::Owned((*r).clone()),
            MutableCow::Owned(o) => MutableCow::Owned(o.clone()),
        }
    }
}

impl<T> std::ops::Deref for MutableCow<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        match self {
            MutableCow::Borrowed(r) => r,
            MutableCow::Owned(o) => o,
        }
    }
}

impl<T> std::ops::DerefMut for MutableCow<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            MutableCow::Borrowed(r) => r,
            MutableCow::Owned(o) => o,
        }
    }
}
