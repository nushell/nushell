use std::fmt::Debug;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

type Handler = Box<dyn Fn() + Send + Sync>;

#[derive(Clone)]
pub struct Handlers {
    handlers: Arc<Mutex<Vec<(usize, Handler)>>>,
    next_id: Arc<AtomicUsize>,
}

#[derive(Clone)]
pub struct Guard {
    id: usize,
    handlers: Arc<Mutex<Vec<(usize, Handler)>>>,
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
        let next_id = Arc::new(AtomicUsize::new(0));
        Handlers { handlers, next_id }
    }

    pub fn add(&self, handler: Handler) -> Guard {
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

impl Debug for Handlers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Handlers")
            .field("handlers", &self.handlers.lock().unwrap().len())
            .finish()
    }
}
