use std::sync::{Arc, Mutex};

type CtrlcHandler = Box<dyn Fn() + Send + Sync>;

#[derive(Clone)]
pub struct CtrlcHandlers {
    handlers: Arc<Mutex<Vec<CtrlcHandler>>>,
}

impl CtrlcHandlers {
    pub fn new() -> CtrlcHandlers {
        CtrlcHandlers {
            handlers: Arc::new(Mutex::new(vec![])),
        }
    }

    pub fn add(&self, handler: CtrlcHandler) {
        if let Some(mut handlers) = self.handlers.lock().ok() {
            handlers.push(handler);
        }
    }

    pub fn run(&self) {
        if let Some(handlers) = self.handlers.lock().ok() {
            for handler in handlers.iter() {
                handler();
            }
        }
    }
}

impl std::fmt::Debug for CtrlcHandlers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CtrlcHandlers")
            .field("handlers", &self.handlers.lock().unwrap().len())
            .finish()
    }
}
