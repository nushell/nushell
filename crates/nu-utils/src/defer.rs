/// Runs a closure when the value is dropped.
///
/// Create this with [`defer`]. The closure runs when the returned guard leaves
/// scope or is explicitly dropped.
#[must_use = "store this value otherwise the drop function gets called immediately"]
pub struct Deferred<F: Fn()>(F);

impl<F: Fn()> Drop for Deferred<F> {
    fn drop(&mut self) {
        self.0();
    }
}

/// Defers running a closure until the returned guard is dropped.
///
/// This is useful for small cleanup actions that should happen when a scope is
/// exited.
///
/// # Example
///
/// ```rust
/// use std::cell::Cell;
/// use nu_utils::defer;
///
/// let was_called = Cell::new(false);
/// {
///     let _deferred = defer(|| was_called.set(true));
///     assert!(!was_called.get());
/// }
///
/// assert!(was_called.get());
/// ```
pub fn defer<F: Fn()>(on_drop: F) -> Deferred<F> {
    Deferred(on_drop)
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::defer;

    #[test]
    fn runs_closure_when_scope_ends() {
        let was_called = Cell::new(false);

        {
            let _deferred = defer(|| was_called.set(true));
            assert!(!was_called.get());
        }

        assert!(was_called.get());
    }

    #[test]
    fn runs_closure_when_explicitly_dropped() {
        let was_called = Cell::new(false);
        let deferred = defer(|| was_called.set(true));

        assert!(!was_called.get());
        drop(deferred);

        assert!(was_called.get());
    }

    #[test]
    fn runs_closure_once() {
        let calls = Cell::new(0);

        {
            let _deferred = defer(|| calls.set(calls.get() + 1));
        }

        assert_eq!(calls.get(), 1);
    }
}
