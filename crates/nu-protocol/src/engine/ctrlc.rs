use std::fmt::Debug;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

type CtrlcHandler = Box<dyn Fn() + Send + Sync>;

#[derive(Clone)]
pub struct CtrlcHandlers {
    handlers: Arc<Mutex<Vec<(usize, CtrlcHandler)>>>,
    next_id: Arc<AtomicUsize>,
}

#[derive(Clone)]
pub struct HandlerGuard {
    id: usize,
    handlers: Arc<Mutex<Vec<(usize, CtrlcHandler)>>>,
}

impl Drop for HandlerGuard {
    fn drop(&mut self) {
        if let Ok(mut handlers) = self.handlers.lock() {
            handlers.retain(|(id, _)| *id != self.id);
        }
    }
}

impl Debug for HandlerGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandlerGuard")
            .field("id", &self.id)
            .finish()
    }
}

impl CtrlcHandlers {
    pub fn new() -> CtrlcHandlers {
        let handlers = Arc::new(Mutex::new(vec![]));
        let next_id = Arc::new(AtomicUsize::new(0));
        CtrlcHandlers { handlers, next_id }
    }

    pub fn add(&self, handler: CtrlcHandler) -> HandlerGuard {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut handlers) = self.handlers.lock() {
            handlers.push((id, handler));
        }
        HandlerGuard {
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

impl Default for CtrlcHandlers {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for CtrlcHandlers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CtrlcHandlers")
            .field("handlers", &self.handlers.lock().unwrap().len())
            .finish()
    }
}
