use nu_protocol::{Handlers, SignalAction, Signals, engine::EngineState};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub(crate) fn ctrlc_protection(engine_state: &mut EngineState) {
    let interrupt = Arc::new(AtomicBool::new(false));
    engine_state.set_signals(Signals::new(interrupt.clone()));

    let signal_handlers = Handlers::new();

    // Register a handler to kill all background jobs on interrupt.
    signal_handlers
        .register_unguarded({
            let jobs = engine_state.jobs.clone();
            Box::new(move |action| {
                if action == SignalAction::Interrupt
                    && let Ok(mut jobs) = jobs.lock()
                {
                    let _ = jobs.kill_all();
                }
            })
        })
        .expect("Failed to register interrupt signal handler");

    engine_state.signal_handlers = Some(signal_handlers.clone());

    ctrlc::set_handler(move || {
        interrupt.store(true, Ordering::Relaxed);
        signal_handlers.run(SignalAction::Interrupt);
    })
    .expect("Error setting Ctrl-C handler");
}
