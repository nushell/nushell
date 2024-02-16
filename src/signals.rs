use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use nu_protocol::engine::EngineState;

pub(crate) fn ctrlc_protection(
    engine_state: &mut EngineState,
    ctrlc: &Arc<AtomicBool>,
    reinit: bool,
) {
    let engine_state_ctrlc = ctrlc.clone();

    // this should not be run again if the repl is being restarted
    if !reinit {
        let handler_ctrlc = ctrlc.clone();
        ctrlc::set_handler(move || {
            handler_ctrlc.store(true, Ordering::SeqCst);
        })
        .expect("Error setting Ctrl-C handler");
    }

    engine_state.ctrlc = Some(engine_state_ctrlc);
}
