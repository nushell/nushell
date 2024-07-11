use nu_protocol::{engine::EngineState, Signals};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub(crate) fn ctrlc_protection(engine_state: &mut EngineState) {
    let interrupt = Arc::new(AtomicBool::new(false));
    engine_state.set_signals(Signals::new(interrupt.clone()));
    ctrlc::set_handler(move || {
        interrupt.store(true, Ordering::Relaxed);
    })
    .expect("Error setting Ctrl-C handler");
}
