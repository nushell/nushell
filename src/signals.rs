use nu_protocol::{
    engine::{ctrlc::Handlers, EngineState},
    Signals,
};
use std::sync::{atomic::AtomicBool, Arc};

pub(crate) fn ctrlc_protection(engine_state: &mut EngineState, ctrlc_handlers: &Handlers) {
    let interrupt = Arc::new(AtomicBool::new(false));
    engine_state.set_signals(Signals::new(interrupt.clone()));
    {
        let ctrlc_handlers = ctrlc_handlers.clone();
        ctrlc::set_handler(move || {
            ctrlc_handlers.run();
        })
        .expect("Error setting Ctrl-C handler");
    }

    engine_state.ctrlc_handlers = Some(ctrlc_handlers.clone());
}
