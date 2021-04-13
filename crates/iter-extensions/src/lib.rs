use std::error::Error;

pub trait TryAllExt: Iterator {
    /// Tests if every element of the iterator matches a predicate.
    ///
    /// `try_all()` takes a closure that returns `Ok(true)`, `Ok(false)` or Err(E). It applies
    /// this closure to each element of the iterator, and if they all return
    /// `Ok(true)`, then so does `try_all()`. If any of them return `Ok(false)`, it
    /// returns `Ok(false)`. If the closure returns Err(E), `try_all` returns Err(E) immediatly
    /// (short-circuiting).
    ///
    /// `try_all()` is short-circuiting; in other words, it will stop processing
    /// as soon as it finds a `Ok(false)` or `Err(E)`
    ///
    /// An empty iterator returns `Ok(true)`.
    fn try_all<F, E>(&mut self, mut f: F) -> Result<bool, E>
    where
        Self: Sized,
        F: FnMut(Self::Item) -> Result<bool, E>,
        E: Error,
    {
        for item in self.next() {
            //if f fails, we return failure
            if !f(item)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

impl<I: Iterator> TryAllExt for I {}
