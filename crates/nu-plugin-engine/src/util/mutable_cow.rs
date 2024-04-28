/// Like [`Cow`] but with a mutable reference instead. So not exactly clone-on-write, but can be
/// made owned.
pub enum MutableCow<'a, T> {
    Borrowed(&'a mut T),
    Owned(T),
}

impl<'a, T: Clone> MutableCow<'a, T> {
    pub fn owned(&self) -> MutableCow<'static, T> {
        match self {
            MutableCow::Borrowed(r) => MutableCow::Owned((*r).clone()),
            MutableCow::Owned(o) => MutableCow::Owned(o.clone()),
        }
    }
}

impl<'a, T> std::ops::Deref for MutableCow<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        match self {
            MutableCow::Borrowed(r) => r,
            MutableCow::Owned(o) => o,
        }
    }
}

impl<'a, T> std::ops::DerefMut for MutableCow<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            MutableCow::Borrowed(r) => r,
            MutableCow::Owned(o) => o,
        }
    }
}
