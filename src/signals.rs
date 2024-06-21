use nu_protocol::engine::{ctrlc::Handlers, EngineState};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub(crate) fn ctrlc_protection(engine_state: &mut EngineState) {

    let ctrlc = Arc::new(AtomicBool::new(false));
    let handlers = Handlers::new();

    {
        let ctrlc = ctrlc.clone();
        let handlers = handlers.clone();
        ctrlc::set_handler(move || {
            ctrlc.store(true, Ordering::SeqCst);
            handlers.run();
        })
        .expect("Error setting Ctrl-C handler");
    }

    engine_state.ctrlc = Some(ctrlc);
    engine_state.ctrlc_handlers = Some(handlers);
}
