use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

/// Returns true if Nu has received a SIGINT signal / ctrl+c event
pub fn was_pressed(ctrlc: &Option<Arc<AtomicBool>>) -> bool {
    if let Some(ctrlc) = ctrlc {
        ctrlc.load(Ordering::SeqCst)
    } else {
        false
    }
}
