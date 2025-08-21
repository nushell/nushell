use std::fmt::Debug;
use std::sync::{Arc, Mutex};

use crate::{ShellError, SignalAction, engine::Sequence};

/// Handler is a closure that can be sent across threads and shared.
pub type Handler = Box<dyn Fn(SignalAction) + Send + Sync>;

/// Manages a collection of handlers.
#[derive(Clone)]
pub struct Handlers {
    /// List of handler tuples containing an ID and the handler itself.
    handlers: Arc<Mutex<Vec<(usize, Handler)>>>,
    /// Sequence generator for unique IDs.
    next_id: Arc<Sequence>,
}

impl Debug for Handlers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Handlers")
            .field("next_id", &self.next_id)
            .finish()
    }
}

/// HandlerGuard that unregisters a handler when dropped.
#[derive(Clone)]
pub struct HandlerGuard {
    /// Unique ID of the handler.
    id: usize,
    /// Reference to the handlers list.
    handlers: Arc<Mutex<Vec<(usize, Handler)>>>,
}

impl Drop for HandlerGuard {
    /// Drops the `Guard`, removing the associated handler from the list.
    fn drop(&mut self) {
        if let Ok(mut handlers) = self.handlers.lock() {
            handlers.retain(|(id, _)| *id != self.id);
        }
    }
}

impl Debug for HandlerGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Guard").field("id", &self.id).finish()
    }
}

impl Handlers {
    pub fn new() -> Handlers {
        let handlers = Arc::new(Mutex::new(vec![]));
        let next_id = Arc::new(Sequence::default());
        Handlers { handlers, next_id }
    }

    /// Registers a new handler and returns an RAII guard which will unregister the handler when
    /// dropped.
    pub fn register(&self, handler: Handler) -> Result<HandlerGuard, ShellError> {
        let id = self.next_id.next()?;
        if let Ok(mut handlers) = self.handlers.lock() {
            handlers.push((id, handler));
        }

        Ok(HandlerGuard {
            id,
            handlers: Arc::clone(&self.handlers),
        })
    }

    /// Registers a new handler which persists for the entire process lifetime.
    ///
    /// Only use this for handlers which should exist for the lifetime of the program.
    /// You should prefer to use `register` with a `HandlerGuard` when possible.
    pub fn register_unguarded(&self, handler: Handler) -> Result<(), ShellError> {
        let id = self.next_id.next()?;
        if let Ok(mut handlers) = self.handlers.lock() {
            handlers.push((id, handler));
        }

        Ok(())
    }

    /// Runs all registered handlers.
    pub fn run(&self, action: SignalAction) {
        if let Ok(handlers) = self.handlers.lock() {
            for (_, handler) in handlers.iter() {
                handler(action);
            }
        }
    }
}

impl Default for Handlers {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    /// Tests registering and running multiple handlers.
    fn test_multiple_handlers() {
        let handlers = Handlers::new();
        let called1 = Arc::new(AtomicBool::new(false));
        let called2 = Arc::new(AtomicBool::new(false));

        let called1_clone = Arc::clone(&called1);
        let called2_clone = Arc::clone(&called2);

        let _guard1 = handlers.register(Box::new(move |_| {
            called1_clone.store(true, Ordering::SeqCst);
        }));
        let _guard2 = handlers.register(Box::new(move |_| {
            called2_clone.store(true, Ordering::SeqCst);
        }));

        handlers.run(SignalAction::Interrupt);

        assert!(called1.load(Ordering::SeqCst));
        assert!(called2.load(Ordering::SeqCst));
    }

    #[test]
    /// Tests the dropping of a guard and ensuring the handler is unregistered.
    fn test_guard_drop() {
        let handlers = Handlers::new();
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = Arc::clone(&called);

        let guard = handlers.register(Box::new(move |_| {
            called_clone.store(true, Ordering::Relaxed);
        }));

        // Ensure the handler is registered
        assert_eq!(handlers.handlers.lock().unwrap().len(), 1);

        drop(guard);

        // Ensure the handler is removed after dropping the guard
        assert_eq!(handlers.handlers.lock().unwrap().len(), 0);

        handlers.run(SignalAction::Interrupt);

        // Ensure the handler is not called after being dropped
        assert!(!called.load(Ordering::Relaxed));
    }
}
