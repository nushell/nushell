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
        eprintln!("Dropping guard: {:?}", self);
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
        eprintln!("Adding guard: {:?}", id);
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
