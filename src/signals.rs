use nu_protocol::{
    engine::{ctrlc::Handlers, EngineState},
    Signals,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub(crate) fn ctrlc_protection(engine_state: &mut EngineState) {
    let interrupt = Arc::new(AtomicBool::new(false));
    engine_state.set_signals(Signals::new(interrupt.clone()));

    let ctrlc_handlers = Handlers::new();
    engine_state.ctrlc_handlers = Some(ctrlc_handlers.clone());

    ctrlc::set_handler(move || {
        interrupt.store(true, Ordering::Relaxed);
        ctrlc_handlers.run();
    })
    .expect("Error setting Ctrl-C handler");
}
