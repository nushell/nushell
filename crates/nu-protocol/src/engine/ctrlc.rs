use std::fmt::Debug;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};

pub type Handler = Box<dyn Fn() + Send + Sync>;

#[derive(Clone)]
pub struct Handlers {
    handlers: Arc<Mutex<Vec<(u64, Handler)>>>,
    // we use an u64 so an overflow is impractical
    next_id: Arc<AtomicU64>,
}

#[derive(Clone)]
pub struct Guard {
    id: u64,
    handlers: Arc<Mutex<Vec<(u64, Handler)>>>,
}

impl Drop for Guard {
    fn drop(&mut self) {
        if let Ok(mut handlers) = self.handlers.lock() {
            handlers.retain(|(id, _)| *id != self.id);
        }
    }
}

impl Debug for Guard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Guard").field("id", &self.id).finish()
    }
}

impl Handlers {
    pub fn new() -> Handlers {
        let handlers = Arc::new(Mutex::new(vec![]));
        let next_id = Arc::new(AtomicU64::new(0));
        Handlers { handlers, next_id }
    }

    pub fn register(&self, handler: Handler) -> Guard {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut handlers) = self.handlers.lock() {
            handlers.push((id, handler));
        }
        Guard {
            id,
            handlers: Arc::clone(&self.handlers),
        }
    }

    pub fn run(&self) {
        if let Ok(handlers) = self.handlers.lock() {
            for (_, handler) in handlers.iter() {
                handler();
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
    use std::sync::atomic::AtomicBool;

    #[test]
    fn test_multiple_handlers() {
        let handlers = Handlers::new();
        let called1 = Arc::new(AtomicBool::new(false));
        let called2 = Arc::new(AtomicBool::new(false));

        let called1_clone = Arc::clone(&called1);
        let called2_clone = Arc::clone(&called2);

        let _guard1 = handlers.register(Box::new(move || {
            called1_clone.store(true, Ordering::SeqCst);
        }));
        let _guard2 = handlers.register(Box::new(move || {
            called2_clone.store(true, Ordering::SeqCst);
        }));

        handlers.run();

        assert!(called1.load(Ordering::SeqCst));
        assert!(called2.load(Ordering::SeqCst));
    }

    #[test]
    fn test_guard_drop() {
        let handlers = Handlers::new();
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = Arc::clone(&called);

        let guard = handlers.register(Box::new(move || {
            called_clone.store(true, Ordering::Relaxed);
        }));

        // Ensure the handler is registered
        assert_eq!(handlers.handlers.lock().unwrap().len(), 1);

        drop(guard);

        // Ensure the handler is removed after dropping the guard
        assert_eq!(handlers.handlers.lock().unwrap().len(), 0);

        handlers.run();

        // Ensure the handler is not called after being dropped
        assert!(!called.load(Ordering::Relaxed));
    }
}
