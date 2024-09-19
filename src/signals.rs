use nu_protocol::{engine::EngineState, Handlers, SignalAction, Signals};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub(crate) fn ctrlc_protection(engine_state: &mut EngineState) {
    let interrupt = Arc::new(AtomicBool::new(false));
    engine_state.set_signals(Signals::new(interrupt.clone()));

    let signal_handlers = Handlers::new();
    engine_state.signal_handlers = Some(signal_handlers.clone());

    ctrlc::set_handler(move || {
        interrupt.store(true, Ordering::Relaxed);
        signal_handlers.run(SignalAction::Interrupt);
    })
    .expect("Error setting Ctrl-C handler");
}
